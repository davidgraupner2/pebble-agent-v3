use crate::{
    actors::{
        connection_manager::{
            arguments::ConnectionManagerStartupArguments, messages::ConnectionManagerMessage,
            state::ConnectionManagerState, utils::handle_websocket,
        },
        controller::messages::AgentControllerMessage,
    },
    platform_messages::{get_function_call_message, get_message_type},
    proxy::ProxySetting,
};
use agent_core::prelude::RuntimeConstants;
use anyhow::{anyhow, Result};
use ractor::{Actor, ActorProcessingErr, ActorRef, MessagingErr};
use reqwest::{ClientBuilder, Proxy};
use reqwest_websocket::RequestBuilderExt;
use reqwest_websocket::{UpgradeResponse, WebSocket};
use std::sync::{atomic::Ordering::Relaxed, Arc};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, trace, warn};
use url::Url;

#[derive(Debug)]
pub struct ConnectionManagerActor {}

impl Actor for ConnectionManagerActor {
    type State = ConnectionManagerState;
    type Msg = ConnectionManagerMessage;
    type Arguments = ConnectionManagerStartupArguments;

    #[instrument(name = "Connection Manager - Pre Start", level = "trace")]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Initialise the actor state
        let state = ConnectionManagerState::new(
            args.controller,
            args.connection_string,
            args.connection_timeout_seconds,
            args.ping_interval_seconds,
            args.retry_interval_seconds,
            args.pong_response_interval,
            args.proxy,
        );

        Ok(state)
    }

    #[instrument(name = "Connection Manager - Post Start", level = "trace")]
    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Start the connection manager main task
        // - To connect to the endpoint
        let _ = myself.send_message(ConnectionManagerMessage::Connect);

        info!(name = "Connection Manager", "started successfully");

        Ok(())
    }

    #[instrument(name = "Connection Manager - Post Stop", level = "trace")]
    async fn post_stop(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!(name = "Connection Manager", "stopped");

        Ok(())
    }

    #[instrument(name = "Connection  Manager - Handle Message", level = "trace")]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ConnectionManagerMessage::Connect => {
                match self.try_connect(state).await {
                    Ok(websocket) => {
                        state.is_running.store(true, Relaxed);

                        // Create a FRESH cancellationtoken for THIS connection
                        state.websocket_cancel_token = CancellationToken::new();

                        // Spawn the WebSocket handler task
                        // - This tasks managed the heartbeat and the messages received from the endpoint
                        tokio::spawn(handle_websocket(
                            websocket,
                            state.ping_interval_seconds,
                            myself,
                            state.websocket_cancel_token.clone(),
                        ));
                    }
                    Err(error) => {
                        error!(errorMsg=%error, "Connection failed - will retry the connection");
                        self.schedule_retry(myself, state.retry_interval_seconds);
                    }
                }
            }
            ConnectionManagerMessage::Disconnected => {
                info!("WebSocket disconnected");
                state.is_running.store(false, Relaxed);
                self.schedule_retry(myself, state.retry_interval_seconds);
            }
            ConnectionManagerMessage::MessageReceived(message) => {
                trace!(message = %message, "Received message from WebSocket endpoint");
            }
        }

        Ok(())
    }
}

impl ConnectionManagerActor {
    // pub fn extract_version_and_type_from_message(&self, message: &str) -> Option<(String, String)> {
    //     let v: Value = serde_json::from_str(message).ok()?;
    //     let version = v.get("$version")?.as_str()?.to_string();
    //     let event_type = v.get("$type")?.as_str()?.to_string();
    //     Some((version, event_type))
    // }

    pub async fn try_connect(&self, state: &mut ConnectionManagerState) -> Result<WebSocket> {
        let connection_string = &state.connection_string;
        let client_id = RuntimeConstants::global().id();
        let groups = "windows";
        let friendly_name = RuntimeConstants::global().host_name();
        let url = Url::parse_with_params(
            connection_string,
            [
                ("client_id", client_id),
                ("groups", groups),
                ("friendlyName", friendly_name),
            ],
        )
        .unwrap()
        .to_string();
        // Loop through the connection strings we have
        match self
            .connect_timeout(
                url,
                state.connection_timeout_seconds.try_into().unwrap(),
                &state.proxy,
            )
            .await
        {
            Ok(websocket) => {
                info!("Agent now connected to endpoint. Will establish keep-alives every {:?} seconds",state.ping_interval_seconds);

                return Ok(websocket);
            }
            Err(error) => {
                warn!(errorMsg=%error,"Agent was not able to connect to endpoint using the provided connection string");
            }
        }
        // If we exhausted all connection strings, return an error
        Err(anyhow!(
            "No connection string allowed a valid connection to the endpoint"
        ))
    }

    pub async fn connect_timeout(
        &self,
        url: String,
        timeout: usize,
        proxy: &ProxySetting,
    ) -> Result<WebSocket> {
        let connection_timeout = Duration::from_secs(timeout as u64);

        let websocket_client = self.create_client(&proxy, connection_timeout)?;

        let response: UpgradeResponse = websocket_client.get(&url).upgrade().send().await?;

        // if everything went ok - return the websocket connection
        Ok(response.into_websocket().await?)
    }

    fn create_client(
        &self,
        proxy_setting: &ProxySetting,
        timeout: Duration,
    ) -> Result<reqwest::Client, reqwest::Error> {
        let builder = ClientBuilder::new().timeout(timeout);

        if let Some(proxy_url) = proxy_setting.proxy_setting_string() {
            let proxy = Proxy::all(proxy_url)?;

            info!("Using proxy server: {}", proxy_setting);

            Ok(builder.proxy(proxy).build()?)
        } else {
            Ok(builder.build()?)
        }
    }

    fn schedule_retry(
        &self,
        actor_ref: ActorRef<ConnectionManagerMessage>,
        retry_interval_seconds: u16,
    ) {
        let retry_interval = Duration::from_secs(retry_interval_seconds as u64);

        tokio::spawn(async move {
            tokio::time::sleep(retry_interval).await;
            let _ = actor_ref.send_message(ConnectionManagerMessage::Connect);
        });
    }
}
