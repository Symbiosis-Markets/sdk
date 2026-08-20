//! Request a quote, watch for offers on the user stream, and accept the
//! first one.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... cargo run --example rfq
//! ```

use std::env;

use symbiosis_sdk::types::{AssetId, B256, RequestQuoteBody, Side, U256, Venue};
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

    let mut stream = client.user_stream().await?;

    let request = client
        .request_quote(&RequestQuoteBody {
            asset_id: AssetId(B256::ZERO),
            venue_id: Venue::Polymarket,
            amount: U256::from(100) * U256::from(1_000_000u64),
            side: Side::Bid,
            disclose_identity: false,
        })
        .await?;
    println!("requested: {}", request.request_id);

    while let Some(event) = stream.next_event().await {
        if let WsEvent::RfqOffer(quote) = event? {
            if quote.request_id == request.request_id {
                println!("accepting {} at {}", quote.quote_id, quote.price);
                client.accept_quote(quote.quote_id).await?;
                break;
            }
        }
    }

    Ok(())
}
