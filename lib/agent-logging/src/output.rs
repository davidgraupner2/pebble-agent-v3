use std::str::FromStr;
pub enum LogOutput {
    Console,
    File,
    Both,
}

impl FromStr for LogOutput {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" | "FILE" | "File" => Ok(LogOutput::File),
            "console" | "Console" | "CONSOLE" => Ok(LogOutput::Console),
            "both" | "Both" | "BOTH" => Ok(LogOutput::Both),
            _ => Ok(LogOutput::Console),
        }
    }
}
