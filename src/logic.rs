/// Core event-handling logic for stream processing.
///
/// This trait defines the essential operations for processing events: loading
/// initial state and handling individual events. It separates the business logic
/// from the consumption loop, allowing the same logic to be used with different
/// stream implementations.
pub trait Logic {
    /// Initializes state and returns the starting offset.
    ///
    /// Implementations typically load a snapshot—a complete state of the system
    /// captured at a known offset—and use that offset to resume processing.
    /// This method may also perform any other initialization needed to set up
    /// ephemeral state.
    fn load(&mut self) -> u64;

    /// Handles a single event.
    ///
    /// Implementations should update internal state based on the event. An active
    /// processor may also evaluate state and publish new events to the stream.
    ///
    /// Returns `true` to continue processing, or `false` to stop.
    fn step(&mut self, event: &[u8]) -> bool;

    /// Returns `true` if the logic is caught up with the stream.
    ///
    /// On startup, the logic may lag behind the stream head. Implementations
    /// should update state but should not evaluate state and produce commands
    /// until caught up, as they may be processing stale data.
    ///
    /// The criteria for being caught up is determined by the implementation.
    /// Typical criteria would be to compare physical time against event timestamps.
    fn caught_up(&mut self) -> bool;
}