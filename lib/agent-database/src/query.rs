use salvo::extract::{Extractible, Metadata};
use salvo::prelude::*;
use std::collections::HashMap;

/// Dynamic filter operators for building flexible queries
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperator {
    Like,
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    NotIn,
}

impl FilterOperator {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "like" => Some(FilterOperator::Like),
            "eq" => Some(FilterOperator::Eq),
            "neq" => Some(FilterOperator::NotEq),
            "gt" => Some(FilterOperator::Gt),
            "gte" => Some(FilterOperator::Gte),
            "lt" => Some(FilterOperator::Lt),
            "lte" => Some(FilterOperator::Lte),
            "in" => Some(FilterOperator::In),
            "nin" => Some(FilterOperator::NotIn),
            _ => None,
        }
    }
}

/// A single filter condition in a dynamic query
#[derive(Debug, Clone)]
pub struct FilterCondition {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

/// Sort direction for query results
#[derive(Debug, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// A sort condition for ordering results
#[derive(Debug, Clone)]
pub struct SortCondition {
    pub field: String,
    pub direction: SortDirection,
}

/// Dynamic query with filters, sorting, and pagination
#[derive(Debug)]
pub struct DynamicQuery {
    pub filters: Vec<FilterCondition>,
    pub sort: Vec<SortCondition>,
    pub page: i64,
    pub page_size: i64,
}

/// Dynamic query with filters
#[derive(Debug)]
pub struct DeleteQuery {
    pub filters: Vec<FilterCondition>,
}

impl DynamicQuery {
    /// Parse raw query parameters into a DynamicQuery
    pub fn from_params(params: HashMap<String, String>) -> Result<Self, String> {
        let mut filters = Vec::new();
        let mut sort = Vec::new();

        // Extract pagination params (case-insensitive)
        let page = params
            .iter()
            .find(|(k, _)| k.to_lowercase() == "page")
            .and_then(|(_, v)| v.parse::<i64>().ok())
            .unwrap_or(1);

        let page_size = params
            .iter()
            .find(|(k, _)| k.to_lowercase() == "page_size")
            .and_then(|(_, v)| v.parse::<i64>().ok())
            .unwrap_or(20);

        // Validate pagination
        if page < 1 {
            return Err("Page must be >= 1".to_string());
        }
        if page_size < 1 || page_size > 100 {
            return Err("Page size must be between 1 and 100".to_string());
        }

        // Process all params
        for (key, value) in &params {
            let key_lower = key.to_lowercase();

            // Skip pagination params (case-insensitive)
            if key_lower == "page" || key_lower == "page_size" {
                continue;
            }

            // Handle orderby (case-insensitive)
            if key_lower.starts_with("orderby") {
                let parts: Vec<&str> = key.split('.').collect();

                if parts.len() == 2 {
                    let field = value.to_string();
                    let direction = match parts[1].to_lowercase().as_str() {
                        "desc" => SortDirection::Desc,
                        _ => SortDirection::Asc,
                    };
                    sort.push(SortCondition { field, direction });
                } else if parts.len() == 1 {
                    sort.push(SortCondition {
                        field: value.to_string(),
                        direction: SortDirection::Asc,
                    });
                }
                continue;
            }

            // Handle filters: field.operator=value
            let parts: Vec<&str> = key.split('.').collect();

            // If there is no operator i.e. name.[eq] or name.[like] - then assume eq
            if parts.len() == 1 {
                filters.push(FilterCondition {
                    field: parts[0].to_string(),
                    operator: FilterOperator::Eq,
                    value: value.clone(),
                })
            }

            if parts.len() == 2 {
                let field = parts[0].to_string();
                if let Some(operator) = FilterOperator::from_str(parts[1]) {
                    filters.push(FilterCondition {
                        field,
                        operator,
                        value: value.clone(),
                    });
                }
            }
        }

        Ok(DynamicQuery {
            filters,
            sort,
            page,
            page_size,
        })
    }

    /// Get the offset for pagination
    pub fn get_offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }

    /// Validate that only allowed fields are being used
    pub fn validate_fields(&self, allowed_fields: &[&str]) -> Result<(), String> {
        for filter in &self.filters {
            if !allowed_fields.contains(&filter.field.as_str()) {
                return Err(format!(
                    "Field '{}' not allowed for filtering",
                    filter.field
                ));
            }
        }

        for sort in &self.sort {
            if !allowed_fields.contains(&sort.field.as_str()) {
                return Err(format!("Field '{}' not allowed for sorting", sort.field));
            }
        }

        Ok(())
    }
}

impl DeleteQuery {
    /// Parse raw query parameters into a DynamicQuery
    pub fn from_params(mut params: HashMap<String, String>) -> Result<Self, String> {
        let mut filters = Vec::new();

        // Remove confirm_delete_all key before iterating
        params.remove("confirm_delete_all");

        // Process all params
        for (key, value) in &params {
            // Handle filters: field.operator=value
            let parts: Vec<&str> = key.split('.').collect();

            // If there is no operator i.e. name.[eq] or name.[like] - then assume eq
            if parts.len() == 1 {
                filters.push(FilterCondition {
                    field: parts[0].to_string(),
                    operator: FilterOperator::Eq,
                    value: value.clone(),
                })
            }

            if parts.len() == 2 {
                let field = parts[0].to_string();
                if let Some(operator) = FilterOperator::from_str(parts[1]) {
                    filters.push(FilterCondition {
                        field,
                        operator,
                        value: value.clone(),
                    });
                }
            }
        }

        Ok(DeleteQuery { filters })
    }

    /// Validate that only allowed fields are being used
    pub fn validate_fields(&self, allowed_fields: &[&str]) -> Result<(), String> {
        for filter in &self.filters {
            if !allowed_fields.contains(&filter.field.as_str()) {
                return Err(format!(
                    "Field '{}' not allowed for filtering",
                    filter.field
                ));
            }
        }

        Ok(())
    }
}

impl<'de> Extractible<'de> for DynamicQuery {
    fn metadata() -> &'static Metadata {
        static METADATA: Metadata = Metadata::new("metadata");
        &METADATA
    }

    fn extract(
        req: &'de mut Request,
        _depot: &'de mut Depot,
    ) -> impl Future<Output = Result<Self, impl Writer + Send + std::fmt::Debug + 'static>> + Send
    where
        Self: Sized,
    {
        async {
            // Extract query parameters into a HashMap
            let params: HashMap<String, String> = req
                .queries()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            // Call your existing from_params function
            Self::from_params(params).map_err(|err_msg| StatusError::bad_request().detail(err_msg))
        }
    }
}

impl<'de> Extractible<'de> for DeleteQuery {
    fn metadata() -> &'static Metadata {
        static METADATA: Metadata = Metadata::new("metadata");
        &METADATA
    }

    fn extract(
        req: &'de mut Request,
        _depot: &'de mut Depot,
    ) -> impl Future<Output = Result<Self, impl Writer + Send + std::fmt::Debug + 'static>> + Send
    where
        Self: Sized,
    {
        async {
            let params: HashMap<String, String> = req
                .queries()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            Self::from_params(params).map_err(|err_msg| StatusError::bad_request().detail(err_msg))
        }
    }
}
