use crate::Receiver;

/// The underlying storage abstraction for an event stream.
///
/// A [`Stream`] must guarantee persistence and durability. While many backends can
/// satisfy these requirements, the stream's latency and throughput characteristics
/// directly determine the system's overall performance. Choose a performant
/// implementation accordingly.
///
/// Common stream backends include Kafka, NATS JetStream, and Aeron.
pub trait Stream {
    type Receiver: Receiver;

    /// Subscribes to the stream starting at the given offset.
    ///
    /// Note that an offset is distinct from a sequence number. While the system's
    /// logic operates on a monotonically increasing sequence of events, the stream
    /// backend tracks its own internal offsets. Since streams typically do not
    /// deduplicate, the same sequence number may appear at multiple offsets.
    ///
    /// Returns a [`Receiver`] for consuming events. The receiver guarantees ordered
    /// delivery of all events from the stream, but does not deduplicate.
    fn subscribe(&self, offset: u64) -> Self::Receiver;
}

/// Provides the ability to publish events to a stream.
pub trait Producer: Sync {
    /// Publishes data to the stream.
    ///
    /// This method blocks until the data is guaranteed to be persisted and durable.
    /// The exact durability semantics depend on the backend—for example, Kafka
    /// provides this guarantee by waiting for an acknowledgment.
    ///
    /// This method does not return an error and is expected to always succeed.
    /// Implementations must handle transient failures internally (e.g., via retries).
    /// If persistence becomes impossible, the implementation must panic or abort to 
    /// prevent the system from continuing without its durability guarantees.
    fn publish(&self, data: &[u8]);
}

