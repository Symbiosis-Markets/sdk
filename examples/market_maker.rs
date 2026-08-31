//! Stream the public RFQ flow for a market and quote every request at 50c.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... \
//! cargo run --example market_maker
//! ```

use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, BookIdentifier, Venue, price_cents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    let markets = [BookIdentifier {
        venue: Venue::Polymarket,
        asset_id: AssetId(B256::ZERO),
    }];
    let mut subscription = client.rfq_stream(&markets).await?;

    // Requests already open at subscribe time come first, then the live
    // flow — one loop covers both.
    while let Some(request) = subscription.next_request().await {
        let request = request?;
        let quote = client
            .create_quote(request.request_id, price_cents(50))
            .await?;
        println!("quoted {}: {}", request.request_id, quote.offer_id);
    }

    Ok(())
}
