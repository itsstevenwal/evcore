//! A simple counter example demonstrating evcore's event-driven architecture.
//!
//! This example implements a basic counter that can be incremented or decremented
//! via commands. It uses in-memory mock implementations for all backends.
//!
//! Run with: `cargo run --example counter`

use evcore::logic::Logic;
use evcore::sequencer::EventGenerator;
use evcore::{Election, Inbox, Producer, Receiver, Sender, Sequencer, Stream};

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

/// Shared subscriber registry for broadcast semantics.
type Subscribers = Arc<Mutex<Vec<mpsc::Sender<Vec<u8>>>>>;

/// In-memory event stream with broadcast support.
///
/// Multiple consumers can subscribe and each will receive all published events.
struct MemoryStream {
    subscribers: Subscribers,
}

impl MemoryStream {
    fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn producer(&self) -> MemoryProducer {
        MemoryProducer {
            subscribers: Arc::clone(&self.subscribers),
        }
    }
}

struct MemoryStreamReceiver {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl Receiver for MemoryStreamReceiver {
    fn recv(&self) -> Vec<u8> {
        self.rx.recv().unwrap()
    }
}

impl Stream for MemoryStream {
    type Receiver = MemoryStreamReceiver;

    fn subscribe(&self, _offset: u64) -> Self::Receiver {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        MemoryStreamReceiver { rx }
    }
}

/// In-memory producer that broadcasts to all subscribers.
struct MemoryProducer {
    subscribers: Subscribers,
}

impl Producer for MemoryProducer {
    fn publish(&self, data: &[u8]) {
        println!(
            "[stream] published event: {:?}",
            String::from_utf8_lossy(data)
        );
        let mut subs = self.subscribers.lock().unwrap();
        // Broadcast to all subscribers, removing any disconnected ones
        subs.retain(|tx| tx.send(data.to_vec()).is_ok());
    }
}

/// In-memory inbox for demonstration purposes.
struct MemoryInbox {
    rx: Mutex<mpsc::Receiver<Vec<u8>>>,
}

impl Receiver for MemoryInbox {
    fn recv(&self) -> Vec<u8> {
        self.rx.lock().unwrap().recv().unwrap()
    }
}

impl Inbox for MemoryInbox {
    fn clear(&self) {
        while self.rx.lock().unwrap().try_recv().is_ok() {}
    }
}

/// Client-side sender for the memory inbox.
struct MemorySender {
    tx: mpsc::Sender<Vec<u8>>,
}

impl Sender for MemorySender {
    fn send(&self, command: &[u8]) {
        println!(
            "[client] sent command: {:?}",
            String::from_utf8_lossy(command)
        );
        self.tx.send(command.to_vec()).unwrap();
    }
}

/// Always-wins election for single-node demonstration.
struct AlwaysLeader;

impl Election for AlwaysLeader {
    fn elect(&self) -> bool {
        println!("[election] acquired leadership");
        true
    }

    fn renew(&self) -> bool {
        true
    }
}

/// Commands that can be sent to the counter.
#[derive(Debug)]
enum Command {
    Increment,
    Decrement,
}

impl Command {
    fn parse(data: &[u8]) -> Option<Self> {
        match data {
            b"inc" => Some(Command::Increment),
            b"dec" => Some(Command::Decrement),
            _ => None,
        }
    }
}

/// Events produced by the counter.
#[derive(Debug)]
enum Event {
    Activation,
    Heartbeat,
    Incremented { new_value: i64 },
    Decremented { new_value: i64 },
}

impl Event {
    fn serialize(&self) -> Vec<u8> {
        match self {
            Event::Heartbeat => b"heartbeat".to_vec(),
            Event::Activation => b"activation".to_vec(),
            Event::Incremented { new_value } => format!("inc:{}", new_value).into_bytes(),
            Event::Decremented { new_value } => format!("dec:{}", new_value).into_bytes(),
        }
    }

