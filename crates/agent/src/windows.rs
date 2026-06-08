use crate::agent_core::run_agent_core;
use crate::{SERVICE_DESCRIPTION, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use std::env;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tracing::{error, info};
use windows_service::service::ServiceType;
use windows_service::{
    define_windows_service,
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType},
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
    Result,
};

// Generate the windows service boilerplate via a Macro.
// The boilerplate contains the low-level service entry function (ffi_service_main) that parses
// incoming service arguments into Vec<OsString> and passes them to user defined service
// entry (my_service_main).
define_windows_service!(ffi_service_main, windows_service_main);

#[derive(Clone, Debug)]
struct ServiceRunConfig {
    standalone: bool,
    api_host: String,
    api_port: u16,
    log_format: String,
    log_output: String,
    logging_level: String,
}

impl Default for ServiceRunConfig {
    fn default() -> Self {
        Self {
            standalone: false,
            api_host: "127.0.0.1".to_string(),
            api_port: 8174,
            log_format: "pretty".to_string(),
            log_output: "file".to_string(),
            logging_level: "info".to_string(),
        }
    }
}

fn parse_launch_arguments(arguments: &[OsString]) -> ServiceRunConfig {
    let mut cfg = ServiceRunConfig::default();

    for arg in arguments {
        if let Some(s) = arg.to_str() {
            if s == "--standalone" {
                cfg.standalone = true;
            } else if let Some(v) = s.strip_prefix("--apihost=") {
                cfg.api_host = v.to_string();
            } else if let Some(v) = s.strip_prefix("--apiport=") {
                if let Ok(port) = v.parse::<u16>() {
                    cfg.api_port = port;
                }
            } else if let Some(v) = s.strip_prefix("--logformat=") {
                cfg.log_format = v.to_string();
            } else if let Some(v) = s.strip_prefix("--logoutput=") {
                cfg.log_output = v.to_string();
            } else if let Some(v) = s.strip_prefix("--loglevel=") {
                cfg.logging_level = v.to_string();
            }
        }
    }

    cfg
}

pub fn install_service() -> windows_service::Result<()> {
    let current_exe = env::current_exe().unwrap();

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_info = ServiceInfo {
        name: SERVICE_NAME.into(),
        display_name: SERVICE_DISPLAY_NAME.into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: current_exe,
        launch_arguments: vec![
            OsString::from("--service"),
            OsString::from("--logformat=pretty"),
            OsString::from("--logoutput=file"),
            OsString::from("--loglevel=info"),
            OsString::from("--apihost=127.0.0.1"),
            OsString::from("--apiport=8174"),
        ],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(SERVICE_DESCRIPTION)?;

    println!("Installed service: {}", SERVICE_DISPLAY_NAME);
    Ok(())
}

pub fn uninstall_service() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    service.delete()?;
    if service.query_status()?.current_state != ServiceState::Stopped {
        service.stop()?;
    }
    drop(service);

    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(1060_i32) {
                println!("{} deleted", SERVICE_DISPLAY_NAME);
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }

    println!("{} marked for deletion", SERVICE_DISPLAY_NAME);
    Ok(())
}

pub fn run_service_dispatcher() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn windows_service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        error!("Service failed: {}", e);
    }
}

fn run_service(arguments: Vec<OsString>) -> Result<()> {
    let config = parse_launch_arguments(&arguments);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let shutdown_tx_for_handler = shutdown_tx.clone();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                let _ = shutdown_tx_for_handler.send(true);
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime creation failed");
    rt.block_on(async move {
        if let Err(e) = run_agent_core(
            config.standalone,
            config.api_host,
            config.api_port,
            config.log_format,
            config.log_output,
            config.logging_level,
            shutdown_rx,
        )
        .await
        {
            error!("Agent core error in service mode: {}", e);
        }
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_millis(500),
        process_id: None,
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("{} stopped", SERVICE_NAME);
    Ok(())
}
