use crate::{actors::controller::messages::AgentControllerMessage, proxy::ProxySetting};
use ractor::ActorRef;

#[derive(Debug)]
pub struct ConnectionManagerStartupArguments {
    pub controller: ActorRef<AgentControllerMessage>,
    pub connection_string: String,
    pub connection_timeout_seconds: u16,
    pub ping_interval_seconds: u16,
    pub retry_interval_seconds: u16,
    pub pong_response_interval: u16,
    pub proxy: ProxySetting,
}
