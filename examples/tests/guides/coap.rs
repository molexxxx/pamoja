//! The CoAP guide example; see docs/guides/coap.md.

/// The two delivery modes a constrained node picks between: a reading fired and forgotten,
/// and a command that retransmits until it is acknowledged.
#[tokio::test]
async fn a_confirmable_request_is_reported_when_no_acknowledgement_arrives() {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
    use pamoja_core::Transport;

    // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
    // Nothing is listening on the far side here, and for a non-confirmable send nothing
    // needs to be.
    let mut reporter = CoapTransport::new(
        CoapConfig::new("127.0.0.1", 5683).reliability(Reliability::NonConfirmable),
    );
    reporter.connect().await.expect("a local socket");
    println!("reporter  connected: {}", reporter.is_connected());

    // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    // is what a battery-powered node sends when one missed reading costs nothing.
    reporter
        .send("sensors/1/temperature", b"21.5")
        .await
        .expect("the datagram leaves");
    println!("reporter  sent 21.5 and did not wait for an answer");

    // A command is different: it has to arrive. Confirmable delivery retransmits until an
    // acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and
    // four retransmissions; both are cut short here so the guide does not sit waiting.
    let mut commander = CoapTransport::new(
        CoapConfig::new("127.0.0.1", 5683)
            .reliability(Reliability::Confirmable)
            .ack_timeout(Duration::from_millis(20))
            .max_retransmits(1),
    );
    commander.connect().await.expect("a local socket");
    match commander.send("actuators/valve", b"open").await {
        Ok(()) => println!("commander the valve acknowledged the command"),
        Err(error) => println!("commander gave up unacknowledged: {error}"),
    }

    reporter.disconnect().await.expect("a clean close");
    println!("reporter  disconnected: {}", !reporter.is_connected());
    // ANCHOR_END: example

    assert!(!reporter.is_connected());
    assert!(commander.send("actuators/valve", b"open").await.is_err());
}
