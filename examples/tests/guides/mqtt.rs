//! The MQTT guide example; see docs/guides/mqtt.md.

/// A client pointed at a broker that is not there: the delivery guarantees it carries are the
/// protocol's, and a refused connection is reported rather than leaving a client that looks
/// connected.
#[tokio::test]
async fn an_unreachable_broker_leaves_the_client_disconnected() {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_core::{Error, Transport};
    use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

    // MQTT numbers its three delivery guarantees 0, 1 and 2 on the wire.
    assert_eq!(QualityOfService::AtMostOnce as u8, 0);
    assert_eq!(QualityOfService::AtLeastOnce as u8, 1);
    assert_eq!(QualityOfService::ExactlyOnce as u8, 2);

    // Nothing listens on this port, so the broker is unreachable. Building the transport
    // touches nothing; only connecting does.
    let config = MqttConfig::new("guide-node", "127.0.0.1", 47811)
        .keep_alive(Duration::from_secs(1))
        .qos(QualityOfService::ExactlyOnce);
    let mut transport = MqttTransport::new(config);
    assert!(!transport.is_connected());

    // A refused connection surfaces as a transport error and leaves the transport as it was,
    // so the same object can be retried once the broker is back.
    let outcome = transport.connect().await;
    assert!(matches!(outcome, Err(Error::Transport(_))));
    assert!(!transport.is_connected());
    // ANCHOR_END: example
}
