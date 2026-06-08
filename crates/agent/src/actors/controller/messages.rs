use crate::platform_messages::FunctionCallMessage;

#[derive(Debug)]
pub enum AgentControllerMessage {
    Shutdown,
    ExecuteFunction(FunctionCallMessage),
}
