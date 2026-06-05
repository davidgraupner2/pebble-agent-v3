use crate::DatabaseError;
use crate::errors::Result;
use crate::models::{NewTag, Tags};
use crate::query::{FilterCondition, FilterOperator};
use crate::schema::tags;
use diesel::sql_types::Bool;
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel::{debug_query, prelude::*};
use tracing::{debug, info};

#[cfg(debug_assertions)]
use tracing::trace;

type BoxedTagCondition = Box<dyn BoxableExpression<tags::table, Sqlite, SqlType = Bool>>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TagsRepository;

/// Build a tag filter condition
fn build_tag_filter_condition(operator: &FilterOperator, value: &str) -> Result<BoxedTagCondition> {
    let value_owned = value.to_string();
    let field: String = "tags.name".to_string();
    match operator {
        FilterOperator::Eq => Ok(Box::new(tags::name.eq(value_owned.clone()))),
        FilterOperator::Like => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(tags::name.like(pattern)))
        }
        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

pub(crate) fn get_tag_ids(conn: &mut SqliteConnection, tags: &[String]) -> Result<Vec<i32>> {
    debug!(count = tags.len(), "Getting tag IDs");
    let tag_ids = tags::table
        .filter(tags::name.eq_any(tags))
        .select(tags::id)
        .load::<i32>(conn)?;
    info!(found = tag_ids.len(), "Tag IDs retrieved");
    Ok(tag_ids)
}

pub(crate) fn get_tag_names_for_filter_conditions(
    conn: &mut SqliteConnection,
    filters: Vec<&FilterCondition>,
) -> Result<Vec<String>> {
    debug!(
        filter_count = filters.len(),
        "Getting tag names for filter conditions"
    );
    let mut tag_query = tags::table.select(Tags::as_select()).into_boxed();
    if filters.is_empty() {
        return Ok(vec![]);
    } else {
        for tag_filter in filters.clone() {
            // Tags can be provided as a comma seperated string
            let values: Vec<&str> = tag_filter.value.split(",").collect();
            for value in values.clone().into_iter() {
                let condition = build_tag_filter_condition(&tag_filter.operator, &value)?;
                tag_query = tag_query.or_filter(condition);
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&tag_query),"Retrieving tag records from database");

        let tag_records = tag_query.load::<Tags>(conn)?;
        info!(found = tag_records.len(), "Tag names retrieved");
        return Ok(tag_records
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<String>>());
    }
}

pub(crate) fn insert_or_get_tag(conn: &mut SqliteConnection, name: &str) -> Result<Tags> {
    debug!(name = %name, "Inserting or getting tag");
    let result = diesel::insert_into(tags::table)
        .values(NewTag {
            name: name.to_string(),
        })
        .on_conflict(tags::name)
        .do_update()
        .set(tags::name.eq(name))
        .returning(Tags::as_returning())
        .get_result(conn)?;
    info!(name = %name, "Tag inserted or retrieved");
    Ok(result)
}
