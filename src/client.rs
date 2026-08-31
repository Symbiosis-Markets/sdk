use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::types::{
    ApiKeySummary, AssetId, AuditEvent, BalanceRequest, BalanceResponse, Chain,
    CreateApiKeyRequest, CreateApiKeyResponse, CreateDepositAddressRequest, CreateQuoteBody,
    DepositAddress, ErrorResponse, GetDepositAddressesResponse, GetOffersResponse,
    GetRequestsResponse, LoginRequest, LoginResponse, OfferQuoteResponse, QuoteId, RequestId,
    RequestQuoteBody, RequestQuoteResponse, SignupRequest, SignupResponse, UsdcBalanceResponse,
    Venue, WithdrawRequest, WithdrawResponse, WsTicketResponse,
};
use crate::{Error, Result};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The production REST endpoint.
pub const PRODUCTION_URL: &str = "https://api.symbiosis.markets";

/// The production websocket endpoint.
pub const PRODUCTION_WS_URL: &str = "wss://ws.symbiosis.markets";

/// A credential for authenticated endpoints.
#[derive(Clone)]
pub enum Credential {
    Session { token: String },
    ApiKey { key_id: String, secret: Vec<u8> },
}

impl Credential {
    /// A session token from [`Client::login`].
    pub fn session(token: impl Into<String>) -> Self {
        Credential::Session {
            token: token.into(),
        }
    }

    /// An API key id and secret from [`Client::create_api_key`].
    pub fn api_key(key_id: impl Into<String>, secret: impl Into<Vec<u8>>) -> Self {
        Credential::ApiKey {
            key_id: key_id.into(),
            secret: secret.into(),
        }
    }
}

/// Redacted so a credential can never leak through logs.
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credential::Session { .. } => f.write_str("Credential::Session { .. }"),
            Credential::ApiKey { key_id, .. } => f
                .debug_struct("Credential::ApiKey")
                .field("key_id", key_id)
                .finish_non_exhaustive(),
        }
    }
}

#[derive(Debug)]
pub struct ClientBuilder {
    base_url: String,
    #[cfg(feature = "ws")]
    ws_url: Option<String>,
    credential: Option<Credential>,
    timeout: Duration,
    http: Option<reqwest::Client>,
}

impl ClientBuilder {
    pub fn credential(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Base url of the websocket service, e.g. `wss://ws.symbiosis.markets`.
    /// Required by the stream methods.
    #[cfg(feature = "ws")]
    pub fn ws_url(mut self, ws_url: impl Into<String>) -> Self {
        self.ws_url = Some(trim_trailing_slash(ws_url.into()));
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Supply a preconfigured [`reqwest::Client`], for proxies or connection
    /// tuning. Overrides [`Self::timeout`].
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    pub fn build(self) -> Result<Client> {
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(Error::InvalidUrl(self.base_url));
        }

        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .timeout(self.timeout)
                .build()
                .map_err(Error::Transport)?,
        };

        Ok(Client {
            base_url: self.base_url,
            #[cfg(feature = "ws")]
            ws_url: self.ws_url,
            credential: self.credential,
            http,
        })
    }
}

/// A client for the Symbiosis API.
///
/// Cheap to clone; clones share the underlying connection pool.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    #[cfg(feature = "ws")]
    ws_url: Option<String>,
    credential: Option<Credential>,
    http: reqwest::Client,
}

