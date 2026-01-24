# evcore

Core abstractions for building event-driven architectures in Rust.

## design

This library provides trait definitions for building systems around a persistent, durable event stream. Event streams serve two primary purposes: **distribution** and **replayability**.

**Distribution.** Any subscriber can reconstruct state by consuming events from the stream. Subscribers do not need to coordinate with a centralized state provider or acquire locks—they simply replay events using the same logic as the sequencer.

**Replayability.** All events are persisted. Given the same source code, it is always possible to reconstruct any historical state by replaying the stream from the beginning.

These properties make event-driven architectures well-suited for systems like exchanges, trading infrastructure, and other applications requiring deterministic state reconstruction.

## license

Apache-2.0 or MIT
