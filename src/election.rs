/// Leader election for sequencer redundancy.
///
/// In an event-driven architecture, multiple sequencers may exist for redundancy,
/// but only one can publish events to the stream at a time. This trait defines
/// the election mechanism that ensures exclusive leadership.
///
/// Semantics:
/// - Once a sequencer acquires leadership, no other sequencer can claim it.
/// - Leadership is lease-based with a timeout, allowing failover if the leader becomes unavailable.
/// - The leader must periodically renew its lease to retain leadership.
/// - A sequencer that loses leadership should terminate immediately.
///
/// Example Backends: Redis, TCP lock server.
pub trait Election: Sync {
    /// Attempts to acquire leadership.
    ///
    /// Returns `true` if this sequencer successfully became the leader.
    fn elect(&self) -> bool;

    /// Renews the leadership lease.
    ///
    /// Returns `true` if the lease was successfully renewed.
    fn renew(&self) -> bool;
}
