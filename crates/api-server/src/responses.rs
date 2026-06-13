use salvo::oapi::ToSchema;
use serde::Serialize;

// #[derive(Debug)]
// pub enum ApiResponseCause {
//     NOTFOUND,
//     EMPTY,
//     DELETED,
// }

/// Generic API envelope for successful or error responses.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    // #[serde(skip_serializing)]
    // pub cause: Option<ApiResponseCause>,
}

/// Pagination metadata
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMetadata {
    pub total_records: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_previous: bool,
}

impl PaginationMetadata {
    pub fn new(total_records: i64, page: i64, page_size: i64) -> Self {
        let total_pages = (total_records as f64 / page_size as f64).ceil() as i64;
        let has_next = page < total_pages;
        let has_previous = page > 1;

        PaginationMetadata {
            total_records,
            page,
            page_size,
            total_pages,
            has_next,
            has_previous,
        }
    }
}

/// Paginated API response with structured pagination
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedApiResponse<T>
where
    T: Serialize,
{
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Success with a payload
    pub fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            // cause: None,
        }
    }

    /// Success with no payload
    // #[allow(dead_code)]
    // pub fn ok_empty(cause: ApiResponseCause) -> Self {
    //     ApiResponse {
    //         success: true,
    //         data: None,
    //         error: None,
    //         cause: Some(cause),
    //     }
    // }

    pub fn err(msg: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg.into()),
            // cause: None,
        }
    }
}

impl<T> PaginatedApiResponse<T>
where
    T: Serialize,
{
    /// Success with paginated data
    ///
    /// # Arguments
    /// * `data` - Vector of items to return
    /// * `total_records` - Total number of records in the database
    /// * `page` - Current page number (1-based)
    /// * `page_size` - Number of records per page
    pub fn ok(data: Vec<T>, total_records: i64, page: i64, page_size: i64) -> Self {
        let pagination = PaginationMetadata::new(total_records, page, page_size);

        PaginatedApiResponse {
            ok: true,
            data: Some(data),
            pagination: Some(pagination),
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn err(msg: impl Into<String>) -> Self {
        PaginatedApiResponse {
            ok: false,
            data: None,
            pagination: None,
            error: Some(msg.into()),
        }
    }
}
