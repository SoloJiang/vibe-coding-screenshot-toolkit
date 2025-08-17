use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("permission denied")] Permission,
    #[error("capture failed")] Capture,
    #[error("unsupported operation")] Unsupported,
    #[error("io error")] Io,
    #[error("upload failed")] Upload,
    #[error("ocr empty")] OcrEmpty,
    #[error("ocr failed")] OcrFail,
    #[error("privacy too much hits")] PrivacyTooMuch,
    #[error("hook timeout")] HookTimeout,
    #[error("hook failed")] HookFail,
    #[error("config invalid")] ConfigInvalid,
    #[error("validation error")] Validation,
    #[error("unknown error")] Unknown,
}

#[derive(Debug, Error)]
#[error("{kind}: {message}")]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}
