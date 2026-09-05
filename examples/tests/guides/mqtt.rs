//! The MQTT guide example; see docs/guides/mqtt.md.

use std::collections::HashMap;
use std::net::TcpListener;
use std::time::Duration;

/// A gateway subscribing to a wildcard topic and a node publishing under it, over a real
/// broker, so the whole publish and subscribe path runs rather than being described.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reading_reaches_a_gateway_over_a_broker() {
    // A broker to talk to. In production this is the MQTT server on the site; here it is
    // an in-process one on a spare port, so the example needs nothing running.
    let port = spawn_broker();

    // ANCHOR: example
    use pamoja_core::Transport;
    use pamoja_mqtt::{MqttConfig, MqttTransport, QualityOfService};

    // The gateway takes every temperature on the site. A `+` stands for exactly one level,
    // so this matches every node's temperature and nothing deeper.
    let gateway_config = MqttConfig::new("site-gateway", "127.0.0.1", port)
        .keep_alive(Duration::from_secs(5))
        .qos(QualityOfService::AtLeastOnce);
    let mut gateway = connect(gateway_config).await;
    gateway
        .subscribe("sensors/+/temperature")
        .await
        .expect("the broker accepts the subscription");
    println!("gateway   subscribed to sensors/+/temperature");

    // A node publishes under that pattern. At-least-once means the broker acknowledges
    // the message, so a node knows its reading was taken rather than hoping.
    let node_config = MqttConfig::new("node-1", "127.0.0.1", port)
        .keep_alive(Duration::from_secs(5))
        .qos(QualityOfService::AtLeastOnce);
    let mut node = connect(node_config).await;
    node.send("sensors/1/temperature", b"21.5")
        .await
        .expect("the broker takes the reading");
    println!("node      published 21.5 to sensors/1/temperature");

    // The gateway receives it with the topic attached, which is how it knows which node
    // sent the reading without the payload having to repeat it.
    let received = gateway
        .recv()
        .await
        .expect("the link is up")
        .expect("a message arrives");
    let reading = String::from_utf8_lossy(&received.payload);
    let topic = &received.topic;
    println!("gateway   got {reading} on {topic}");

    // Disconnecting leaves the transport reusable, so a node that loses its link can
    // reconnect the same object when the broker comes back.
    node.disconnect().await.expect("a clean disconnect");
    let still_up = node.is_connected();
    println!("node      disconnected, still connected: {still_up}");

    // A broker that is not there is reported rather than leaving a client that looks
    // connected, so a retry loop has something to test. Nothing listens on port 1.
    let mut nowhere = MqttTransport::new(
        MqttConfig::new("node-2", "127.0.0.1", 1).qos(QualityOfService::ExactlyOnce),
    );
    match nowhere.connect().await {
        Ok(()) => {
            println!("an unreachable broker accepted a connection, which should never happen")
        }
        Err(error) => println!("unreachable broker refused: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(received.topic, "sensors/1/temperature");
    assert_eq!(received.payload, b"21.5");
    assert!(!node.is_connected());
    assert!(!nowhere.is_connected());
}

/// Connects to the embedded broker, retrying while it finishes starting up.
async fn connect(config: pamoja_mqtt::MqttConfig) -> pamoja_mqtt::MqttTransport {
    use pamoja_core::Transport;

    for _ in 0..50 {
        let mut transport = pamoja_mqtt::MqttTransport::new(config.clone());
        if transport.connect().await.is_ok() {
            return transport;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the embedded broker never accepted a connection");
}

/// Starts an in-process MQTT broker on a spare port and returns the port.
fn spawn_broker() -> u16 {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("a spare port")
        .local_addr()
        .expect("its address")
        .port();

    let server = rumqttd::ServerSettings {
        name: "v4-1".to_owned(),
        listen: format!("127.0.0.1:{port}")
            .parse()
            .expect("a listen address"),
        tls: None,
        next_connection_delay_ms: 0,
        connections: rumqttd::ConnectionSettings {
            connection_timeout_ms: 5_000,
            max_payload_size: 20_480,
            max_inflight_count: 100,
            auth: None,
            external_auth: None,
            dynamic_filters: false,
        },
    };
    let mut v4 = HashMap::new();
    v4.insert("v4-1".to_owned(), server);
    let config = rumqttd::Config {
        id: 0,
        router: rumqttd::RouterConfig {
            max_connections: 100,
            max_outgoing_packet_count: 200,
            max_segment_size: 104_857_600,
            max_segment_count: 10,
            ..Default::default()
        },
        v4: Some(v4),
        ..Default::default()
    };

    std::thread::spawn(move || {
        let _ = rumqttd::Broker::new(config).start();
    });
    port
}
