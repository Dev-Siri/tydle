use std::str::FromStr;

use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct LogLevel<'a>(&'a str);

impl<'a> FromStr for LogLevel<'a> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let level = s.to_ascii_lowercase();

        match level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => {
                Ok(LogLevel(Box::leak(level.into_boxed_str())))
            }
            _ => Err(anyhow!(
                "Invalid log level: {} (expected one of error, warn, info, debug, trace)",
                s
            )),
        }
    }
}

impl<'a> From<&'a str> for LogLevel<'a> {
    fn from(s: &'a str) -> Self {
        LogLevel::from_str(s).unwrap_or_else(|_| LogLevel("info"))
    }
}

impl<'a> LogLevel<'a> {
    fn as_str(&self) -> &str {
        self.0
    }
}

impl<'a> Default for LogLevel<'a> {
    fn default() -> Self {
        LogLevel("info")
    }
}

#[cfg(feature = "logging")]
pub fn init_logging<L>(level: L)
where
    L: Into<LogLevel<'static>>,
{
    use std::sync::Once;
    static INIT: Once = Once::new();
    let level = level.into();

    INIT.call_once(|| {
        let level_str = level.as_str();
        unsafe { std::env::set_var("RUST_LOG", format!("tydle={}", level_str)) };

        env_logger::init();
        log::info!("Logging initialized at level: {}", level_str);
    });
}

#[cfg(not(feature = "logging"))]
pub fn init_logging(_: Option<&str>) {}
