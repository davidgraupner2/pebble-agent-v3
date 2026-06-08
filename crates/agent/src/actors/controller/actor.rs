use crate::actors::controller::arguments::ControllerArguments;
use crate::actors::controller::messages::AgentControllerMessage;
use crate::actors::controller::state::ControllerState;
use agent_core::prelude::*;
use agent_logging::initialise_logging;
use ractor::Actor;
use ractor::ActorProcessingErr;
use ractor::ActorRef;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug)]
pub struct Controller;

impl Actor for Controller {
    type State = ControllerState;
    type Msg = AgentControllerMessage;
    type Arguments = ControllerArguments;

    // Invoked when the controller is being started
    // Panics in pre_start do not invoke the supervision strategy and the actor won’t be started
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let runtime_constants = RuntimeConstants::global();

        let logging_guards = initialise_logging(
            runtime_constants.folders().logs(),
            runtime_constants.exe_name(),
            &arguments.log_format,
            &arguments.log_output,
            Some(&arguments.logging_level),
        );

        let mut state = ControllerState::new();
        state.tracing_worker_guards = logging_guards;

        println!("Starting Agent: {:#?}", arguments);
        info!("This is info logging");
        debug!("This is debug logging");
        error!("This is error logging");
        trace!("This is trace logging");
        warn!("This is warn logging");

        Ok(state)
    }

    #[instrument(name = "Agent Controller Post Start", level = "trace")]
    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("Agent Controller has started");

        println!("Agent has started");

        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // We received a message to execute a function
            // Lets pass that to the worker factory to execute on the worker pool
            AgentControllerMessage::ExecuteFunction(payload) => {}
            AgentControllerMessage::Shutdown => {
                myself.stop(None);
            }
        }

        // println!("Received Message: {:#?}", message);
        Ok(())
    }

    #[instrument(name = "Controller_Supervision_Handler", level = "trace")]
    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: ractor::SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }
}
