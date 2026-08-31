# symbiosis-sdk

Rust client for the Symbiosis REST and websocket APIs.

The wire types mirror the server's own definitions, so amounts are
`alloy_primitives::U256`, ids are typed newtypes, and enums are real enums.
The [OpenAPI spec](https://github.com/symbiosis-markets/backend/blob/dev/docs/openapi.yaml)
remains the canonical contract.

## Install

```toml
[dependencies]
symbiosis-sdk = { git = "https://github.com/symbiosis-markets/sdk" }
```

The `ws` feature (on by default) adds the websocket streams. REST-only:

```toml
symbiosis-sdk = { git = "https://github.com/symbiosis-markets/sdk", default-features = false }
```

## Authentication

Two credentials are supported. A session token comes from `login`:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use symbiosis_sdk::{Client, Credential};

let anon = Client::builder("https://api.symbiosis.markets").build()?;
let token = anon.login("you@example.com", "password").await?.token;

let client = Client::builder("https://api.symbiosis.markets")
    .credential(Credential::session(token))
    .build()?;
# Ok(())
# }
```

An API key signs every request with HMAC-SHA256 over
`timestamp_ms "\n" METHOD "\n" PATH_AND_QUERY "\n" BODY`. The client handles
this transparently:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
use symbiosis_sdk::{Client, Credential};

let client = Client::builder("https://api.symbiosis.markets")
    .credential(Credential::api_key("key-id", "key-secret"))
    .build()?;

let balance = client.get_usdc_balance().await?;
# Ok(())
# }
```

API keys are minted with `create_api_key` under a session credential; the
secret is returned exactly once.

## RFQ

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
# use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, RequestQuoteBody, Side, U256, Venue};

# let client = Client::builder("x").build()?;
let request = client
    .request_quote(&RequestQuoteBody {
        asset_id: AssetId(B256::ZERO),
        venue_id: Venue::Polymarket,
        amount: U256::from(100_000_000u64),
        side: Side::Bid,
        disclose_identity: false,
    })
    .await?;

let quotes = client.get_quotes(request.request_id).await?;
if let Some(quote) = quotes.offers.first() {
    client.accept_quote(quote.quote_id).await?;
}
# Ok(())
# }
```

Prices are integers scaled by 1e6 (`types::SCALE_FACTOR`), amounts are `U256`
in the same scale, and fees are basis points over `types::BPS_SCALE`.

## Websocket streams

With a `ws_url` configured, the client mints tickets and connects in one
call. The user stream carries quotes on your requests, RFQ matches, and
trade events; the RFQ stream carries the public request flow for subscribed
markets, opening with a snapshot of their open requests:

```rust,no_run
# async fn run() -> Result<(), symbiosis_sdk::Error> {
# use symbiosis_sdk::Client;
use symbiosis_sdk::types::{AssetId, B256, BookIdentifier, Venue};
use symbiosis_sdk::ws::WsEvent;

let client = Client::builder("https://api.symbiosis.markets")
    .ws_url("wss://ws.symbiosis.markets")
    .build()?;

let markets = [BookIdentifier {
    venue: Venue::Polymarket,
    asset_id: AssetId(B256::ZERO),
}];
let mut subscription = client.rfq_stream(&markets).await?;

while let Some(event) = subscription.events.next_event().await {
    if let WsEvent::RfqRequest(request) = event? {
        println!("new request: {}", request.request_id);
    }
}
# Ok(())
# }
```

Tickets are short-lived, so reconnect by calling the stream method again.

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
