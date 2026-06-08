use agent::agent_core::run_agent_core;
#[cfg(windows)]
use agent::windows::{install_service, uninstall_service};
use agent_core::prelude::*;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(clap::ValueEnum, Debug, Clone)]
enum LogFormat {
    Json,
    Full,
    Pretty,
    Compact,
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Json => write!(f, "json"),
            LogFormat::Full => write!(f, "full"),
            LogFormat::Pretty => write!(f, "pretty"),
            LogFormat::Compact => write!(f, "compact"),
        }
    }
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum LogOutput {
    Console,
    File,
    Both,
}

impl std::fmt::Display for LogOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogOutput::Console => write!(f, "console"),
            LogOutput::File => write!(f, "file"),
            LogOutput::Both => write!(f, "both"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    name = "pebble-agent",
    about = "Pebble Agent",
    long_about = "Pebble Agent",
    propagate_version = true,
    help_template = "{before-help}{name} {version}\n{about-section}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
struct Args {
    #[arg(short='s',help="Runs the agent without an accompanying API Server and associated database", long="standalone",num_args=0..=1, default_value = "false", action=clap::ArgAction::Set)]
    standalone: bool,

    #[arg(short='f',long="logformat",help="Sets the log file format", value_enum, default_value_t = LogFormat::Pretty)]
    log_format: LogFormat,

    #[arg(short='l',long="loglevel",help="Sets the level of logging with 'debug' and 'trace' being the most verbose and 'error' the least", value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,

    #[arg(short='o',long="logoutput",help="Sets the log output", value_enum, default_value_t = LogOutput::File)]
    log_output: LogOutput,

    #[arg(
        short = 'a',
        long = "apihost",
        help = "API Host server to communicate with. (Note: this value is ignored if standalone is 'true')",
        default_value = "127.0.0.1"
    )]
    api_host: String,

    #[arg(
        short = 'p',
        long = "apiport",
        help = "API Host server Port to communicate with. (Note: this value is ignored if standalone is 'true')",
        default_value_t = 8174u16
    )]
    api_port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Gets information about the running agent, including its version and ID")]
    Info,
    #[command(about = "Installs the agent to run as a Windows Service")]
    Install,
    #[command(about = "Removed the agent as a Windows Service")]
    Uninstall,
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
    let runtime_constants = RuntimeConstants::global();

    #[cfg(windows)]
    if std::env::args().any(|a| a == "--service") {
        agent::windows::run_service_dispatcher()
            .map_err(|e| anyhow::anyhow!("Windows service dispatcher failed: {}", e))?;
        return Ok(());
    }

    // Get any command line arguments
    let cli = Args::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Info => {
                println!("--- Agent Info ---");
                println!("Version: {}", RuntimeConstants::global().version());
                println!("ID: {}", RuntimeConstants::global().id());
                return Ok(());
            }
            Commands::Install => {
                #[cfg(windows)]
                {
                    if let Err(e) = install_service() {
                        eprintln!("Failed to install service: {}", e);
                    }
                    return Ok(());
                }
                #[cfg(not(windows))]
                {
                    println!("The 'install' command is only supported on Windows targets.");
                    return Ok(());
                }
            }
            Commands::Uninstall => {
                #[cfg(windows)]
                {
                    if let Err(e) = uninstall_service() {
                        eprintln!("Failed to uninstall service: {}", e);
                    }
                    return Ok(());
                }
                #[cfg(not(windows))]
                {
                    println!("The 'uninstall' command is only supported on Windows targets.");
                    return Ok(());
                }
            }
        }
    }
    // Handle info command
    // if let Some(Commands::Info) = cli.command {
    //     println!("--- Agent Info ---");
    //     println!("Version: {}", env!("CARGO_PKG_VERSION"));
    //     println!("ID: {}", RuntimeConstants::global().id());
    //     return Ok(());
    // }

    // Handle install command
    // if let Some(Commands::Install) = cli.command {
    //     println!(
    //         "--- Installing Agent {} as a Windows Service ---",
    //         runtime_constants.version()
    //     );
    //     install_service();
    //     return Ok(());
    // }

    // let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // // Console shutdown source (Ctrl-C)
    // tokio::spawn(async move {
    //     let _ = tokio::signal::ctrl_c().await;
    //     let _ = shutdown_tx.send(true);
    // });

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

    // if !cli.standalone {
    //     let base_url = format!("http://{}:{}", cli.api_host, cli.api_port);
    //     let jwt = get_api_jwt(
    //         &format!("{}/api/v1/registration/challenge", base_url),
    //         &format!("{}/api/v1/registration/complete", base_url),
    //     )
    //     .await;

    //     if jwt.is_ok() {
    //         println!("We got a JWT: {:#?}", jwt.unwrap());
    //     } else {
    //         println!("We got an error: {}", jwt.unwrap_err());
    //     }
    // }

    #[cfg(target_os = "linux")]
    {
        return agent::linux::run_linux(
            cli.standalone,
            cli.api_host,
            cli.api_port,
            cli.log_format.to_string(),
            cli.log_output.to_string(),
            cli.log_level.to_string(),
        )
        .await;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_tx.send(true);
        });

        return run_agent_core(
            cli.standalone,
            cli.api_host,
            cli.api_port,
            cli.log_format.to_string(),
            cli.log_output.to_string(),
            cli.log_level.to_string(),
            shutdown_rx,
        )
        .await;

        Ok(())
    }
}
