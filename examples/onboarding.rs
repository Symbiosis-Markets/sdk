//! Sign up, log in, and mint a trading API key.
//!
//! ```sh
//! SYMBIOSIS_EMAIL=... SYMBIOSIS_PASSWORD=... cargo run --example onboarding
//! ```

use std::env;

use symbiosis_sdk::types::{CreateApiKeyRequest, Scope};
use symbiosis_sdk::{Client, Credential};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let email = env::var("SYMBIOSIS_EMAIL")?;
    let password = env::var("SYMBIOSIS_PASSWORD")?;

    let anon = Client::production().build()?;
    let signup = anon.signup(&email, &password).await?;
    println!("account created: {}", signup.user_id);

    let token = anon.login(&email, &password).await?.token;
    let client = anon.with_credential(Credential::session(token));

    let key = client
        .create_api_key(&CreateApiKeyRequest {
            label: "trading-bot".to_owned(),
            scopes: vec![Scope::Read, Scope::Trade],
            expires_in_secs: Some(30 * 24 * 60 * 60),
        })
        .await?;
    // The raw secret is returned exactly once; store it now.
    println!("minted key {}, secret: {}", key.api_key_id, key.secret);

    for key in client.list_api_keys().await? {
        println!(
            "{} [{}] scopes: {:?}",
            key.api_key_id, key.label, key.scopes
        );
    }

    Ok(())
}
