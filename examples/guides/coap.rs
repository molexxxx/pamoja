//! The CoAP guide example; see docs/guides/coap.md.
//!
//! Run: `cargo run -p pamoja-examples --example coap`

use std::error::Error;

/// An orchard's gateway taking moisture readings from sensors in the rows, which observe
/// the irrigation valve it holds, over CoAP on this machine.
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_coap::{CoapConfig, CoapServer, CoapTransport, Reliability};
    use pamoja_core::{Receive, Transport};

    // The gateway in the orchard's shed. It takes moisture readings from every row, and
    // holds the irrigation valve's state for the rows to observe. Port 0 lets the system
    // pick a free port, which the rows are pointed at below.
    let mut gateway = CoapServer::new("127.0.0.1:0");
    gateway.connect().await?;
    gateway.subscribe("orchard/+/moisture").await?;
    gateway.send_text("orchard/valve", "closed").await?;
    let port = gateway.local_addr().expect("bound").port();
    println!("gateway   takes moisture readings on orchard/+/moisture");

    // A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
    // gateway's acknowledgment and retransmits until one comes back.
    let mut row7 = CoapTransport::new(
        CoapConfig::new("127.0.0.1", port).ack_timeout(Duration::from_millis(200)),
    );
    row7.connect().await?;
    row7.send_text("orchard/row-7/moisture", "31").await?;
    println!("row-7     reported 31, and the gateway acknowledged it");
    let reading = gateway.recv().await?.expect("a reading");
    println!("gateway   took {} from {}", reading.text()?, reading.topic);

    // Observing the valve registers the row with the gateway, which answers with the
    // valve's state now and notifies every change after it, as RFC 7641 describes.
    row7.subscribe("orchard/valve").await?;
    let current = row7.recv().await?.expect("the current state");
    println!(
        "row-7     observes {}, which reads {}",
        current.topic,
        current.text()?
    );
    gateway.send_text("orchard/valve", "open").await?;
    let observers = gateway.observers("orchard/valve");
    println!("gateway   opened the valve for {observers} observer");
    let change = row7.recv().await?.expect("the change");
    println!("row-7     {} now reads {}", change.topic, change.text()?);

    // A path the gateway does not take is answered 4.04, and a confirmable send reports
    // that rather than counting the reading as delivered.
    match row7.send_text("orchard/row-7/battery", "3.1").await {
        Ok(()) => println!("row-7     the battery reading was taken, which should never happen"),
        Err(error) => println!("row-7     battery refused: {error}"),
    }

    // Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio
    // time and suits a reading whose loss costs nothing.
    let mut row8 = CoapTransport::new(
        CoapConfig::new("127.0.0.1", port).reliability(Reliability::NonConfirmable),
    );
    row8.connect().await?;
    row8.send_text("orchard/row-8/moisture", "27").await?;
    println!("row-8     sent 27 without waiting for an answer");
    let unconfirmed = gateway.recv().await?.expect("a reading");
    println!(
        "gateway   took {} from {}",
        unconfirmed.text()?,
        unconfirmed.topic
    );

    // Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on
    // a doubling wait and then gives up. RFC 7252's defaults would take more than a minute
    // to get there, so this one waits 20 ms and retransmits once.
    let mut row9 = CoapTransport::new(
        CoapConfig::new("127.0.0.1", 1)
            .ack_timeout(Duration::from_millis(20))
            .max_retransmits(1),
    );
    row9.connect().await?;
    match row9.send_text("orchard/row-9/moisture", "29").await {
        Ok(()) => println!("row-9     an empty port acknowledged it, which should never happen"),
        Err(error) => println!("row-9     gave up unacknowledged: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(reading.topic, "orchard/row-7/moisture");
    assert_eq!(current.payload, b"closed");
    assert_eq!(change.payload, b"open");
    assert_eq!(observers, 1);
    assert_eq!(unconfirmed.topic, "orchard/row-8/moisture");
    assert!(row7
        .send_text("orchard/row-7/battery", "3.1")
        .await
        .is_err());
    assert!(row9
        .send_text("orchard/row-9/moisture", "29")
        .await
        .is_err());

    Ok(())
}
