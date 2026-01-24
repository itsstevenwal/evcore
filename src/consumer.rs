use crate::{logic::Logic, Receiver, stream::Stream};

/// Runs the consumer loop, reading events from the given stream.
///
/// This method subscribes to the stream at the offset returned by [`load`],
/// then repeatedly calls [`recv`] on the receiver, passing each event to
/// [`update`].
///
/// The loop continues until [`update`] returns `false`.
pub fn run<S, L>(stream: &S, logic: &mut L)
where
    S: Stream,
    L: Logic,
{
    let offset = logic.load();
    let receiver = stream.subscribe(offset);

    loop {
        let event = receiver.recv();
        if !logic.step(&event) {
            break;
        }
    }
}