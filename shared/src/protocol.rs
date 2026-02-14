use serde::{Deserialize, Serialize};

/// Messages sent from platform to app
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PlatformMessage {
    /// Upload a file to the app
    UploadFile {
        filename: String,
        #[serde(with = "base64_serde")]
        data: Vec<u8>,
    },
    /// Request app to send file data for download
    RequestDownload,
    /// Delete selected file/directory
    Delete,
    /// Custom command with arbitrary data
    Command {
        command: String,
        #[serde(default)]
        params: serde_json::Value,
    },
}

/// Messages sent from app to platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AppMessage {
    /// App state update
    State {
        /// Current path/location in the app
        path: String,
        /// Currently selected item (if any)
        selected: Option<String>,
        /// Available actions based on current context
        actions: Vec<String>,
        /// Additional state data
        #[serde(default)]
        metadata: serde_json::Value,
    },
    /// File data for download
    DownloadData {
        filename: String,
        #[serde(with = "base64_serde")]
        data: Vec<u8>,
    },
    /// Operation completed successfully
    Success {
        operation: String,
        #[serde(default)]
        message: Option<String>,
    },
    /// Error occurred
    Error {
        message: String,
        #[serde(default)]
        code: Option<String>,
    },
    /// Log message for debugging
    Log {
        level: LogLevel,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Helper module for base64 encoding/decoding with serde
mod base64_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use base64::{engine::general_purpose::STANDARD, Engine};
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}
