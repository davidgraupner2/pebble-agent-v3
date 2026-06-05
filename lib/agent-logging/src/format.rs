use std::str::FromStr;
pub(crate) enum LogFileFormat {
    JSON,
    Full,
    Pretty,
    Compact,
}

impl FromStr for LogFileFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" | "JSON" => Ok(LogFileFormat::JSON),
            "full" | "Full" => Ok(LogFileFormat::Full),
            "pretty" | "Pretty" => Ok(LogFileFormat::Pretty),
            "compact" | "Compact" | "COMPACT" => Ok(LogFileFormat::Compact),
            _ => Ok(LogFileFormat::JSON),
        }
    }
}
