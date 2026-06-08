#[derive(Debug)]
pub struct ControllerArguments {
    pub standalone: bool,
    pub api_host: String,
    pub api_port: u16,
    pub log_format: String,
    pub logging_level: String,
    pub log_output: String,
}

impl ControllerArguments {
    pub fn new(
        standalone: bool,
        api_host: String,
        api_port: u16,
        log_file_format: String,
        log_file_output: String,
        log_level: String,
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
            log_format: log_file_format,
            log_output: log_file_output,
            logging_level: logging_level,
        }
    }
}
