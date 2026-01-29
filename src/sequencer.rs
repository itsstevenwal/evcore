//! Sequencer implementation for event-driven command processing.
//!
//! A sequencer is the active component that receives commands from an inbox,
//! processes them into events, and publishes those events to the stream. Only
//! one sequencer can be active at a time, enforced through leader election.

use crate::{
    consumer,
    election::Election,
    inbox::Inbox,
    logic::Logic,
    stream::{Producer, Stream},
};

use std::{
    process,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

const STATUS_STARTING: usize = 0;
const STATUS_CAUGHT_UP: usize = 1;
const STATUS_LEADER: usize = 2;
const STATUS_ACTIVATED: usize = 3;

/// A function that produces an event for the sequencer.
pub trait EventGenerator: Fn() -> Vec<u8> + Send + Sync {}

// Blanket impl: any closure or fn matching the signature automatically implements EventMaker.
impl<T: Fn() -> Vec<u8> + Send + Sync> EventGenerator for T {}

/// Logic for processing commands into events.
///
/// A [`Sequencer`] extends [`Logic`] with the ability to receive commands,
/// transform them into events, and publish those events to the stream. It
/// also handles leadership transitions through activation events.
///
/// The sequencer lifecycle:
/// 1. Start consuming the stream to rebuild state
/// 2. Once caught up, attempt to acquire leadership via election
/// 3. Upon becoming leader, publish an activation event
/// 4. Begin processing commands from the inbox
pub trait Sequencer: Logic {
    /// Processes a command and optionally produces an event.
    ///
    /// Typically validates the command against current state and, if valid,
    /// stamps it with a sequence number to produce an event. Returns `None`
    /// to reject the command.
    fn process(&self, command: &[u8]) -> Option<Vec<u8>>;

    /// Returns the activator function for this sequencer.
    ///
    /// The activator produces an activation event that signals leadership
    /// transition in the stream.
    fn activator(&self) -> Box<dyn EventGenerator>;

    /// Returns the heartbeat function for this sequencer.
    ///
    /// The heartbeat function is used to keep the sequencer alive by publishing
    /// a heartbeat event to the stream.
    fn heartbeat(&self) -> Box<dyn EventGenerator>;

    /// Returns `true` if the event is this sequencer's own activation event.
    ///
    /// Used to detect when the activation event published by this sequencer
    /// has been committed to the stream, signaling it can begin processing.
    fn is_activation(&self, event: &[u8]) -> bool;
}

struct Wrapper<'a, S> {
    status: &'a AtomicUsize,
    logic: &'a mut S,
}

impl<'a, S: Sequencer> Wrapper<'a, S> {
    fn check_caught_up(&mut self) {
        if self.logic.caught_up() {
            self.status.store(STATUS_CAUGHT_UP, Ordering::Relaxed);
        }
    }
}

impl<S: Sequencer> Logic for Wrapper<'_, S> {
    fn load(&mut self) -> u64 {
        let offset = self.logic.load();
        self.check_caught_up();

        offset
    }

    fn step(&mut self, event: &[u8]) -> bool {
        self.check_caught_up();

        let cont = self.logic.step(event);
        if self.logic.is_activation(event) {
            self.status.store(STATUS_ACTIVATED, Ordering::Relaxed);
            return false;
        }

        cont
    }

    fn caught_up(&mut self) -> bool {
        self.logic.caught_up()
    }
}

/// Runs the sequencer loop.
///
/// Spawns a background thread to manage election and activation, while the
/// main thread handles stream consumption and command processing. If the
/// sequencer fails to renew its leadership lease, it terminates immediately
/// to prevent split-brain scenarios.
pub fn run<S, P, I, E, L>(
    stream: &S,
    producer: &P,
    inbox: &I,
    election: &E,
    mut logic: L,
    interval: Duration,
) where
    S: Stream,
    P: Producer,
    I: Inbox,
    E: Election,
    L: Sequencer,
{
    let status = AtomicUsize::new(STATUS_STARTING);
    let activate = logic.activator();
    let heartbeat = logic.heartbeat();

    thread::scope(|s| {
        s.spawn(|| {
            loop {
                match status.load(Ordering::Relaxed) {
                    // Phase 1: Consume stream to rebuild state. Clear inbox since
                    // commands received before leadership should be discarded.
                    STATUS_STARTING => inbox.clear(),

                    // Phase 2: Caught up with stream. Attempt to acquire leadership.
                    STATUS_CAUGHT_UP => {
                        if election.elect() {
                            status.store(STATUS_LEADER, Ordering::Relaxed);
                        }
                    }

                    // Phase 3: Won election. Repeatedly publish activation until it
                    // lands at the stream tip, ensuring no events are overwritten.
                    STATUS_LEADER => {
                        if !election.renew() {
                            process::exit(1);
                        }
                        producer.publish(&activate());
                    }

                    // Phase 4: Activation observed. Continue renewing lease.
                    STATUS_ACTIVATED => {
                        if !election.renew() {
                            process::exit(1);
                        }
                        producer.publish(&heartbeat());
                    }

                    _ => unreachable!(),
                }
                thread::sleep(interval);
            }
        });

        // Consume stream until activation event is observed
        let mut wrapper = Wrapper {
            status: &status,
            logic: &mut logic,
        };
        consumer::run(stream, &mut wrapper);

        // Phase 4 (continued): Process commands from inbox
        loop {
            let command = inbox.recv();
            if let Some(event) = wrapper.logic.process(&command) {
                producer.publish(&event);
                wrapper.logic.step(&event);
            }
        }
    });
}
