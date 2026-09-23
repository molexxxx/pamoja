//! The own-link guide example; see docs/guides/link.md.
//!
//! Run: `cargo run -p pamoja-examples --example link`

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use pamoja_core::{Error, Message, Receive, Result, Transport};
use tokio::sync::Notify;

// ANCHOR: parts
/// What the modem vendor's SDK holds: what it was given to send, the filters it
/// listens on, what it has to hand over, and whether it has signal.
#[derive(Default)]
struct Sdk {
    sent: Vec<Message>,
    filters: Vec<String>,
    inbox: VecDeque<Result<Message>>,
    no_signal: bool,
}

/// A cellular modem reached through its vendor's SDK. Nothing in it names a broker
/// or a protocol: it needs only the operations the contract asks for, and
/// `Receive` is what makes it a link that delivers.
struct Modem {
    sdk: Arc<Mutex<Sdk>>,
    arrived: Arc<Notify>,
    connected: bool,
    ended: bool,
}

impl Transport for Modem {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        self.ended = false;
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut sdk = self.sdk.lock().expect("the SDK");
        if sdk.no_signal {
            return Err(Error::Transport("no signal".to_owned()));
        }
        sdk.sent.push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        self.sdk
            .lock()
            .expect("the SDK")
            .filters
            .push(topic.to_owned());
        Ok(())
    }
}

impl Receive for Modem {
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        if self.ended {
            return Ok(None);
        }
        loop {
            let next = self.sdk.lock().expect("the SDK").inbox.pop_front();
            if let Some(next) = next {
                self.ended = next.is_err();
                return next.map(Some);
            }
            self.arrived.notified().await;
        }
    }
}

/// A satellite messenger: it sends and never receives, so it implements
/// `Transport` alone and goes on a ladder as an uplink.
struct Satellite {
    sent: Arc<Mutex<Vec<Message>>>,
}

impl Transport for Satellite {
    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.sent
            .lock()
            .expect("the messenger")
            .push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, _topic: &str) -> Result<()> {
        Err(Error::Transport(
            "a satellite messenger only sends".to_owned(),
        ))
    }
}
// ANCHOR_END: parts

/// A moored buoy with two links pamoja has never heard of, a cellular modem and a
/// satellite messenger, composed on a ladder like shipped ones: readings go out,
/// commands come back, and each failure shows up where a node would see it.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: example
    use pamoja_ladder::TransportLadder;
    use pamoja_sync::MemoryStore;

    // The SDK's state is shared with the example, which plays the vendor's side: it
    // hands the modem a command, takes its signal away, and drops its session.
    let sdk = Arc::new(Mutex::new(Sdk::default()));
    let arrived = Arc::new(Notify::new());
    let beamed = Arc::new(Mutex::new(Vec::new()));
    let hand_over = |next: Result<Message>| {
        sdk.lock().expect("the SDK").inbox.push_back(next);
        arrived.notify_one();
    };
    let modem = Modem {
        sdk: Arc::clone(&sdk),
        arrived: Arc::clone(&arrived),
        connected: false,
        ended: false,
    };
    let satellite = Satellite {
        sent: Arc::clone(&beamed),
    };

    // The modem is the cheaper link and goes first. The messenger only sends, so it
    // goes on as an uplink, which a ladder never listens on or subscribes.
    let mut ladder = TransportLadder::new(MemoryStore::new())
        .rung(modem)
        .uplink(satellite);
    ladder.connect().await?;

    // A reading goes out over the first link that takes it.
    ladder.send_text("waves/height", "1.8").await?;
    let carried = sdk.lock().expect("the SDK").sent[0].clone();
    println!("modem     carried {} {}", carried.topic, carried.text()?);

    // A subscription reaches every link that listens, and only those: the messenger,
    // whose subscribe would refuse, is never asked.
    ladder.subscribe("commands/#").await?;
    let filter = sdk.lock().expect("the SDK").filters[0].clone();
    println!("modem     listens on {filter}, and the satellite was never asked");

    // What the modem hands over comes back through the ladder.
    hand_over(Ok(Message::new("commands/interval", b"600")));
    let command = ladder.recv().await?.expect("a command");
    println!("buoy      took {} {}", command.topic, command.text()?);

    // A link that refuses a send passes the reading down to the next link.
    sdk.lock().expect("the SDK").no_signal = true;
    ladder.send_text("waves/height", "2.4").await?;
    let relayed = beamed.lock().expect("the messenger")[0].clone();
    println!(
        "satellite carried {} {} while the modem had no signal",
        relayed.topic,
        relayed.text()?
    );

    // A link that fails while listening says why, once, and has ended after that.
    // With no other link listening, the ladder then has nothing to wait on.
    hand_over(Err(Error::Transport(
        "the modem lost its session".to_owned(),
    )));
    let lost = ladder.recv().await.expect_err("the session was lost");
    println!("buoy      lost the modem: {lost}");
    let idle = ladder.recv().await.expect_err("no link listens");
    println!("buoy      has no link left to listen on: {idle}");

    // Connecting again brings the modem back, and the ladder places its filter on it
    // again, so a link's subscribe runs once for every connect.
    ladder.connect().await?;
    let filters = sdk.lock().expect("the SDK").filters.clone();
    println!(
        "modem     reconnected, and the ladder placed {} on it again",
        filters[1]
    );
    hand_over(Ok(Message::new("commands/interval", b"900")));
    let later = ladder.recv().await?.expect("a command");
    println!("buoy      took {} {}", later.topic, later.text()?);
    // ANCHOR_END: example

    assert_eq!(carried, Message::new("waves/height", b"1.8"));
    assert_eq!(filters, ["commands/#", "commands/#"]);
    assert_eq!(command, Message::new("commands/interval", b"600"));
    assert_eq!(relayed, Message::new("waves/height", b"2.4"));
    assert_eq!(
        lost.to_string(),
        "transport error: the modem lost its session"
    );
    assert!(matches!(idle, Error::Closed));
    assert_eq!(later, Message::new("commands/interval", b"900"));

    Ok(())
}
