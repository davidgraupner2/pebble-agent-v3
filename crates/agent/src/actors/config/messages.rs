use crate::actors::config::config::AgentConfig;
use ractor::RpcReplyPort;
use std::fmt;

#[derive(Debug)]
pub enum ConfigUpdate {
    UpdatePingInterval(u16),
    ResetToDefault,
}
#[derive(Debug)]
pub enum ConfigManagerMessage {
    /// Retrieve a copy of the current settings via an RPC port
    GetConfig(RpcReplyPort<AgentConfig>),
    /// Mutate settings and trigger a disk save
    UpdateConfig(ConfigUpdate),
}
