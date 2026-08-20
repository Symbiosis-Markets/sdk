//! Rust client for the Symbiosis REST and websocket APIs.
//!
//! The [`Client`] covers the full REST surface: accounts, API keys, custody,
//! and RFQ trading. With the `ws` feature (on by default) it also connects the
//! user and RFQ event streams.
//!
//! # Authentication
//!
//! Two credentials are supported:
//!
//! - [`Credential::session`]: a bearer token from [`Client::login`].
//! - [`Credential::api_key`]: an HMAC key pair from [`Client::create_api_key`].
//!   Every request is signed with HMAC-SHA256 over
//!   `timestamp_ms "\n" METHOD "\n" PATH_AND_QUERY "\n" BODY`.
//!
//! # Example
//!
//! ```no_run
//! use symbiosis_sdk::{Client, Credential, types::{RequestQuoteBody, Side, Venue}};
//!
//! # async fn run() -> Result<(), symbiosis_sdk::Error> {
//! let client = Client::builder("https://api.symbiosis.markets")
//!     .credential(Credential::api_key("key-id", "key-secret"))
//!     .build()?;
//!
//! let balance = client.get_usdc_balance().await?;
//! println!("balance: {}", balance.balance);
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
pub mod sign;
pub mod types;

#[cfg(feature = "ws")]
pub mod ws;

pub use client::{Client, ClientBuilder, Credential};
pub use error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;
