use crate::actors::connection_manager::messages::ConnectionManagerMessage;
use futures_util::SinkExt;
use futures_util::TryStreamExt;
use ractor::ActorRef;
use ractor::MessagingErr;
use reqwest_websocket::WebSocket;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::{error, info, warn};

/// Handles an active WebSocket connection with automatic heartbeat management.
///
/// This function manages the lifecycle of a WebSocket connection to a remote endpoint,
/// handling incoming messages and periodic heartbeat (ping) operations. It runs until
/// the connection is closed or an error occurs.
///
/// # Arguments
///
/// * `websocket` - The active WebSocket connection to manage.
/// * `ping_interval_seconds` - The interval (in seconds) at which to send heartbeat ping messages.
///   This keeps the connection alive and detects dead connections.
/// * `actor_ref` - Reference to the `ConnectionManagerActor` that owns this WebSocket.
///   Used to notify the actor when the connection is lost.
///
/// # Message Handling
///
/// The function handles the following WebSocket message types:
/// - **Text messages**: Logged for debugging purposes
/// - **Binary messages**: Logged as unsupported with a warning
/// - **Ping messages**: Logged as unsupported with a warning
/// - **Pong messages**: Acknowledged silently (server is still responsive)
/// - **Close frames**: Gracefully terminates the connection
/// - **Errors**: Logs the error and terminates
///
/// # Heartbeat Mechanism
///
/// A background task sends ping messages at regular intervals. If the sender fails,
/// it indicates the connection handler has stopped and the task exits gracefully.
///
/// # Disconnection Handling
///
/// When the WebSocket closes (either gracefully or due to error), the function sends
/// a `Disconnected` message to the actor, allowing it to clean up state and optionally
/// retry the connection.
///
/// # Example
///
/// ```ignore
/// let websocket = establish_connection().await?;
/// handle_websocket(websocket, 30, actor_ref).await;
/// // At this point, the connection is closed and the actor has been notified
/// ```
pub async fn handle_websocket(
    mut websocket: WebSocket,
    ping_interval_seconds: u16,
    actor_ref: ActorRef<ConnectionManagerMessage>,
    cancel_token: CancellationToken,
) {
    // Create a channel for sending ping messages from the heartbeat task
    let (tx_heartbeat, mut rx_heartbeat) = mpsc::channel(100);

    // Spawn heartbeat task
    let heartbeat_interval = Duration::from_secs(ping_interval_seconds as u64);
    tokio::spawn(heartbeat_task(tx_heartbeat, heartbeat_interval));

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                debug!("Websocket cancelled - closing connection");
                return;
            }

            // Handle incoming WebSocket messages
            message = websocket.try_next() => {
                match message {
                    Ok(Some(reqwest_websocket::Message::Text(message))) => {
                        actor_ref.send_message(ConnectionManagerMessage::MessageReceived(message)).ok();
                                        },
                    Ok(Some(reqwest_websocket::Message::Binary(_items))) => {
                        warn!("Binary message received - unsupported!");
                    }
                    Ok(Some(reqwest_websocket::Message::Ping(_items))) => {
                        warn!("Ping message received - unsupported!");
                    }
                    Ok(Some(reqwest_websocket::Message::Pong(_items))) => {
                        info!("Pong received!");
                    },
                    Ok(Some(reqwest_websocket::Message::Close { code, reason })) => {
                        warn!(code=%code, reason=%reason, "Connection Closed");
                        break;
                    },
                    Ok(None) => {
                        warn!("WebSocket stream has ended");
                        break;
                    }
                    Err(error) => {
                        error!(errorMsg=%error, "WebSocket error");
                        break;
                    }
                }
            }

            // Everytime we receive a message on the channel - its time to send a ping.
            Some(_) = rx_heartbeat.recv() => {
                if let Err(error) = websocket.send(reqwest_websocket::Message::Ping(vec![])).await {
                    error!(errorMsg=%error, "Failed to send ping");
                    break;
                }
            }
        }
    }

    info!("WebSocket handler exiting");
    // Notify actor that connection is lost
    let _ = actor_ref.send_message(ConnectionManagerMessage::Disconnected);
}

/// Sends periodic heartbeat (ping) messages at a specified interval.
///
/// This task runs indefinitely, sending signals through the provided channel at regular
/// intervals. It acts as the timing source for the WebSocket heartbeat mechanism.
///
/// # Arguments
///
/// * `tx` - The MPSC sender channel to signal on each heartbeat interval.
/// * `interval` - The duration to wait between heartbeats.
///
/// # Behavior
///
/// The task will exit gracefully if the receiver end of the channel is dropped,
/// indicating that the WebSocket handler has stopped and no longer needs heartbeat signals.
async fn heartbeat_task(tx: mpsc::Sender<()>, interval: Duration) {
    let mut interval_timer = tokio::time::interval(interval);

    loop {
        interval_timer.tick().await;
        if tx.send(()).await.is_err() {
            // Receiver dropped, exit
            break;
        }
    }
}
