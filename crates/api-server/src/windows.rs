use crate::server_core::run_server_core;
use crate::{bootstrap_api_server, LOGGING_WORKER_GUARDS, SERVICE_DESCRIPTION};
use crate::{SERVICE_DISPLAY_NAME, SERVICE_NAME};
use agent_core::prelude::RuntimeConstants;
use agent_logging::initialise_logging;
use std::env;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
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

/// Installs the current executable as a Windows Service
pub fn install_service() -> windows_service::Result<()> {
    // Get the path of the current running binary
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
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(SERVICE_DESCRIPTION)?;

    println!("Successfully installed service: {}", SERVICE_DISPLAY_NAME);
    Ok(())
}

/// Uninstalls the service profile from Windows
pub fn uninstall_service() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(1060 as i32) {
                println!("{} is deleted.", SERVICE_DISPLAY_NAME);
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("{} is marked for deletion.", SERVICE_DISPLAY_NAME);
    Ok(())
}

pub fn run() -> Result<()> {
    // Register the service with the Windows Service Control Manager
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn windows_service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        // An error occurred running the service. There is no stdout or stderr
        error!(
            "Error occurred during service startup {}",
            error.to_string()
        );
    }
}

fn run_service() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    // Define event handler to catch STOP commands from Windows
    let event_handler = {
        let shutdown_tx = Arc::clone(&shutdown_tx);
        move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                ServiceControl::Stop => {
                    if let Ok(mut sender) = shutdown_tx.lock() {
                        if let Some(tx) = sender.take() {
                            let _ = tx.send(());
                        }
                    }
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler).unwrap();

    // Tell Windows the service is running
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .unwrap();

    // Start the async runtime and block on our core service logic
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async move {
        match bootstrap_api_server() {
            Ok(bootstrap_parameters) => {
                let runtime_constants = RuntimeConstants::global();

                // Initialise logging
                let worker_guards = initialise_logging(
                    runtime_constants.folders().logs(),
                    runtime_constants.exe_name(),
                    &bootstrap_parameters.logging_format,
                    &bootstrap_parameters.logging_output,
                    Some(&bootstrap_parameters.logging_level),
                );

                // Save the logging guards long term
                // This allows us to write logs for as long as the api runs
                LOGGING_WORKER_GUARDS.set(worker_guards).unwrap();

                run_server_core(bootstrap_parameters, shutdown_rx).await;
            }
            Err(e) => {
                error!("Bootstrap failed in Windows service mode: {}", e);
            }
        }
    });

    // Tell the system that service is stopping.
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_millis(500),
        process_id: None,
    })?;

    // Tell Windows the service has stopped safely
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .unwrap();

    info!("{} has been shutdown...", SERVICE_NAME);

    Ok(())
}