    fn parse(data: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(data).ok()?;
        if s == "heartbeat" {
            return Some(Event::Heartbeat);
        }
        if s == "activation" {
            return Some(Event::Activation);
        }
        if let Some(val) = s.strip_prefix("inc:") {
            return Some(Event::Incremented {
                new_value: val.parse().ok()?,
            });
        }
        if let Some(val) = s.strip_prefix("dec:") {
            return Some(Event::Decremented {
                new_value: val.parse().ok()?,
            });
        }
        None
    }
}

/// Counter logic shared between sequencer and consumer.
struct CounterLogic {
    value: i64,
    caught_up: bool,
    label: &'static str,
}

impl CounterLogic {
    fn new(label: &'static str) -> Self {
        Self {
            value: 0,
            caught_up: false,
            label,
        }
    }
}

impl Logic for CounterLogic {
    fn load(&mut self) -> u64 {
        println!("[{}] loading state, starting from offset 0", self.label);
        // In a real implementation, this would load from a snapshot
        self.caught_up = true; // For demo, we're immediately caught up
        0
    }

    fn step(&mut self, event: &[u8]) -> bool {
        if let Some(parsed) = Event::parse(event) {
            match parsed {
                Event::Heartbeat => {
                    println!("[{}] observed heartbeat event", self.label);
                }
                Event::Activation => {
                    println!("[{}] observed activation event", self.label);
                }
                Event::Incremented { new_value } => {
                    self.value = new_value;
                    println!("[{}] counter incremented to {}", self.label, self.value);
                }
                Event::Decremented { new_value } => {
                    self.value = new_value;
                    println!("[{}] counter decremented to {}", self.label, self.value);
                }
            }
        }
        true
    }

    fn caught_up(&mut self) -> bool {
        self.caught_up
    }
}

impl Sequencer for CounterLogic {
    fn process(&self, command: &[u8]) -> Option<Vec<u8>> {
        let cmd = Command::parse(command)?;
        let event = match cmd {
            Command::Increment => Event::Incremented {
                new_value: self.value + 1,
            },
            Command::Decrement => Event::Decremented {
                new_value: self.value - 1,
            },
        };
        println!(
            "[{}] processing {:?} -> {:?}, state: counter = {}",
            self.label, cmd, event, self.value
        );
        Some(event.serialize())
    }

    fn activator(&self) -> Box<dyn EventGenerator> {
        Box::new(|| Event::Activation.serialize())
    }

    fn heartbeat(&self) -> Box<dyn EventGenerator> {
        Box::new(|| Event::Heartbeat.serialize())
    }

    fn is_activation(&self, event: &[u8]) -> bool {
        matches!(Event::parse(event), Some(Event::Activation))
    }
}

fn main() {
    println!("evcore counter example");
    println!("======================");

    // Create channel for inbox
    let (inbox_tx, inbox_rx) = mpsc::channel();

    // Create broadcast stream and producer
    let stream = MemoryStream::new();
    let producer = stream.producer();
    let inbox = MemoryInbox {
        rx: Mutex::new(inbox_rx),
    };
    let sender = MemorySender { tx: inbox_tx };
    let election = AlwaysLeader;

    thread::scope(|s| {
        // Spawn a consumer thread using evcore::consumer::run
        s.spawn(|| {
            let mut consumer = CounterLogic::new("consumer");
            evcore::consumer::run(&stream, &mut consumer);
        });

        // Spawn a client thread that perpetually sends commands
        s.spawn(|| {
            use std::time::SystemTime;

            thread::sleep(Duration::from_millis(500)); // Wait for sequencer to start

            println!("[client] starting perpetual command loop...");
            loop {
                // Simple randomization using current time
                let nanos = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                if nanos % 2 == 0 {
                    sender.send(b"inc");
                } else {
                    sender.send(b"dec");
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        // Run the sequencer (this blocks forever in a real application)
        println!("[main] starting sequencer...");
        let sequencer = CounterLogic::new("sequencer");
        evcore::sequencer::run(
            &stream,
            &producer,
            &inbox,
            &election,
            sequencer,
            Duration::from_millis(100),
        );
    });
}
