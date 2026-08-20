//! Wire types for the Symbiosis API.
//!
//! Token amounts are [`U256`] and serialize as 0x-prefixed hex strings.
//! Prices are integers scaled by [`SCALE_FACTOR`]. Fee rates are basis
//! points over [`BPS_SCALE`].

use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use alloy_primitives::{Address, B256, U256};

/// Prices and token amounts are fixed-point with six decimals.
pub const SCALE_FACTOR: u64 = 1_000_000;

/// The denominator for fee rates expressed in basis points.
pub const BPS_SCALE: u64 = 10_000;

/// API key scope names accepted by [`CreateApiKeyRequest`].
pub mod scopes {
    pub const READ: &str = "read";
    pub const TRADE: &str = "trade";
    pub const WITHDRAW: &str = "withdraw";
}

macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

uuid_id!(
    /// A user account id.
    UserId
);
uuid_id!(
    /// An RFQ quote request id.
    RequestId
);
uuid_id!(
    /// An RFQ quote id.
    QuoteId
);
uuid_id!(
    /// An RFQ match id.
    MatchId
);

/// A market's asset id, a 32-byte value rendered as a 0x-prefixed hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(pub B256);

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<B256> for AssetId {
    fn from(b256: B256) -> Self {
        Self(b256)
    }
}

/// The supported markets of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue {
    Polymarket,
    Kalshi,
    Limitless,
}

impl Venue {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Venue::Polymarket => "polymarket",
            Venue::Kalshi => "kalshi",
            Venue::Limitless => "limitless",
        }
    }
}

impl Display for Venue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Polygon,
    Solana,
    Base,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    pub const fn opposite(&self) -> Self {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuoteStatus {
    Open,
    Matched,
    Expired,
}

// Auth

#[derive(Debug, Clone, Serialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignupResponse {
    pub user_id: UserId,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyRequest {
    pub label: String,
    /// See [`scopes`]. Password sessions cannot mint keys with the
    /// `withdraw` scope.
    pub scopes: Vec<String>,
    pub expires_in_secs: Option<u64>,
}

/// The only response that ever carries the raw secret.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyResponse {
    pub api_key_id: String,
    pub secret: String,
    pub label: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeySummary {
    pub api_key_id: String,
    pub label: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub user_id: UserId,
    pub event_type: String,
    pub created_at: DateTime<Utc>,
    pub api_key_id: Option<String>,
    pub ip_address: Option<String>,
}

/// A short-lived websocket ticket, passed as the `ticket` query parameter on
/// the upgrade request.
#[derive(Debug, Clone, Deserialize)]
pub struct WsTicketResponse {
    pub ticket: String,
}

// Custody

#[derive(Debug, Clone, Serialize)]
pub struct CreateDepositAddressRequest {
    pub chain: Chain,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepositAddress {
    pub chain: Chain,
    pub address: B256,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepositAddressRecord {
    pub chain: Chain,
    pub address: B256,
    pub nonce: i64,
    pub current: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetDepositAddressesResponse {
    pub addresses: Vec<DepositAddressRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceRequest {
    pub venue: Venue,
    pub asset_id: AssetId,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub asset_id: AssetId,
    pub balance: U256,
    pub pending_balance: U256,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsdcBalanceResponse {
    pub balance: U256,
    pub pending_balance: U256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "withdraw_asset_type", rename_all = "snake_case")]
pub enum WithdrawAsset {
    Asset {
        venue: Venue,
        asset_id: AssetId,
    },
    #[serde(rename = "u_s_d_c")]
    Usdc {
        chain: Chain,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct WithdrawRequest {
    pub recipient: Address,
    pub asset: WithdrawAsset,
    pub amount: U256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WithdrawStatus {
    Queued,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawResponse {
    pub status: WithdrawStatus,
}

// RFQ

#[derive(Debug, Clone, Serialize)]
pub struct RequestQuoteBody {
    pub asset_id: AssetId,
    pub venue_id: Venue,
    pub amount: U256,
    pub side: Side,
    pub disclose_identity: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestQuoteResponse {
    pub request_id: RequestId,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateQuoteBody {
    pub request_id: RequestId,
    /// Scaled by [`SCALE_FACTOR`].
    pub price: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OfferQuoteResponse {
    pub request_id: RequestId,
    pub offer_id: QuoteId,
}

/// A quote request as disclosed to the market. `user_id` is present only when
/// the requester opted into disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicQuoteRequest {
    pub request_id: RequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<UserId>,
    pub asset_id: AssetId,
    pub venue_id: Venue,
    pub amount: U256,
    pub side: Side,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub request_id: RequestId,
    pub quoter_id: UserId,
    pub requester_id: UserId,
    pub quote_id: QuoteId,
    /// Scaled by [`SCALE_FACTOR`].
    pub price: u64,
    pub status: QuoteStatus,
    /// The taker fee rate pinned when the quote was created, paid by the
    /// requester on the USDC leg at settlement.
    pub fee_bps: u16,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetRequestsResponse {
    pub requests: Vec<PublicQuoteRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetOffersResponse {
    pub offers: Vec<Quote>,
}

/// A venue and asset pair identifying a single market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookIdentifier {
    pub venue: Venue,
    pub asset_id: AssetId,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ErrorResponse {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venue_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Venue::Polymarket).unwrap(),
            "\"polymarket\""
        );
        assert_eq!(serde_json::to_string(&Chain::Base).unwrap(), "\"Base\"");
        assert_eq!(serde_json::to_string(&Side::Bid).unwrap(), "\"Bid\"");
    }

    #[test]
    fn withdraw_asset_tag_matches_wire_format() {
        let usdc = WithdrawAsset::Usdc {
            chain: Chain::Polygon,
        };
        assert_eq!(
            serde_json::to_string(&usdc).unwrap(),
            r#"{"withdraw_asset_type":"u_s_d_c","chain":"Polygon"}"#
        );

        let asset = WithdrawAsset::Asset {
            venue: Venue::Kalshi,
            asset_id: AssetId(B256::repeat_byte(1)),
        };
        let json = serde_json::to_value(asset).unwrap();
        assert_eq!(json["withdraw_asset_type"], "asset");
        assert_eq!(json["venue"], "kalshi");
    }

    #[test]
    fn u256_round_trips_as_hex() {
        let response: UsdcBalanceResponse =
            serde_json::from_str(r#"{"balance":"0xf4240","pending_balance":"0x0"}"#).unwrap();
        assert_eq!(response.balance, U256::from(1_000_000u64));
        assert_eq!(response.pending_balance, U256::ZERO);
    }

    #[test]
    fn public_quote_request_parses_without_user_id() {
        let json = r#"{
            "request_id": "b4b9ad60-90b1-4b3e-8a0a-52960e11e46a",
            "asset_id": "0x0101010101010101010101010101010101010101010101010101010101010101",
            "venue_id": "polymarket",
            "amount": "0x64",
            "side": "Ask"
        }"#;

        let request: PublicQuoteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_id, None);
        assert_eq!(request.amount, U256::from(100u64));
    }
}
