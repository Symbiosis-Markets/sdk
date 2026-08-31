#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// A non-success response from the API, with the server's error message
    /// when one was provided.
    #[error("api error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("failed to encode request body: {0}")]
    Encode(#[source] serde_json::Error),

    #[error("failed to decode response body: {0}")]
    Decode(#[source] serde_json::Error),

    #[error("this endpoint requires a credential, but none is configured")]
    MissingCredential,

    #[error("websocket methods require ClientBuilder::ws_url")]
    MissingWsUrl,

    #[error("invalid base url: {0}")]
    InvalidUrl(String),

    #[error("environment variable {0} is not set")]
    MissingEnv(&'static str),

    #[error("timed out before the flow completed")]
    Timeout,

    /// The event stream closed before the flow completed.
    #[error("the event stream closed before the flow completed")]
    StreamClosed,

    #[error("system clock is before the unix epoch")]
    Clock,

    #[cfg(feature = "ws")]
    #[error("websocket error: {0}")]
    Ws(#[from] Box<tokio_tungstenite::tungstenite::Error>),

    /// The server rejected the websocket handshake and replied with a plain
    /// text reason instead of a snapshot frame.
    #[cfg(feature = "ws")]
    #[error("websocket handshake rejected: {0}")]
    WsHandshake(String),
}

#[cfg(feature = "ws")]
impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(error: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::Ws(Box::new(error))
    }
}
