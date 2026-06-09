use std::fmt::Display;

#[derive(Debug)]
pub struct ProxySetting {
    server: String,
    port: u16,
    username: String,
    password: String,
}

impl ProxySetting {
    pub fn new(
        server_name: Option<String>,
        server_port: Option<u16>,
        user_name: Option<String>,
        user_password: Option<String>,
    ) -> Self {
        let server = if server_name.is_some() {
            server_name.unwrap()
        } else {
            "".to_string()
        };

        let port = if server_port.is_some() {
            server_port.unwrap()
        } else {
            0
        };

        let username = if user_name.is_some() {
            user_name.unwrap()
        } else {
            "".to_string()
        };

        let password = if user_password.is_some() {
            user_password.unwrap()
        } else {
            "".to_string()
        };

        Self {
            server,
            port,
            username,
            password,
        }
    }

    pub fn proxy_setting_string(&self) -> Option<String> {
        if self.server.is_empty() {
            return None;
        }

        let mut result = String::from("");

        // Add credentials if provided
        if !self.username.is_empty() {
            result.push_str(&self.username);

            if !self.password.is_empty() {
                result.push(':');
                result.push_str(&self.password);
            }

            result.push('@');
        }

        // Add server
        result.push_str(&self.server);

        // Add port if specified
        if self.port != 0 {
            result.push(':');
            result.push_str(&self.port.to_string());
        }

        Some(result)
    }
}

impl Display for ProxySetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let proxy_string = self.proxy_setting_string().unwrap_or_default();

        if proxy_string.contains(':') && proxy_string.contains('@') {
            if let Some((_, host)) = proxy_string.split_once('@') {
                return write!(f, "***:***@{}", host);
            }
        }

        write!(f, "{}", proxy_string)
    }
}
