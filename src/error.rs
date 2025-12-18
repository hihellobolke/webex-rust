use reqwest::StatusCode;

/// Errors that can occur when using the Webex API client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // Foreign errors
    /// IO error from standard library operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    ///
    /// Occurs when parsing API responses or serializing request bodies.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::error::Error),

    /// URL form encoding error when serializing query parameters.
    #[error("URL form encoding error: {0}")]
    FormEncoding(#[from] serde_html_form::ser::Error),

    /// UTF-8 decoding error.
    #[error("UTF8 error: {0}")]
    UTF8(#[from] std::str::Utf8Error),

    /// HTTP client error from reqwest.
    ///
    /// Wraps errors from the underlying HTTP client, including network errors,
    /// connection failures, and timeout errors.
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    // WS/request errors
    /// WebSocket connection was closed.
    ///
    /// The WebSocket connection to the Webex event stream was closed,
    /// either by the server or due to a network error.
    #[error("Connection was closed: {0}")]
    Closed(String),

    /// HTTP error status code without detailed message.
    ///
    /// The API returned an HTTP error status code (4xx or 5xx).
    /// Common codes:
    /// - 401: Unauthorized (invalid or expired token)
    /// - 403: Forbidden (missing OAuth scopes)
    /// - 404: Not Found (resource doesn't exist)
    /// - 429: Too Many Requests (rate limited)
    /// - 500: Internal Server Error
    #[error("HTTP Status: '{0}'")]
    Status(StatusCode),

    /// HTTP error status code with detailed error message.
    ///
    /// Like [`Status`](Error::Status), but includes the error message from the API response.
    #[error("HTTP Status: '{0}' Message: {1}")]
    StatusText(StatusCode, String),

    /// Rate limiting error with optional retry delay.
    ///
    /// The API returned HTTP 429 (Too Many Requests). The second field contains
    /// the number of seconds to wait before retrying, if provided by the API.
    #[error("{0} Retry in: '{1:?}'")]
    Limited(StatusCode, Option<i64>),

    /// WebSocket protocol error from tungstenite.
    ///
    /// Errors from the underlying WebSocket implementation, such as protocol
    /// violations, handshake failures, or frame parsing errors.
    #[error("{0} {1}")]
    Tungstenite(Box<tokio_tungstenite::tungstenite::Error>, String),

    /// Webex API behavior changed unexpectedly.
    ///
    /// The API response format or behavior differs from what this library expects.
    /// This usually indicates that Cisco changed the API in a backwards-incompatible way.
    #[error("Webex API changed: {0}")]
    Api(&'static str),

    /// Authentication or authorization error.
    ///
    /// Generic authentication failure, typically when the token is invalid
    /// or missing required permissions.
    #[error("Authentication error")]
    Authentication,

    /// User-facing error message.
    ///
    /// Error created from application logic with a custom message intended
    /// for end users.
    #[error("{0}")]
    UserError(String),

    // catch-all
    /// Unknown or uncategorized error.
    ///
    /// Fallback error type for errors that don't fit other categories.
    #[error("Unknown error: {0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}
