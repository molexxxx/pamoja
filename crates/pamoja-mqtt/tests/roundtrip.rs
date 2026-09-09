//! End-to-end transport test against an in-process MQTT broker.
//!
//! An embedded `rumqttd` broker is started on an ephemeral port so the full
//! publish/subscribe path is exercised with no external infrastructure.

use std::collections::HashMap;
use std::net::TcpListener;
use std::time::Duration;

use pamoja_core::{Receive, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};

/// Reserves an ephemeral TCP port for the broker to listen on.
fn pick_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port()
}

/// Builds a minimal single-listener MQTT v4 broker configuration.
fn broker_config(port: u16) -> Config {
    let connections = ConnectionSettings {
        connection_timeout_ms: 5_000,
        max_payload_size: 20_480,
        max_inflight_count: 100,
        auth: None,
        external_auth: None,
        dynamic_filters: false,
    };
    let server = ServerSettings {
        name: "v4-1".to_owned(),
        listen: format!("127.0.0.1:{port}").parse().expect("listen addr"),
        tls: None,
        next_connection_delay_ms: 0,
        connections,
    };
    let router = RouterConfig {
        max_connections: 100,
        max_outgoing_packet_count: 200,
        max_segment_size: 104_857_600,
        max_segment_count: 10,
        ..Default::default()
    };
    let mut v4 = HashMap::new();
    v4.insert("v4-1".to_owned(), server);

    Config {
        id: 0,
        router,
        v4: Some(v4),
        ..Default::default()
    }
}

/// Starts the broker on a background OS thread and returns once it is listening.
fn spawn_broker(port: u16) {
    let config = broker_config(port);
    std::thread::spawn(move || {
        let mut broker = Broker::new(config);
        let _ = broker.start();
    });
}

/// Connects a transport, retrying until the broker accepts the connection.
async fn connect_with_retry(config: MqttConfig) -> MqttTransport {
    let mut last_error = None;
    for _ in 0..50 {
        let mut transport = MqttTransport::new(config.clone());
        match transport.connect().await {
            Ok(()) => return transport,
            Err(err) => {
                last_error = Some(err);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    panic!("could not connect to embedded broker: {last_error:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_and_subscribe_round_trip() {
    let port = pick_port();
    spawn_broker(port);

    let topic = "pamoja/it/round-trip";

    let mut subscriber = connect_with_retry(
        MqttConfig::new("ze-sub", "127.0.0.1", port).keep_alive(Duration::from_secs(5)),
    )
    .await;
    subscriber.subscribe(topic).await.expect("subscribe");
    // Let the broker register the subscription before publishing.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut publisher = connect_with_retry(
        MqttConfig::new("ze-pub", "127.0.0.1", port).keep_alive(Duration::from_secs(5)),
    )
    .await;
    publisher.send(topic, b"hello-edge").await.expect("publish");

    let received = tokio::time::timeout(Duration::from_secs(5), subscriber.recv())
        .await
        .expect("recv timed out")
        .expect("recv returned an error")
        .expect("event loop ended before a message arrived");

    assert_eq!(received.topic, topic);
    assert_eq!(received.payload, b"hello-edge");
}
