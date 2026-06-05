use agent::registration::challenge::get_api_jwt;
use agent_core::prelude::*;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio::signal;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    name = "pebble-agent",
    about = "Pebble Agent",
    long_about = "Pebble Agent",
    propagate_version = true
)]

struct Args {
    #[arg(long, default_value_t = false)]
    api_server: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Get information about the running agent, including its version and ID")]
    Info,
}

#[cfg(target_os = "linux")]
const AGENT_NAME: &str = "Linux Agent";

#[cfg(target_os = "windows")]
const AGENT_NAME: &str = "Windows Agent";

#[cfg(target_os = "macos")]
const AGENT_NAME: &str = "MacOS Agent";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDiscovery {
    pub host: String,
    pub port: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    RuntimeConstants::init(AGENT_NAME);

    let api_discovery: ApiDiscovery =
        match std::fs::read_to_string(RuntimeConstants::global().api_discovery_file_name()) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(discovery) => {
                    println!("Found discovery: {:#?}", discovery);
                    discovery
                }
                Err(_) => {
                    println!("Unable to get API Discovery");
                    return Err(anyhow::anyhow!("Failed to parse API Discovery"));
                }
            },
            Err(e) => {
                println!(
                    "Unable to read API Discovery file '{}': {}",
                    RuntimeConstants::global().api_discovery_file_name(),
                    e
                );
                return Err(anyhow::anyhow!("Failed to read API Discovery file"));
            }
        };

    // Get any command line arguments
    let cli = Args::parse();

    // Handle info command
    if let Some(Commands::Info) = cli.command {
        println!("--- Agent Info ---");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("ID: {}", RuntimeConstants::global().id());
    } else {
        // // Generate the controllers arguments
        // // - passing in whether we have an API Server or not
        // let agent_runtime_controller_arguments = ControllerArguments {
        //     api_server: cli.api_server,
        // };

        // // Start the runtime controller
        // let (_actor, _actor_handle) = Actor::spawn(
        //     Some("AgentRuntimeController".to_string()),
        //     Controller,
        //     agent_runtime_controller_arguments,
        // )
        // .await
        // .expect("Agent RuntimeController failed to start");
    }

    let jwt = get_api_jwt(
        "http://127.0.0.1:8174/api/v1/registration/challenge",
        "http://127.0.0.1:8174/api/v1/registration/complete",
    )
    .await;

    if jwt.is_ok() {
        println!("We got a JWT: {:#?}", jwt.unwrap());
    } else {
        println!("We got an error: {}", jwt.unwrap_err());
    }

    // Wait here until we receive a CTRL-C Signal break
    signal::ctrl_c().await.expect("Failed to wait");

    // tracing_subscriber::fmt::init();

    Ok(())
}
