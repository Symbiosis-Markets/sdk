//! Create a deposit address, check balances, and queue a USDC withdrawal.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... \
//! SYMBIOSIS_WITHDRAW_TO=0x... cargo run --example custody
//! ```

use std::env;

use symbiosis_sdk::types::{Address, Chain, SCALE_FACTOR, U256, WithdrawAsset, WithdrawRequest};
use symbiosis_sdk::{Client, Credential};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder("https://api.symbiosis.markets")
        .credential(Credential::api_key(
            env::var("SYMBIOSIS_API_KEY_ID")?,
            env::var("SYMBIOSIS_API_KEY_SECRET")?,
        ))
        .build()?;

    let deposit = client.create_deposit_address(Chain::Base).await?;
    println!("deposit USDC on {:?} at {}", deposit.chain, deposit.address);

    let usdc = client.get_usdc_balance().await?;
    println!("usdc: {} (pending {})", usdc.balance, usdc.pending_balance);

    // Withdraw 25 USDC; amounts are fixed-point with six decimals. Needs an
    // API key minted with the `withdraw` scope.
    let recipient: Address = env::var("SYMBIOSIS_WITHDRAW_TO")?.parse()?;
    let withdrawal = client
        .withdraw(&WithdrawRequest {
            recipient,
            asset: WithdrawAsset::Usdc { chain: Chain::Base },
            amount: U256::from(25) * U256::from(SCALE_FACTOR),
        })
        .await?;
    println!("withdrawal {:?}", withdrawal.status);

    Ok(())
}
