use crate::actors::{
    config::messages::ConfigManagerMessage, connection_manager::messages::ConnectionManagerMessage,
};
use ractor::ActorRef;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Debug)]
pub struct Actors {
    pub connection_manager: Option<ActorRef<ConnectionManagerMessage>>,
    pub config_manager: Option<ActorRef<ConfigManagerMessage>>,
}

#[derive(Debug)]
pub struct ControllerState {
    pub tracing_worker_guards: Vec<WorkerGuard>,
    pub spawned_actors: Actors,
}

impl ControllerState {
    pub fn new() -> Self {
        Self {
            tracing_worker_guards: vec![],
            spawned_actors: Actors {
                connection_manager: None,
                config_manager: None,
            },
        }
    }
}
