use crate::actors::config::actor::ConfigManager;
use crate::actors::config::arguments::ConfigManagerArguments;
use crate::actors::config::messages::ConfigManagerMessage;
use crate::actors::connection_manager::actor::ConnectionManagerActor;
use crate::actors::connection_manager::arguments::ConnectionManagerStartupArguments;
use crate::actors::connection_manager::messages::ConnectionManagerMessage;
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
        myself: ActorRef<Self::Msg>,
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

        // Start the Connection Manager as a linked actor i.e. Controller is the supervisor
        let connection_manager_startup_args = ConnectionManagerStartupArguments {
            controller: myself.clone(),
            connection_string: arguments.connection_string.clone(),
            connection_timeout_seconds: arguments.connection_timeout,
            ping_interval_seconds: arguments.ping_interval,
            retry_interval_seconds: arguments.retry_interval,
            pong_response_interval: arguments.pong_response_interval,
            proxy: arguments.proxy_settings,
        };

        state.spawned_actors.connection_manager =
            start_agent_connection_manager(myself.clone(), connection_manager_startup_args).await;

        println!("Agent has started");

        // println!("Starting Agent: {:#?}", arguments);
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

        state.spawned_actors.config_manager =
            start_config_manager(myself.clone(), ConfigManagerArguments {}).await;

        let config = state
            .spawned_actors
            .config_manager
            .as_ref()
            .unwrap()
            .call(ConfigManagerMessage::GetConfig, None)
            .await;

        println!("Config: {:#?}", config.unwrap());

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

async fn start_agent_connection_manager(
    controller: ActorRef<AgentControllerMessage>,
    startup_args: ConnectionManagerStartupArguments,
) -> Option<ActorRef<ConnectionManagerMessage>> {
    // Start the connection manager as a linked actor i.e. Controller is the supervisor

    match controller
        .spawn_linked(
            Some("Connection Manager".to_string()),
            ConnectionManagerActor {},
            startup_args,
        )
        .await
    {
        Ok(result) => Some(result.0),
        Err(error) => {
            error!(errorMsg = %error, "Error spawning {}", "Connection Manager");
            None
        }
    }
}

async fn start_config_manager(
    controller: ActorRef<AgentControllerMessage>,
    startup_args: ConfigManagerArguments,
) -> Option<ActorRef<ConfigManagerMessage>> {
    // Start the config manager manager as a linked actor i.e. Controller is the supervisor

    match controller
        .spawn_linked(
            Some("Config Manager".to_string()),
            ConfigManager {},
            startup_args,
        )
        .await
    {
        Ok(result) => Some(result.0),
        Err(error) => {
            error!(errorMsg = %error, "Error spawning {}", "Config Manager");
            None
        }
    }
}
