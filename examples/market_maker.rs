//! Stream the public RFQ flow for a market and quote every request at 50c.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... \
//! cargo run --example market_maker
//! ```

use std::env;

use symbiosis_sdk::types::{AssetId, B256, BookIdentifier, SCALE_FACTOR, Venue};
use symbiosis_sdk::ws::WsEvent;
use symbiosis_sdk::{Client, Credential};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder("https://api.symbiosis.markets")
        .ws_url("wss://ws.symbiosis.markets")
        .credential(Credential::api_key(
            env::var("SYMBIOSIS_API_KEY_ID")?,
            env::var("SYMBIOSIS_API_KEY_SECRET")?,
        ))
        .build()?;

    let markets = [BookIdentifier {
        venue: Venue::Polymarket,
        asset_id: AssetId(B256::ZERO),
    }];
    let mut subscription = client.rfq_stream(&markets).await?;

    // The subscription opens with a snapshot: quote what is already open.
    let price = SCALE_FACTOR / 2;
    for market in &subscription.snapshot {
        for request in &market.requests {
            let quote = client.create_quote(request.request_id, price).await?;
            println!(
                "quoted open request {}: {}",
                request.request_id, quote.offer_id
            );
        }
    }

    // Then quote each new request as it arrives.
    while let Some(event) = subscription.events.next_event().await {
        if let WsEvent::RfqRequest(request) = event? {
            let quote = client.create_quote(request.request_id, price).await?;
            println!("quoted {}: {}", request.request_id, quote.offer_id);
        }
    }

    Ok(())
}
