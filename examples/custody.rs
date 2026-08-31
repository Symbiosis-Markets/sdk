//! Create a deposit address, check balances, and queue a USDC withdrawal.
//!
//! ```sh
//! SYMBIOSIS_API_KEY_ID=... SYMBIOSIS_API_KEY_SECRET=... \
//! SYMBIOSIS_WITHDRAW_TO=0x... cargo run --example custody
//! ```

use std::env;

use symbiosis_sdk::Client;
use symbiosis_sdk::types::{Address, Chain, WithdrawAsset, WithdrawRequest, usdc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    let deposit = client.create_deposit_address(Chain::Base).await?;
    println!("deposit USDC on {:?} at {}", deposit.chain, deposit.address);

    let balance = client.get_usdc_balance().await?;
    println!(
        "usdc: {} (pending {})",
        balance.balance, balance.pending_balance
    );

    // Withdraw 25 USDC. Needs an API key minted with the `withdraw` scope.
    let recipient: Address = env::var("SYMBIOSIS_WITHDRAW_TO")?.parse()?;
    let withdrawal = client
        .withdraw(&WithdrawRequest {
            recipient,
            asset: WithdrawAsset::Usdc { chain: Chain::Base },
            amount: usdc(25),
        })
        .await?;
    println!("withdrawal {:?}", withdrawal.status);

    Ok(())
}
