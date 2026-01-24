use crate::Receiver;

/// Source of incoming events to be sequenced.
///
/// An inbox provides a connection to an external event source (e.g., a message queue)
/// and implements [`Receiver`] to yield events for the sequencer to process.
///
/// Unlike [`Stream`], an inbox does not require persistence or durability guarantees.
/// Instead, it prioritizes low latency—minimizing the roundtrip time between consumers
/// submitting events and the sequencer processing them. Events that fail to reach the
/// sequencer can simply be retried by the consumer.
///
/// Common inbox implementations include gRPC, ZeroMQ, and shared memory IPC.
pub trait Inbox: Receiver + Sync {
    /// Clears the inbox.
    fn clear(&self);
}

/// Client-side handle for submitting commands to a sequencer's inbox.
///
/// A sender connects to the same backend as the corresponding [`Inbox`] (e.g., gRPC,
/// ZeroMQ, shared memory). Like the inbox, senders do not provide durability guarantees.
pub trait Sender {
    /// Submits a command to the sequencer's inbox.
    ///
    /// Implementations must handle transient failures internally (e.g., via retries).
    /// The exact delivery guarantees (blocking, buffered, fire-and-forget) are
    /// determined by the implementation.
    fn send(&self, command: &[u8]);
}