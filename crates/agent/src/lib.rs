pub mod actors;
pub mod agent_core;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod platform_messages;
pub mod registration;

#[cfg(windows)]
pub mod windows;

pub const SERVICE_NAME: &str = "PebbleAgentService";
pub const SERVICE_DISPLAY_NAME: &str = "Pebble Agent V3";
pub const SERVICE_DESCRIPTION: &str = "This Agent is part of the Pebble Agent Suite and serves to a provide an automation layer on top of existing Windows computers";
