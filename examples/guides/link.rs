//! The own-link guide example; see docs/guides/link.md.
//!
//! Run: `cargo run -p pamoja-examples --example link`

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use pamoja_core::{Error, Message, Receive, Result, Transport};
use tokio::sync::Notify;

// ANCHOR: parts
/// The vendor's side of a link: what it was given to send, what it was told to
/// listen for, and what it has to deliver.
#[derive(Default)]
struct Vendor {
    sent: Vec<Message>,
    filters: Vec<String>,
    inbox: VecDeque<Message>,
}

/// A link over the vendor's queues, standing in for a radio or cloud SDK. Nothing
/// about it names a broker: it needs only the operations the contract asks for.
struct QueueLink {
    vendor: Arc<Mutex<Vendor>>,
    delivered: Arc<Notify>,
    connected: bool,
}

impl Transport for QueueLink {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut vendor = self.vendor.lock().expect("the vendor");
        vendor.sent.push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut vendor = self.vendor.lock().expect("the vendor");
        vendor.filters.push(topic.to_owned());
        Ok(())
    }
}

impl Receive for QueueLink {
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        loop {
            if let Some(message) = self.vendor.lock().expect("the vendor").inbox.pop_front() {
                return Ok(Some(message));
            }
            self.delivered.notified().await;
        }
    }
}
// ANCHOR_END: parts

/// A link of the maker's own, composed like a shipped one: a reading out through a
/// ladder reaches it, a subscription placed on the ladder reaches it, and what it
/// delivers comes back through the ladder.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: example
    use pamoja_ladder::TransportLadder;
    use pamoja_sync::MemoryStore;

    // The vendor side is shared with the link so the example can watch it, the way a
    // real SDK exposes its own handles.
    let vendor = Arc::new(Mutex::new(Vendor::default()));
    let delivered = Arc::new(Notify::new());
    let link = QueueLink {
        vendor: Arc::clone(&vendor),
        delivered: Arc::clone(&delivered),
        connected: false,
    };

    // The link is a rung like any shipped transport, and the ladder is the link a
    // node is written against.
    let mut ladder = TransportLadder::new(MemoryStore::new()).rung(link);
    ladder.connect().await?;

    // A reading out through the ladder lands in the link, topic and bytes intact.
    ladder.send_text("sensors/1", "21.5").await?;
    let carried = vendor.lock().expect("the vendor").sent[0].clone();
    let reading = carried.text().expect("text");
    println!("link carried: {} {reading}", carried.topic);

    // A subscription placed on the ladder reaches the link.
    ladder.subscribe("commands/#").await?;
    let filter = vendor.lock().expect("the vendor").filters[0].clone();
    println!("link subscribed to: {filter}");

    // What the link delivers comes back through the ladder.
    vendor
        .lock()
        .expect("the vendor")
        .inbox
        .push_back(Message::new("commands/1", b"open"));
    delivered.notify_one();
    let command = ladder.recv().await?.expect("a command");
    let order = command.text().expect("text");
    println!("command over the ladder: {} {order}", command.topic);
    // ANCHOR_END: example

    assert_eq!(carried, Message::new("sensors/1", b"21.5"));
    assert_eq!(filter, "commands/#");
    assert_eq!(command, Message::new("commands/1", b"open"));

    Ok(())
}
