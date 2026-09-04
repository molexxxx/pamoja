//! The CoAP guide example; see docs/guides/coap.md.

/// The two delivery modes RFC 7252 defines, over a socket with nothing on the far side:
/// a non-confirmable datagram leaves unacknowledged, and a confirmable one retransmits
/// and then reports that it was never acknowledged.
#[tokio::test]
async fn a_confirmable_request_is_reported_when_no_acknowledgement_arrives() {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
    use pamoja_core::Transport;

    // CoAP runs over UDP and opens no session, so connecting only binds a local socket.
    // Nothing is listening on the far side here, and nothing needs to be.
    let mut reporter = CoapTransport::new(
        CoapConfig::new("127.0.0.1", 5683).reliability(Reliability::NonConfirmable),
    );
    assert!(!reporter.is_connected());
    reporter.connect().await.unwrap();
    assert!(reporter.is_connected());

    // Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    // is what a battery-powered node sends when one missed reading costs nothing.
    reporter
        .send("sensors/1/temperature", b"21.5")
        .await
        .unwrap();

    // Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults
    // at a two-second wait and four retransmissions; both are cut short here.
    let mut commander = CoapTransport::new(
        CoapConfig::new("127.0.0.1", 5683)
            .reliability(Reliability::Confirmable)
            .ack_timeout(Duration::from_millis(20))
            .max_retransmits(1),
    );
    commander.connect().await.unwrap();
    assert!(commander.send("actuators/valve", b"open").await.is_err());

    reporter.disconnect().await.unwrap();
    assert!(!reporter.is_connected());
    // ANCHOR_END: example
}
