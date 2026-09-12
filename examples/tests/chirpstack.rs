//! Interop against a real ChirpStack network server.
//!
//! The `pamoja-gateway` tests prove the packet forwarder protocol against the vectors the
//! protocol publishes, and the network side against our own device code. Neither proves that
//! someone else reads what we write. This does: a real LoRaWAN network server, running in
//! Docker, is handed a real OTAA join and an encrypted uplink through the datagrams this crate
//! builds, and what it made of them is read back off its own MQTT integration.
//!
//! It is `#[ignore]`d because it needs that stack; `cargo xtask chirpstack` brings it up,
//! registers the gateway and the device, and runs this with the endpoints set:
//!
//! - `PAMOJA_CHIRPSTACK_UDP` is the gateway bridge, which speaks the Semtech protocol.
//! - `PAMOJA_CHIRPSTACK_MQTT` is the broker the network server publishes its events on.
//! - The identifiers and the key are the ones the stack was provisioned with.

use std::time::Duration;

use pamoja_core::{Receive, Transport};
use pamoja_gateway::udp::{Eui, Packet, Rxpk, Uplink};
use pamoja_lora::LinkSettings;
use pamoja_lorawan::{Device, Uplink as LorawanUplink};
use pamoja_mqtt::{MqttConfig, MqttTransport};
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// How long any single exchange may take. A cold stack is slow to answer the first join.
const BUDGET: Duration = Duration::from_secs(30);

/// An EU868 uplink channel, at the data rate a device joins on.
const FREQUENCY_HZ: u32 = 868_100_000;

/// What the device sends once it has joined.
const READING: &[u8] = b"21.5";

/// The port it sends on.
const PORT: u8 = 2;

fn setting(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must be set; run this with `cargo xtask chirpstack`"))
}

fn eight(hex: &str) -> [u8; 8] {
    let bytes = decode_hex(hex);
    bytes.try_into().expect("an identifier is eight bytes")
}

fn sixteen(hex: &str) -> [u8; 16] {
    let bytes = decode_hex(hex);
    bytes.try_into().expect("a key is sixteen bytes")
}

fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hexadecimal digits"))
        .collect()
}

/// Wraps a frame in the PUSH_DATA a gateway forwards, as if it had just been heard.
fn forwarded(gateway: Eui, token: u16, frame: &[u8], timestamp_us: u32) -> Vec<u8> {
    let link = LinkSettings::new(7, 125_000);
    let heard = Rxpk::new(FREQUENCY_HZ, link, frame.to_vec())
        .with_rssi_dbm(-35)
        .with_snr_db(5.1)
        .with_timestamp_us(timestamp_us);
    Packet::PushData {
        token,
        gateway,
        uplink: Uplink::from(heard),
    }
    .to_bytes()
}

/// Reads datagrams until the server sends one down to transmit, and returns its payload.
async fn downlink(socket: &UdpSocket) -> Vec<u8> {
    let mut buffer = [0u8; 2048];
    loop {
        let read = timeout(BUDGET, socket.recv(&mut buffer))
            .await
            .expect("the server answers within the budget")
            .expect("the socket stays open");
        let Ok(packet) = Packet::parse(&buffer[..read]) else {
            continue;
        };
        // The acknowledgments come back first; the packet to transmit is what matters here.
        if let Packet::PullResp { transmit, .. } = packet {
            return transmit.payload;
        }
    }
}

/// Waits for one of the network server's own events on a topic, and returns its body.
async fn event(mqtt: &mut MqttTransport, ending: &str) -> String {
    loop {
        let message = timeout(BUDGET, mqtt.recv())
            .await
            .expect("the server publishes within the budget")
            .expect("the broker stays connected")
            .expect("the broker does not end the stream");
        if message.topic.ends_with(ending) {
            return String::from_utf8(message.payload).expect("the event is JSON text");
        }
    }
}

#[tokio::test]
#[ignore = "needs a running ChirpStack stack; run via `cargo xtask chirpstack`"]
async fn a_real_network_server_accepts_a_join_and_an_uplink() {
    let gateway = Eui::from_hex(&setting("PAMOJA_CHIRPSTACK_GATEWAY_EUI"))
        .expect("sixteen hexadecimal digits");
    let dev_eui = eight(&setting("PAMOJA_CHIRPSTACK_DEV_EUI"));
    let join_eui = eight(&setting("PAMOJA_CHIRPSTACK_JOIN_EUI"));
    let app_key = sixteen(&setting("PAMOJA_CHIRPSTACK_APP_KEY"));
    let application = setting("PAMOJA_CHIRPSTACK_APPLICATION_ID");
    let broker = setting("PAMOJA_CHIRPSTACK_MQTT");
    let forwarder = setting("PAMOJA_CHIRPSTACK_UDP");

    // Listen for what the network server makes of what we send, before sending any of it.
    let (host, port) = broker.rsplit_once(':').expect("host:port");
    let mut mqtt = MqttTransport::new(MqttConfig::new(
        "pamoja-interop",
        host,
        port.parse().expect("a port"),
    ));
    mqtt.connect().await.expect("the broker accepts a client");
    mqtt.subscribe(&format!("application/{application}/device/+/event/+"))
        .await
        .expect("the subscription is accepted");

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("a local port to forward from");
    socket
        .connect(&forwarder)
        .await
        .expect("the gateway bridge is listening");

    // A gateway holds its route open before anything can be sent back down it.
    socket
        .send(
            &Packet::PullData {
                token: 0x0001,
                gateway,
            }
            .to_bytes(),
        )
        .await
        .expect("the route opens");

    // The device asks to join, and the gateway forwards the request.
    let device = Device::new(dev_eui, join_eui, app_key);
    let request = device.join_request(0x0102);
    socket
        .send(&forwarded(gateway, 0x0002, request.as_bytes(), 1_000_000))
        .await
        .expect("the join request is forwarded");

    let joined = event(&mut mqtt, "/event/join").await;
    println!("join      {joined}");
    assert!(
        joined.contains(&setting("PAMOJA_CHIRPSTACK_DEV_EUI")),
        "the join is for the device we registered"
    );

    // The server sends the accept back down for the gateway to transmit, and the device reads
    // it with the same key it signed the request with.
    let accept = downlink(&socket).await;
    let session = device
        .accept_join(&accept, 0x0102)
        .expect("the accept verifies against the key the server holds")
        .session();
    println!("accept    {} bytes, session granted", accept.len());

    // Now a reading, encrypted with the session that join produced.
    let frame = session
        .encode_uplink(&LorawanUplink::new(0, PORT, READING))
        .expect("it fits one frame");
    socket
        .send(&forwarded(gateway, 0x0003, frame.as_bytes(), 9_000_000))
        .await
        .expect("the uplink is forwarded");

    let uplink = event(&mut mqtt, "/event/up").await;
    println!("uplink    {uplink}");

    // The server publishes what it decrypted, base64 encoded. Reading it back with our own
    // base64 closes the loop: their bytes are our bytes.
    let encoded = pamoja_gateway::base64::encode(READING);
    assert!(
        uplink.contains(&encoded),
        "the reading arrives as the device sent it, as {encoded}"
    );
    assert!(
        uplink.contains(&format!("\"fPort\":{PORT}")),
        "on the port it was sent on"
    );

    mqtt.disconnect().await.expect("the client closes cleanly");
}
