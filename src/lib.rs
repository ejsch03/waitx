//! Minimal synchronous concurrency primitives.
//!
//! This crate provides:
//! - A counted blocking notification primitive ([`Waker`]/[`Waiter`])
//! - A single-slot synchronous channel ([`Sender`]/[`Receiver`])
//!
//! # Example
//!
//! ```
//! use waitx::{channel, pair};
//!
//! // Single-slot channel
//! let (tx, rx) = channel();
//! std::thread::spawn(move || {
//!     tx.send(10);
//! });
//! assert_eq!(rx.recv(), 10);
//!
//! // Counted notification pair
//! let (waker, waiter) = pair();
//! std::thread::spawn(move || {
//!     waker.signal();
//! });
//! waiter.wait();
//! ```

mod atomic_wait;
mod prelude;
mod util;

#[cfg(feature = "loom")]
mod loom;

pub mod channel;
pub mod pair;

pub use channel::*;
pub use pair::*;
