use agent_database::query::{DeleteQuery, DynamicQuery, FilterCondition, FilterOperator};
use agent_database::{
    get_db_connection_pool, ApiEncryptionKey, RepositoryContainer, RepositoryDynamicQuery,
};
use chrono::Timelike;
use chrono::{Duration, NaiveDateTime, Utc};
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
}

async fn add_async_job<F, Fut>(
    sched: &JobScheduler,
    cron_expr: &'static str,
    job_name: &'static str,
    deps: Arc<ScheduledJobDependencies>,
    job_fn: F,
) -> Result<(), JobSchedulerError>
where
    F: Fn(Arc<ScheduledJobDependencies>, String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AppResult<()>> + Send + 'static,
{
    let job_fn = Arc::new(job_fn);

    let job = Job::new_async(cron_expr, move |_job_id, _lock| {
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

async fn expire_cache_records(
    deps: Arc<ScheduledJobDependencies>,
    job_name: String,
) -> AppResult<()> {
    let pool = deps.db_pool.clone();

    tokio::task::spawn_blocking(move || -> AppResult<()> {
        // Build filter for checking if cache records have expired
        let now = Utc::now().naive_utc();
        let now = now.with_nanosecond(0).unwrap();
        let filters = vec![FilterCondition {
            field: "expires_at".to_string(),
            operator: FilterOperator::Lt,
            value: now.to_string(),
        }];

        // Get a database connection from the pool
        let mut conn = pool
            .get()
            .map_err(|err| ApiError::BadRequest(err.to_string()))?;

        // Get Access to the Cache DB reporitory
        let cache_repo = deps.repos.cache_repo.clone();

        // Attempt to delete expired records
        let deleted = cache_repo.delete_by_dynamic_query(&mut conn, &DeleteQuery { filters })?;

        debug!(
            "{} executed and removed {} expired cache records",
            job_name, deleted
        );

        println!(
            "{} executed and removed {} expired cache records",
            job_name, deleted
        );

        Ok(())
    })
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()));

    Ok(())
}

// async fn expire_cache_records2(deps: Arc<ScheduledJobDependencies>) -> AppResult<()> {
//     let pool = deps.db_pool.clone();
//     let repos = deps.repos.clone();

//     tokio::task::spawn_blocking(move || -> AppResult<()> {
//         let mut conn = pool
//             .get()
//             .map_err(|err| ApiError::BadRequest(err.to_string()));

//         println!("We run less often");
//         Ok(())
//     })
//     .await
//     .map_err(|err| ApiError::BadRequest(err.to_string()));

//     Ok(())
// }

pub async fn setup_scheduler(dependencies: ScheduledJobDependencies) -> AppResult<JobScheduler> {
    // Create our background job scheduler and our job dependencies so we can interact with the  database
    let sched = JobScheduler::new().await?;
    let deps = Arc::new(dependencies);

    // Create our list of async jobs
    add_async_job(
        &sched,
        "0/10 * * * * *",
        "Expire Cache Records",
        deps.clone(),
        expire_cache_records,
    )
    .await?;

    // add_async_job(
    //     &sched,
    //     "0/20 * * * * *",
    //     "Expire Cache Records 2",
    //     deps.clone(),
    //     expire_cache_records2,
    // )
    // .await?;

    Ok(sched)
}
