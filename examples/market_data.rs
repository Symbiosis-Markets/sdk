//! List the open RFQ requests on a venue over plain REST. Read-only: a key
//! with just the `read` scope is enough.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... \
//! cargo run --example market_data
//! ```

use std::env;

use symbiosis_sdk::types::{SCALE_FACTOR, U256, Venue};
use symbiosis_sdk::{Client, Credential};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder("https://api.symbiosis.markets")
        .credential(Credential::api_key(
            env::var("SYMBIOSIS_API_KEY_ID")?,
            env::var("SYMBIOSIS_API_KEY_SECRET")?,
        ))
        .build()?;

    // Both filters are optional; None-None lists every open request.
    let open = client
        .get_open_requests(Some(Venue::Polymarket), None)
        .await?;
    println!("{} open requests on polymarket", open.requests.len());

    let scale = U256::from(SCALE_FACTOR);
    for request in &open.requests {
        println!(
            "{} {:?} {} shares on {} ({})",
            request.request_id,
            request.side,
            request.amount / scale,
            request.asset_id,
            match request.user_id {
                Some(user_id) => format!("from {user_id}"),
                None => "anonymous".to_owned(),
            },
        );
    }

    // The user's own requests, across every market.
    let mine = client.get_user_requests().await?;
    println!("{} open requests of our own", mine.requests.len());

    Ok(())
}
