pub(crate) mod format;
pub(crate) mod output;

use crate::format::LogFileFormat;
use crate::output::LogOutput;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::str::FromStr;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

/// Initialise logging to console only. .
fn initialise_logging_console(log_file_format: &str, filter: EnvFilter) -> Vec<WorkerGuard> {
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_writer(std::io::stdout);

    match log_file_format {
        "json" => {
            let subscriber = subscriber.json().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "pretty" => {
            let subscriber = subscriber.pretty().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "compact" => {
            let subscriber = subscriber.compact().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        _ => {
            let subscriber = subscriber.finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
    }
    Vec::new()
}

/// Initialise logging to a rolling daily file.
fn initialise_logging_file(
    log_file_folder: &PathBuf,
    log_file_name: &str,
    log_file_format: &str,
    filter: EnvFilter,
) -> Vec<WorkerGuard> {
    let file_appender = rolling::daily(log_file_folder, log_file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false);

    match log_file_format {
        "json" => {
            let subscriber = subscriber.json().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "pretty" => {
            let subscriber = subscriber.pretty().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "compact" => {
            let subscriber = subscriber.compact().finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        _ => {
            let subscriber = subscriber.finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
    }

    vec![guard]
}

/// Initialise logging to both console and file.
fn initialise_logging_both(
    log_file_folder: &PathBuf,
    log_file_name: &str,
    log_file_format: &str,
    filter: EnvFilter,
) -> Vec<WorkerGuard> {
    let _ = LogFileFormat::from_str(log_file_format).unwrap_or(LogFileFormat::Pretty);

    let file_appender = rolling::daily(log_file_folder, log_file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    match log_file_format {
        "json" => {
            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(filter.clone());

            let console_layer = fmt::layer()
                .json()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(filter);

            let subscriber = Registry::default().with(file_layer).with(console_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "pretty" => {
            let file_layer = fmt::layer()
                .pretty()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(filter.clone());

            let console_layer = fmt::layer()
                .pretty()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(filter);

            let subscriber = Registry::default().with(file_layer).with(console_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        "compact" => {
            let file_layer = fmt::layer()
                .compact()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(filter.clone());

            let console_layer = fmt::layer()
                .compact()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(filter);

            let subscriber = Registry::default().with(file_layer).with(console_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
        _ => {
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(filter.clone());

            let console_layer = fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(filter);

            let subscriber = Registry::default().with(file_layer).with(console_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("setting default tracing subscriber failed");
        }
    }

    vec![guard]
}

/// New wrapper that accepts an explicit output selection.
pub fn initialise_logging(
    log_file_folder: &PathBuf,
    log_file_name: &str,
    log_file_format: &str,
    output: &str,
    override_filter: Option<&str>,
) -> Vec<WorkerGuard> {
    // If a filter was passed in, use that - otherwise use the env filter
    let filter = if let Some(level) = override_filter {
        EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    match LogOutput::from_str(output) {
        Ok(LogOutput::Console) => initialise_logging_console(log_file_format, filter),
        Ok(LogOutput::File) => {
            initialise_logging_file(log_file_folder, log_file_name, log_file_format, filter)
        }
        Ok(LogOutput::Both) => {
            initialise_logging_both(log_file_folder, log_file_name, log_file_format, filter)
        }
        _ => initialise_logging_file(log_file_folder, log_file_name, log_file_format, filter),
    }
}
