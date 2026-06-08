use std::sync::Arc;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Debug)]
pub struct Actors {}

#[derive(Debug)]
pub struct ControllerState {
    pub tracing_worker_guards: Vec<WorkerGuard>,
    pub spawned_actors: Actors,
}

impl ControllerState {
    pub fn new() -> Self {
        Self {
            tracing_worker_guards: vec![],
            spawned_actors: Actors {},
        }
    }
}
