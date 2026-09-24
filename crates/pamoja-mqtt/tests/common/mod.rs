//! The in-process broker the integration tests run against.

#![allow(dead_code)]

use std::collections::HashMap;
use std::net::TcpListener;
use std::time::Duration;

use pamoja_core::Transport;
use pamoja_mqtt::{MqttConfig, MqttTransport};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};

/// Reserves an ephemeral TCP port for a listener.
pub fn pick_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port()
}

/// Builds a single-listener MQTT v4 broker configuration, with the users it accepts.
fn broker_config(port: u16, users: Option<HashMap<String, String>>) -> Config {
    let connections = ConnectionSettings {
        connection_timeout_ms: 5_000,
        max_payload_size: 20_480,
        max_inflight_count: 100,
        auth: users,
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

/// Starts a broker that admits anyone, on a background OS thread.
pub fn spawn_broker(port: u16) {
    spawn(broker_config(port, None));
}

/// Starts a broker that admits only the given user.
pub fn spawn_broker_with_user(port: u16, username: &str, password: &str) {
    let users = HashMap::from([(username.to_owned(), password.to_owned())]);
    spawn(broker_config(port, Some(users)));
}

fn spawn(config: Config) {
    std::thread::spawn(move || {
        let mut broker = Broker::new(config);
        let _ = broker.start();
    });
}

/// A config for a client of the test broker listening on `port`.
pub fn client_of(port: u16, id: &str) -> MqttConfig {
    MqttConfig::new(id, "127.0.0.1", port).keep_alive(Duration::from_secs(5))
}

/// Connects a transport, retrying until the broker accepts the connection.
pub async fn connect_with_retry(config: MqttConfig) -> MqttTransport {
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

/// Waits until something listens on a port.
pub async fn listening(port: u16) {
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("nothing listens on {port}");
}
