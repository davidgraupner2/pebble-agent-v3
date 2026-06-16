use crate::config::Config;
use crate::properties::{
    DEFAULT_PROPERTY_API_PURGE_EXPIRED_CACHE_RECORDS_SCHEDULE,
    PROPERTY_API_PURGE_EXPIRED_CACHE_RECORDS_SCHEDULE,
};
use agent_core::prelude::RuntimeConstants;
use agent_database::query::{DeleteQuery, FilterCondition, FilterOperator};
use agent_database::{RepositoryContainer, RepositoryDynamicQuery};
use chrono::Timelike;
use chrono::Utc;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::future::Future;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::debug;

use crate::error::{ApiError, AppResult};

#[derive(Clone)]
pub struct ScheduledJobDependencies {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub repos: RepositoryContainer,
    pub config: Config,
}

async fn add_async_job<F, Fut>(
    sched: &JobScheduler,
    // cron_expr: &'static str,
    cron_expr: String,
    job_name: &'static str,
    deps: Arc<ScheduledJobDependencies>,
    job_fn: F,
) -> Result<(), JobSchedulerError>
where
    F: Fn(Arc<ScheduledJobDependencies>, String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AppResult<()>> + Send + 'static,
{
    let job_fn = Arc::new(job_fn);

    let job = Job::new_async(cron_expr.clone(), move |_job_id, _lock| {
        let deps = Arc::clone(&deps);
        let job_fn = Arc::clone(&job_fn);

        Box::pin(async move {
            if let Err(err) = (job_fn)(deps, job_name.to_string()).await {
                tracing::error!(job = job_name, error = %err, "scheduled job failed");
            }
        })
    })?;

    sched.add(job).await?;
    tracing::info!(job = job_name, cron = cron_expr, "scheduled job added");
    Ok(())
}

async fn purge_expired_cache_records(
    deps: Arc<ScheduledJobDependencies>,
    job_name: String,
) -> AppResult<()> {
    let pool = deps.db_pool.clone();

    tokio::task::spawn_blocking(move || -> AppResult<()> {
        // Build filter for checking if cache records have expired
        let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();

        // let filters = vec![FilterCondition {
        //     field: "expires_at".to_string(),
        //     operator: FilterOperator::Lt,
        //     value: now.to_string(),
        // }];

        // // Get a database connection from the pool
        // let mut conn = pool.get().map_err(|err| {
        //     ApiError::JobSchedulerError(format!("Error getting DB Connection - {}", err))
        // })?;

        // // Get Access to the Cache DB reporitory
        // let cache_repo = deps.repos.cache_repo.clone();

        // // Attempt to delete expired records
        // let deleted = cache_repo.delete_by_dynamic_query(&mut conn, &DeleteQuery { filters })?;

        // debug!(
        //     "{} executed and removed {} expired cache records",
        //     job_name, deleted
        // );

        Ok(())
    })
    .await
    .map_err(|err| {
        ApiError::JobSchedulerError(format!(
            "Error joining 'purge_expired_cache_records' task: {}",
            err
        ))
    })??;

    Ok(())
}

pub async fn setup_scheduler(dependencies: ScheduledJobDependencies) -> AppResult<JobScheduler> {
    // Get our schedules from the properties
    let runtime_constants = RuntimeConstants::global();
    let purge_expired_cache_records_schedule = dependencies.config.get_string(
        PROPERTY_API_PURGE_EXPIRED_CACHE_RECORDS_SCHEDULE,
        DEFAULT_PROPERTY_API_PURGE_EXPIRED_CACHE_RECORDS_SCHEDULE,
        runtime_constants.api_id().to_string(),
    );

    // Create our background job scheduler and our job dependencies so we can interact with the  database
    let sched = JobScheduler::new().await?;
    let deps = Arc::new(dependencies);

    // Create our list of async jobs
    // add_async_job(
    //     &sched,
    //     "0/10 * * * * *",
    //     "Expire Cache Records",
    //     deps.clone(),
    //     purge_expired_cache_records,
    // )
    // .await?;

    add_async_job(
        &sched,
        purge_expired_cache_records_schedule,
        "Expire Cache Records",
        deps.clone(),
        purge_expired_cache_records,
    )
    .await?;

    Ok(sched)
}