impl Client {
    pub fn builder(base_url: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            base_url: trim_trailing_slash(base_url.into()),
            #[cfg(feature = "ws")]
            ws_url: None,
            credential: None,
            timeout: DEFAULT_TIMEOUT,
            http: None,
        }
    }

    /// A builder preconfigured with the production endpoints.
    pub fn production() -> ClientBuilder {
        let builder = Client::builder(PRODUCTION_URL);
        #[cfg(feature = "ws")]
        let builder = builder.ws_url(PRODUCTION_WS_URL);

        builder
    }

    /// A client authenticated from the environment: the API key in
    /// `SYMBIOSIS_API_KEY_ID` and `SYMBIOSIS_API_KEY_SECRET`, and the
    /// production endpoints unless `SYMBIOSIS_API_URL` (and, with the `ws`
    /// feature, `SYMBIOSIS_WS_URL`) override them.
    pub fn from_env() -> Result<Client> {
        let key_id = env_var("SYMBIOSIS_API_KEY_ID")?;
        let secret = env_var("SYMBIOSIS_API_KEY_SECRET")?;

        let base_url =
            std::env::var("SYMBIOSIS_API_URL").unwrap_or_else(|_| PRODUCTION_URL.to_owned());
        let builder = Client::builder(base_url);
        #[cfg(feature = "ws")]
        let builder = builder.ws_url(
            std::env::var("SYMBIOSIS_WS_URL").unwrap_or_else(|_| PRODUCTION_WS_URL.to_owned()),
        );

        builder
            .credential(Credential::api_key(key_id, secret))
            .build()
    }

    /// A clone of this client using `credential`, keeping the endpoints,
    /// timeout, and connection pool.
    pub fn with_credential(&self, credential: Credential) -> Client {
        let mut client = self.clone();
        client.credential = Some(credential);

        client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // Auth

    /// Create an account. Does not authenticate the client; follow with
    /// [`Self::login`].
    pub async fn signup(&self, email: &str, password: &str) -> Result<SignupResponse> {
        let body = SignupRequest {
            email: email.to_owned(),
            password: password.to_owned(),
        };

        self.request(Method::POST, "/auth/signup", Some(&body), false)
            .await
    }

    /// Exchange an email and password for a session token. The token is
    /// returned, not installed; build a new client with
    /// [`Credential::session`] to use it.
    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse> {
        let body = LoginRequest {
            email: email.to_owned(),
            password: password.to_owned(),
        };

        self.request(Method::POST, "/auth/login", Some(&body), false)
            .await
    }

    /// List the user's API keys. Requires a session credential.
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeySummary>> {
        self.request(Method::GET, "/auth/api-keys", None::<&()>, true)
            .await
    }

    /// Mint an API key. The response is the only time the raw secret is
    /// returned. Requires a session credential.
    pub async fn create_api_key(&self, body: &CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        self.request(Method::POST, "/auth/api-keys", Some(body), true)
            .await
    }

    /// Revoke one of the user's API keys. Requires a session credential.
    pub async fn revoke_api_key(&self, api_key_id: &str) -> Result<()> {
        let path = format!("/auth/api-keys/{api_key_id}");

        self.request_empty(Method::DELETE, &path, None::<&()>).await
    }

    /// List the user's audit log events. Requires a session credential.
    pub async fn list_audit_events(&self) -> Result<Vec<AuditEvent>> {
        self.request(Method::GET, "/auth/audit-log", None::<&()>, true)
            .await
    }

    /// Mint a websocket ticket: read-only from a session credential, scoped
    /// from an API key credential.
    pub async fn ws_ticket(&self) -> Result<WsTicketResponse> {
        let path = match self.credential {
            Some(Credential::Session { .. }) => "/auth/ws-ticket",
            Some(Credential::ApiKey { .. }) => "/auth/ws-ticket/signed",
            None => return Err(Error::MissingCredential),
        };

        self.request(Method::POST, path, None::<&()>, true).await
    }

    // Custody

    pub async fn create_deposit_address(&self, chain: Chain) -> Result<DepositAddress> {
        let body = CreateDepositAddressRequest { chain };

        self.request(
            Method::POST,
            "/custody/create-deposit-address",
            Some(&body),
            true,
        )
        .await
    }

    pub async fn get_deposit_addresses(&self) -> Result<GetDepositAddressesResponse> {
        self.request(
            Method::GET,
            "/custody/get-deposit-addresses",
            None::<&()>,
            true,
        )
        .await
    }

    /// Queue a withdrawal to an external address. Requires the `withdraw`
    /// scope.
    pub async fn withdraw(&self, body: &WithdrawRequest) -> Result<WithdrawResponse> {
        self.request(Method::POST, "/custody/withdraw", Some(body), true)
            .await
    }

    pub async fn get_balance(&self, venue: Venue, asset_id: AssetId) -> Result<BalanceResponse> {
        let body = BalanceRequest { venue, asset_id };

        self.request(Method::GET, "/custody/get-balance", Some(&body), true)
            .await
    }

    pub async fn get_usdc_balance(&self) -> Result<UsdcBalanceResponse> {
        self.request(Method::GET, "/custody/get-usdc-balance", None::<&()>, true)
            .await
    }

    // RFQ

    /// Create a quote request.
    pub async fn request_quote(&self, body: &RequestQuoteBody) -> Result<RequestQuoteResponse> {
        self.request(Method::POST, "/rfq/request", Some(body), true)
            .await
    }

    /// Cancel one of the user's open requests.
    pub async fn cancel_request(&self, request_id: RequestId) -> Result<()> {
        let body = serde_json::json!({ "request_id": request_id });

        self.request_empty(Method::DELETE, "/rfq/request", Some(&body))
            .await
    }

    /// Get the open requests for a venue and asset. Both filters are
    /// optional; identifiers are hex and enum strings, so no encoding is
    /// needed.
    pub async fn get_open_requests(
        &self,
        venue: Option<Venue>,
        asset_id: Option<AssetId>,
    ) -> Result<GetRequestsResponse> {
        let mut path = String::from("/rfq/request");
        let mut sep = '?';

        if let Some(venue) = venue {
            path.push(sep);
            path.push_str("venue_id=");
            path.push_str(venue.as_str());
            sep = '&';
        }
        if let Some(asset_id) = asset_id {
            path.push(sep);
            path.push_str("asset_id=");
            path.push_str(&asset_id.to_string());
        }

        self.request(Method::GET, &path, None::<&()>, true).await
    }

    /// Get all the user's open requests.
    pub async fn get_user_requests(&self) -> Result<GetRequestsResponse> {
        self.request(Method::GET, "/rfq/get-requests", None::<&()>, true)
            .await
    }

    /// Get the open quotes for one of the user's requests.
    pub async fn get_quotes(&self, request_id: RequestId) -> Result<GetOffersResponse> {
        let path = format!("/rfq/quote?request_id={request_id}");

        self.request(Method::GET, &path, None::<&()>, true).await
    }

    /// Quote an open request. `price` is scaled by
    /// [`crate::types::SCALE_FACTOR`].
    pub async fn create_quote(
        &self,
        request_id: RequestId,
        price: u64,
    ) -> Result<OfferQuoteResponse> {
        let body = CreateQuoteBody { request_id, price };

        self.request(Method::POST, "/rfq/quote", Some(&body), true)
            .await
    }

    /// Accept a quote, matching the request.
    pub async fn accept_quote(&self, offer_id: QuoteId) -> Result<()> {
        let body = serde_json::json!({ "offer_id": offer_id });

        self.request_empty(Method::POST, "/rfq/accept", Some(&body))
            .await
    }

    // Websockets

    /// Connect the user event stream: quotes on the user's requests, RFQ
    /// matches, and trade events. Mints a fresh ticket per call.
    #[cfg(feature = "ws")]
    pub async fn user_stream(&self) -> Result<crate::ws::EventStream> {
        let ws_url = self.ws_url.as_deref().ok_or(Error::MissingWsUrl)?;
        let ticket = self.ws_ticket().await?.ticket;

        crate::ws::connect_user(ws_url, &ticket).await
    }

    /// Subscribe to the RFQ streams of `markets`. Returns the open-request
    /// snapshot together with the live event stream. Mints a fresh ticket
    /// per call.
    #[cfg(feature = "ws")]
    pub async fn rfq_stream(
        &self,
        markets: &[crate::types::BookIdentifier],
    ) -> Result<crate::ws::RfqSubscription> {
        let ws_url = self.ws_url.as_deref().ok_or(Error::MissingWsUrl)?;
        let ticket = self.ws_ticket().await?.ticket;

        crate::ws::connect_rfq(ws_url, &ticket, markets).await
    }

    /// The user stream with automatic reconnection: when the connection
    /// drops, a fresh ticket is minted and the stream resumes after a short
    /// pause. Events sent while disconnected are not replayed.
    #[cfg(feature = "ws")]
    pub async fn user_stream_reconnecting(&self) -> Result<ReconnectingUserStream> {
        Ok(ReconnectingUserStream {
            inner: self.user_stream().await?,
            client: self.clone(),
        })
    }

    /// The whole taker flow in one call: request a quote, accept the first
    /// offer to arrive on the user stream, and return the match
    /// confirmation. Fails with [`Error::Timeout`] if no offer arrives and
    /// matches within `timeout`.
    #[cfg(feature = "ws")]
    pub async fn request_and_accept_first(
        &self,
        body: &RequestQuoteBody,
        timeout: Duration,
    ) -> Result<crate::ws::RfqMatch> {
        use crate::ws::WsEvent;

        // Connect before requesting so no offer can slip past.
        let mut stream = self.user_stream().await?;
        let request_id = self.request_quote(body).await?.request_id;

        let flow = async {
            let mut accepted = false;
            while let Some(event) = stream.next_event().await {
                match event? {
                    WsEvent::RfqOffer(quote) if quote.request_id == request_id && !accepted => {
                        self.accept_quote(quote.quote_id).await?;
                        accepted = true;
                    }
                    WsEvent::RfqMatch(rfq_match) if rfq_match.request_id == request_id => {
                        return Ok(rfq_match);
                    }
                    _ => {}
                }
            }

            Err(Error::StreamClosed)
        };

        tokio::time::timeout(timeout, flow)
            .await
            .map_err(|_| Error::Timeout)?
    }

    // Plumbing

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path_and_query: &str,
        body: Option<&impl Serialize>,
        authed: bool,
    ) -> Result<T> {
        let response = self.send(method, path_and_query, body, authed).await?;
        let bytes = response.bytes().await.map_err(Error::Transport)?;

        serde_json::from_slice(&bytes).map_err(Error::Decode)
    }

    async fn request_empty(
        &self,
        method: Method,
        path_and_query: &str,
        body: Option<&impl Serialize>,
    ) -> Result<()> {
        self.send(method, path_and_query, body, true).await?;

        Ok(())
    }

    async fn send(
        &self,
        method: Method,
        path_and_query: &str,
        body: Option<&impl Serialize>,
        authed: bool,
    ) -> Result<reqwest::Response> {
        let body_bytes = match body {
            Some(body) => serde_json::to_vec(body).map_err(Error::Encode)?,
            None => Vec::new(),
        };

        let url = format!("{}{}", self.base_url, path_and_query);
        let mut request = self.http.request(method.clone(), url);

        if authed {
            request = match &self.credential {
                Some(Credential::Session { token }) => {
                    request.header(AUTHORIZATION, format!("Bearer {token}"))
                }
                Some(Credential::ApiKey { key_id, secret }) => {
                    let timestamp_ms = now_ms()?;
                    let message = crate::sign::canonical_message(
                        timestamp_ms,
                        method.as_str(),
                        path_and_query,
                        &body_bytes,
                    );

                    request
                        .header("APIKEY", key_id)
                        .header("HMAC_TIMESTAMP", timestamp_ms.to_string())
                        .header("HMAC_SIGNATURE", crate::sign::sign(secret, &message))
                }
                None => return Err(Error::MissingCredential),
            };
        }

        // The signature covers `body_bytes` exactly, so those bytes are what
        // must go on the wire.
        if body.is_some() {
            request = request
                .header(CONTENT_TYPE, "application/json")
                .body(body_bytes);
        }

        let response = request.send().await.map_err(Error::Transport)?;
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        let message = match response.text().await {
            Ok(text) => match serde_json::from_str::<ErrorResponse>(&text) {
                Ok(parsed) => parsed.error,
                Err(_) if !text.trim().is_empty() => text,
                Err(_) => status.canonical_reason().unwrap_or("unknown").to_owned(),
            },
            Err(_) => status.canonical_reason().unwrap_or("unknown").to_owned(),
        };

        Err(Error::Api {
            status: status.as_u16(),
            message,
        })
    }
}

