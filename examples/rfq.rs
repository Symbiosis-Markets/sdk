//! The whole taker flow in one call: request a quote, accept the first
//! offer to arrive, and await the match confirmation.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... cargo run --example rfq
//! ```

use std::time::Duration;

use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, RequestQuoteBody, Side, Venue, shares};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    let body = RequestQuoteBody::new(
        Venue::Polymarket,
        AssetId(B256::ZERO),
        Side::Bid,
        shares(100),
    );
    let rfq_match = client
        .request_and_accept_first(&body, Duration::from_secs(30))
        .await?;

    println!(
        "matched {} at {} (fee {} bps)",
        rfq_match.match_id, rfq_match.price, rfq_match.fee_bps
    );

    Ok(())
}
