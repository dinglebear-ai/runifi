//! UniFi Network API client and action dispatch primitives.

pub mod actions;
pub mod api;
pub mod capabilities;
pub mod http;

mod client;
mod config;
mod service;

pub use actions::{ActionDispatcher, ActionRequest};
pub use client::UnifiClient;
pub use config::UnifiConfig;
pub use service::UnifiService;