/// [`Client::user_stream`] wrapped with reconnection: connection drops are
/// healed by minting a fresh ticket, while errors reconnecting cannot fix (a
/// revoked credential, an undecodable frame) are returned.
#[cfg(feature = "ws")]
#[derive(Debug)]
pub struct ReconnectingUserStream {
    client: Client,
    inner: crate::ws::EventStream,
}

#[cfg(feature = "ws")]
impl ReconnectingUserStream {
    const RECONNECT_PAUSE: Duration = Duration::from_secs(1);

    /// The next event; reconnects instead of ending when the connection
    /// closes or fails.
    pub async fn next_event(&mut self) -> Result<crate::ws::WsEvent> {
        loop {
            match self.inner.next_event().await {
                Some(Ok(event)) => return Ok(event),
                Some(Err(Error::Ws(_))) | None => {
                    tokio::time::sleep(Self::RECONNECT_PAUSE).await;
                    self.inner = self.client.user_stream().await?;
                }
                Some(Err(error)) => return Err(error),
            }
        }
    }
}

fn env_var(name: &'static str) -> Result<String> {
    std::env::var(name).map_err(|_| Error::MissingEnv(name))
}

fn now_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Clock)?;

    Ok(elapsed.as_millis() as i64)
}

fn trim_trailing_slash(mut url: String) -> String {
    while url.ends_with('/') {
        url.pop();
    }

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_debug_is_redacted() {
        let session = format!("{:?}", Credential::session("super-secret-token"));
        assert!(!session.contains("super-secret-token"));

        let api_key = format!("{:?}", Credential::api_key("key-id", "super-secret"));
        assert!(api_key.contains("key-id"));
        assert!(!api_key.contains("super-secret"));
    }

    #[test]
    fn builder_rejects_non_http_urls() {
        assert!(matches!(
            Client::builder("ftp://example.com").build(),
            Err(Error::InvalidUrl(_))
        ));
    }

    #[test]
    fn builder_trims_trailing_slashes() {
        let client = Client::builder("https://api.example.com/").build().unwrap();
        assert_eq!(client.base_url(), "https://api.example.com");
    }
}
