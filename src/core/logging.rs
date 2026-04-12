use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn to_usize(&self) -> usize {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

impl LogEntry {
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        LogEntry {
            timestamp,
            level: level.as_str().to_string(),
            target: target.into(),
            message: message.into(),
            task_id: None,
            attributes: None,
        }
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        if self.attributes.is_none() {
            self.attributes = Some(HashMap::new());
        }
        if let Some(ref mut attrs) = self.attributes {
            attrs.insert(key.into(), value.into());
        }
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

pub struct Logger {
    min_level: Arc<RwLock<LogLevel>>,
    outputs: Arc<RwLock<Vec<LogOutput>>>,
}

#[derive(Debug, Clone)]
pub enum LogOutput {
    Stdout,
    Stderr,
    Callback(String),
    File(String),
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            min_level: Arc::new(RwLock::new(LogLevel::Info)),
            outputs: Arc::new(RwLock::new(vec![LogOutput::Stdout])),
        }
    }

    pub async fn set_level(&self, level: LogLevel) {
        let mut min = self.min_level.write().await;
        *min = level;
    }

    pub async fn add_output(&self, output: LogOutput) {
        let mut outputs = self.outputs.write().await;
        outputs.push(output);
    }

    pub async fn log(&self, entry: &LogEntry) {
        let min_level = *self.min_level.read().await;
        let level = match entry.level.as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        };

        if level < min_level {
            return;
        }

        let outputs = self.outputs.read().await;
        for output in outputs.iter() {
            match output {
                LogOutput::Stdout => {
                    println!("{}", entry.to_json());
                }
                LogOutput::Stderr => {
                    eprintln!("{}", entry.to_json());
                }
                LogOutput::Callback(_url) => {
                }
                LogOutput::File(_path) => {
                }
            }
        }
    }

    pub fn trace(&self, target: impl Into<String>, message: impl Into<String>) -> LogEntry {
        LogEntry::new(LogLevel::Trace, target, message)
    }

    pub fn debug(&self, target: impl Into<String>, message: impl Into<String>) -> LogEntry {
        LogEntry::new(LogLevel::Debug, target, message)
    }

    pub fn info(&self, target: impl Into<String>, message: impl Into<String>) -> LogEntry {
        LogEntry::new(LogLevel::Info, target, message)
    }

    pub fn warn(&self, target: impl Into<String>, message: impl Into<String>) -> LogEntry {
        LogEntry::new(LogLevel::Warn, target, message)
    }

    pub fn error(&self, target: impl Into<String>, message: impl Into<String>) -> LogEntry {
        LogEntry::new(LogLevel::Error, target, message)
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_LOGGER: once_cell::sync::Lazy<Logger> = once_cell::sync::Lazy::new(Logger::new);

pub fn get_global_logger() -> &'static Logger {
    &GLOBAL_LOGGER
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        let entry = $crate::core::logging::get_global_logger().trace("tthsd", format!($($arg)*));
        let _ = $crate::core::logging::get_global_logger().log(&entry);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        let entry = $crate::core::logging::get_global_logger().debug("tthsd", format!($($arg)*));
        let _ = $crate::core::logging::get_global_logger().log(&entry);
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        let entry = $crate::core::logging::get_global_logger().info("tthsd", format!($($arg)*));
        let _ = $crate::core::logging::get_global_logger().log(&entry);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        let entry = $crate::core::logging::get_global_logger().warn("tthsd", format!($($arg)*));
        let _ = $crate::core::logging::get_global_logger().log(&entry);
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        let entry = $crate::core::logging::get_global_logger().error("tthsd", format!($($arg)*));
        let _ = $crate::core::logging::get_global_logger().log(&entry);
    };
}