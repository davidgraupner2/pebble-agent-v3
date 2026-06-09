use crate::{actors::controller::messages::AgentControllerMessage, proxy::ProxySetting};
use ractor::ActorRef;
use std::sync::atomic::AtomicBool;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct ConnectionManagerState {
    pub controller: ActorRef<AgentControllerMessage>,
    pub connection_string: String,
    pub connection_timeout_seconds: u16,
    pub ping_interval_seconds: u16,
    pub retry_interval_seconds: u16,
    pub is_running: AtomicBool,
    pub websocket_cancel_token: CancellationToken,
    pub pong_response_interval: u16,
    pub proxy: ProxySetting,
}

impl ConnectionManagerState {
    pub fn new(
        controller: ActorRef<AgentControllerMessage>,
        connection_string: String,
        connection_timeout: u16,
        ping_interval: u16,
        retry_interval: u16,
        pong_response_interval: u16,
        proxy_settings: ProxySetting,
    ) -> Self {
        let is_running = AtomicBool::new(false);

        Self {
            connection_timeout_seconds: connection_timeout,
            connection_string,
            ping_interval_seconds: ping_interval,
            retry_interval_seconds: retry_interval,
            pong_response_interval,
            proxy: proxy_settings,
            is_running,
            websocket_cancel_token: CancellationToken::new(),
            controller,
        }
    }
}
