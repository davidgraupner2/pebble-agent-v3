#[derive(Debug)]
pub enum ConnectionManagerMessage {
    // Retry,
    Connect,
    Disconnected,
    MessageReceived(String),
}
