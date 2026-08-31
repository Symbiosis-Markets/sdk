# symbiosis-sdk

Rust client for the Symbiosis REST and websocket APIs.

## Install

```toml
[dependencies]
symbiosis-sdk = { git = "https://github.com/Symbiosis-Markets/sdk" }
```

The `ws` feature (on by default) adds the websocket streams. REST-only:

```toml
symbiosis-sdk = { git = "https://github.com/Symbiosis-Markets/sdk", default-features = false }
```

## Authentication

Two credentials are supported. A session token comes from `login`:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use symbiosis_sdk::{Client, Credential};

let anon = Client::production().build()?;
let token = anon.login("you@example.com", "password").await?.token;
let client = anon.with_credential(Credential::session(token));
# Ok(())
# }
```

An API key signs every request with HMAC-SHA256 over
`timestamp_ms "\n" METHOD "\n" PATH_AND_QUERY "\n" BODY`. The client handles
this transparently. `Client::from_env` builds an API-key client in one call,
reading `SYMBIOSIS_API_KEY_ID` and `SYMBIOSIS_API_KEY_SECRET` and defaulting
to the production endpoints (`SYMBIOSIS_API_URL`/`SYMBIOSIS_WS_URL`
override them):

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use symbiosis_sdk::{Client, Credential};

let client = Client::from_env()?;
// or explicitly:
let client = Client::production()
    .credential(Credential::api_key("key-id", "key-secret"))
    .build()?;

let balance = client.get_usdc_balance().await?;
# Ok(())
# }
```

API keys are minted with `create_api_key` under a session credential; the
secret is returned exactly once. `Client::builder` remains for pointing at
non-production deployments.

## RFQ

The whole taker flow is one call:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use std::time::Duration;

use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, RequestQuoteBody, Side, Venue, shares};

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
# Ok(())
# }
```

The granular methods — `request_quote`, `get_quotes`, `create_quote`,
`accept_quote`, `cancel_request` — cover the same flow piece by piece.

Prices are integers scaled by 1e6 (`types::SCALE_FACTOR`), amounts are `U256`
in the same scale, and fees are basis points over `types::BPS_SCALE`. The
helpers `types::usdc`, `types::shares`, and `types::price_cents` build
correctly scaled values from whole units.

## Websocket streams

With a `ws_url` configured, the client mints tickets and connects in one
call. The user stream carries quotes on your requests, RFQ matches, and
trade events; the RFQ stream carries the public request flow for subscribed
markets, opening with a snapshot of their open requests:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, BookIdentifier, Venue};

let client = Client::from_env()?;

let markets = [BookIdentifier {
    venue: Venue::Polymarket,
    asset_id: AssetId(B256::ZERO),
}];
let mut subscription = client.rfq_stream(&markets).await?;

// Snapshot requests first, then the live flow — one loop covers both.
while let Some(request) = subscription.next_request().await {
    println!("open request: {}", request?.request_id);
}
# Ok(())
# }
```

Tickets are short-lived and single-use. `user_stream_reconnecting` wraps the
user stream with automatic reconnection; for an RFQ stream, reconnect by
calling `rfq_stream` again (a fresh subscription re-delivers the snapshot).

## Examples

Runnable flows live in [examples/](examples/):

- [onboarding.rs](examples/onboarding.rs) — sign up, log in, and mint a
  trading API key.
- [custody.rs](examples/custody.rs) — deposit addresses, balances, and a
  USDC withdrawal.
- [rfq.rs](examples/rfq.rs) — the taker side end to end: request a quote,
  watch the user stream, accept the first offer.
- [market_maker.rs](examples/market_maker.rs) — the maker side: stream a
  market's RFQ flow and quote every request.
- [market_data.rs](examples/market_data.rs) — list open requests over plain
  REST with a read-only key.

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
