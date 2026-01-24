//! Core abstractions for building event-driven architectures.

pub mod consumer;
pub mod election;
pub mod inbox;
pub mod sequencer;
pub mod stream;
pub mod logic;

pub use election::Election;
pub use inbox::{Inbox, Sender};
pub use sequencer::Sequencer;
pub use stream::{Stream, Producer};

/// A receiver for consuming events from a stream or inbox.
///
/// This trait abstracts over different message reception mechanisms,
/// providing a unified interface for blocking receives with timeout.
pub trait Receiver {
    /// Receives the next event, blocking until data is available or the timeout expires.
    ///
    /// This method is not expected to return any error. The underlying implementation
    /// is responsible for maintaining the connection and continuing to receive data.
    /// Transient failures should be handled internally (e.g., via reconnection and retries).
    fn recv(&self) -> Vec<u8>;
}

