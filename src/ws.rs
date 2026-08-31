//! The user and RFQ websocket streams.
//!
//! Connections authenticate with a short-lived ticket passed as the `ticket`
//! query parameter, minted by [`crate::Client::ws_ticket`]. The RFQ stream
//! opens with a handshake naming the subscribed markets and answers with a
//! snapshot of their open requests before any live events.
//!
//! Tickets are single-use and short-lived, so reconnecting means minting a
//! fresh ticket; [`crate::Client::user_stream`] and
//! [`crate::Client::rfq_stream`] do both in one call.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::types::{
    AssetId, BookIdentifier, MatchId, PublicQuoteRequest, Quote, QuoteId, RequestId, UserId, Venue,
};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
struct MarketHandshake<'a> {
    markets: &'a [BookIdentifier],
}

/// An RFQ match, delivered to both counterparties on the user stream.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RfqMatch {
    pub match_id: MatchId,
    pub request_id: RequestId,
    pub quote_id: QuoteId,
    pub requester_id: UserId,
    pub quoter_id: UserId,
    /// Scaled by [`crate::types::SCALE_FACTOR`].
    pub price: u64,
    pub fee_bps: u16,
}

/// The open requests in one subscribed market, sent as the first RFQ frame.
#[derive(Debug, Clone, Deserialize)]
pub struct RfqRequestSnapshot {
    pub venue_id: Venue,
    pub asset_id: AssetId,
    pub requests: Vec<PublicQuoteRequest>,
}

/// A single event frame.
///
/// Trade and orderbook events are passed through undecoded until those
/// endpoints stabilize.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event_kind", content = "event")]
#[non_exhaustive]
pub enum WsEvent {
    #[serde(rename = "RFQRequest")]
    RfqRequest(PublicQuoteRequest),
    #[serde(rename = "RFQOffer")]
    RfqOffer(Quote),
    #[serde(rename = "RFQMatch")]
    RfqMatch(RfqMatch),
    TradeMatch(serde_json::Value),
    PriceLevelUpdate(serde_json::Value),
}

/// A live event stream over a websocket connection.
pub struct EventStream {
    inner: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl EventStream {
    /// The next event, or `None` once the server closes the connection.
    /// Control frames are handled internally.
    pub async fn next_event(&mut self) -> Option<Result<WsEvent>> {
        loop {
            match self.inner.next().await? {
                Ok(Message::Text(text)) => {
                    return Some(serde_json::from_str(text.as_str()).map_err(Error::Decode));
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => continue,
                Err(error) => return Some(Err(error.into())),
            }
        }
    }

    pub async fn close(mut self) -> Result<()> {
        self.inner.close(None).await?;

        Ok(())
    }
}

impl std::fmt::Debug for EventStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

/// An RFQ subscription: the snapshot taken at subscribe time plus the live
/// stream of events from that point on. Events may overlap the snapshot;
/// sequencing them is the consumer's job.
#[derive(Debug)]
pub struct RfqSubscription {
    pub snapshot: Vec<RfqRequestSnapshot>,
    pub events: EventStream,
}

impl RfqSubscription {
    /// The next open request: the snapshot entries first, then live
    /// `RFQRequest` events, so one loop covers requests that were already
    /// open and requests that arrive later. Other event kinds are skipped;
    /// consume [`Self::events`] directly to see them. Drains
    /// [`Self::snapshot`] as it goes.
    pub async fn next_request(&mut self) -> Option<Result<PublicQuoteRequest>> {
        while let Some(market) = self.snapshot.first_mut() {
            if market.requests.is_empty() {
                self.snapshot.remove(0);
            } else {
                return Some(Ok(market.requests.remove(0)));
            }
        }

        loop {
            match self.events.next_event().await? {
                Ok(WsEvent::RfqRequest(request)) => return Some(Ok(request)),
                Ok(_) => continue,
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

/// Connect the user event stream at `{ws_url}/ws/user`.
pub async fn connect_user(ws_url: &str, ticket: &str) -> Result<EventStream> {
    let (inner, _) = connect_async(format!("{ws_url}/ws/user?ticket={ticket}")).await?;

    Ok(EventStream { inner })
}

/// Connect the RFQ stream at `{ws_url}/ws/rfq` and subscribe to `markets`.
pub async fn connect_rfq(
    ws_url: &str,
    ticket: &str,
    markets: &[BookIdentifier],
) -> Result<RfqSubscription> {
    let (mut inner, _) = connect_async(format!("{ws_url}/ws/rfq?ticket={ticket}")).await?;

    let handshake = serde_json::to_string(&MarketHandshake { markets }).map_err(Error::Encode)?;
    inner.send(Message::Text(handshake.into())).await?;

    let snapshot = loop {
        let frame = match inner.next().await {
            Some(frame) => frame?,
            None => return Err(Error::WsHandshake("connection closed".to_owned())),
        };

        match frame {
            Message::Text(text) => {
                // A rejected handshake answers with a plain text reason in
                // place of the snapshot array.
                match serde_json::from_str(text.as_str()) {
                    Ok(snapshot) => break snapshot,
                    Err(_) => return Err(Error::WsHandshake(text.as_str().to_owned())),
                }
            }
            Message::Close(_) => return Err(Error::WsHandshake("connection closed".to_owned())),
            _ => continue,
        }
    };

    Ok(RfqSubscription {
        snapshot,
        events: EventStream { inner },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_frames_decode_by_kind() {
        let request = r#"{
            "event_kind": "RFQRequest",
            "event": {
                "request_id": "b4b9ad60-90b1-4b3e-8a0a-52960e11e46a",
                "asset_id": "0x0101010101010101010101010101010101010101010101010101010101010101",
                "venue_id": "limitless",
                "amount": "0x64",
                "side": "Bid"
            }
        }"#;

        match serde_json::from_str(request).unwrap() {
            WsEvent::RfqRequest(event) => assert_eq!(event.venue_id, Venue::Limitless),
            other => panic!("wrong variant: {other:?}"),
        }

        let rfq_match = r#"{
            "event_kind": "RFQMatch",
            "event": {
                "match_id": "0d2f3a35-2b34-4bb5-a3e0-6f849d1cf37b",
                "request_id": "b4b9ad60-90b1-4b3e-8a0a-52960e11e46a",
                "quote_id": "e2b21c2c-2c4f-4b5e-9d7e-1baf6d2f38aa",
                "requester_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
                "quoter_id": "8f14e45f-ceea-4a7a-9c5d-64ba3f6ae6b1",
                "price": 500000,
                "fee_bps": 25
            }
        }"#;

        match serde_json::from_str(rfq_match).unwrap() {
            WsEvent::RfqMatch(event) => assert_eq!(event.fee_bps, 25),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn snapshot_frame_decodes() {
        let frame = r#"[{
            "venue_id": "polymarket",
            "asset_id": "0x0101010101010101010101010101010101010101010101010101010101010101",
            "requests": []
        }]"#;

        let snapshot: Vec<RfqRequestSnapshot> = serde_json::from_str(frame).unwrap();
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot[0].requests.is_empty());
    }
}
