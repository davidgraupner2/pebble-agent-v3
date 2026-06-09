use crate::proxy::ProxySetting;

#[derive(Debug)]
pub struct ControllerArguments {
    pub standalone: bool,
    pub api_host: String,
    pub api_port: u16,
    pub log_format: String,
    pub logging_level: String,
    pub log_output: String,
    pub connection_string: String,
    pub connection_timeout: u16,
    pub ping_interval: u16,
    pub retry_interval: u16,
    pub pong_response_interval: u16,
    pub proxy_settings: ProxySetting,
}

impl ControllerArguments {
    pub fn new(
        standalone: bool,
        api_host: String,
        api_port: u16,
        log_format: String,
        log_output: String,
        log_level: String,
        connection_string: String,
        connection_timeout: u16,
        ping_interval: u16,
        retry_interval: u16,
        pong_response_interval: u16,
        proxy_settings: ProxySetting,
    ) -> Self {
        let valid_levels = ["info,warn,error,debug,trace"];
        let logging_level = if valid_levels.contains(&log_level.as_str()) {
            log_level
        } else {
            "info".to_string()
        };

        Self {
            standalone,
            api_host,
            api_port,
            log_format,
            log_output,
            logging_level: logging_level,
            connection_string,
            connection_timeout,
            ping_interval,
            retry_interval,
            pong_response_interval,
            proxy_settings,
        }
    }
}
