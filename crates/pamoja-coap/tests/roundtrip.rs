//! End-to-end transport test against an in-process CoAP server.
//!
//! A minimal UDP server built on `coap-lite` runs on an ephemeral port so the full
//! PUT and observe paths are exercised with no external infrastructure. The server
//! acknowledges a PUT, answers an observe registration with a piggybacked first
//! notification, and then pushes a follow-up notification.

use std::net::UdpSocket as StdUdpSocket;
use std::time::Duration;

use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType, ResponseType};
use pamoja_coap::{CoapConfig, CoapTransport};
use pamoja_core::Transport;
use tokio::net::UdpSocket;

/// Reserves an ephemeral UDP port for the server to listen on.
fn pick_port() -> u16 {
    StdUdpSocket::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port()
}

/// Copies the `Uri-Path` options from `request` onto `response`.
fn echo_path(request: &Packet, response: &mut Packet) {
    if let Some(segments) = request.get_option(CoapOption::UriPath) {
        for segment in segments {
            response.add_option(CoapOption::UriPath, segment.clone());
        }
    }
}

/// Starts the CoAP server on a background task and waits until it is listening.
async fn spawn_server(port: u16) {
    let socket = UdpSocket::bind(("127.0.0.1", port))
        .await
        .expect("bind server socket");

    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        loop {
            let (len, peer) = match socket.recv_from(&mut buf).await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let Ok(request) = Packet::from_bytes(&buf[..len]) else {
                continue;
            };

            match request.header.code {
                MessageClass::Request(RequestType::Put) => {
                    let mut ack = Packet::new();
                    ack.header.set_version(1);
                    ack.header.set_type(MessageType::Acknowledgement);
                    ack.header.code = MessageClass::Response(ResponseType::Changed);
                    ack.header.message_id = request.header.message_id;
                    ack.set_token(request.get_token().to_vec());
                    let bytes = ack.to_bytes().expect("encode put ack");
                    let _ = socket.send_to(&bytes, peer).await;
                }
                MessageClass::Request(RequestType::Get)
                    if request.get_option(CoapOption::Observe).is_some() =>
                {
                    // Piggybacked acknowledgment carrying the first notification.
                    let mut first = Packet::new();
                    first.header.set_version(1);
                    first.header.set_type(MessageType::Acknowledgement);
                    first.header.code = MessageClass::Response(ResponseType::Content);
                    first.header.message_id = request.header.message_id;
                    first.set_token(request.get_token().to_vec());
                    first.add_option(CoapOption::Observe, vec![1]);
                    echo_path(&request, &mut first);
                    first.payload = b"22.0".to_vec();
                    let bytes = first.to_bytes().expect("encode first notification");
                    let _ = socket.send_to(&bytes, peer).await;

                    // A separate non-confirmable follow-up notification.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let mut second = Packet::new();
                    second.header.set_version(1);
                    second.header.set_type(MessageType::NonConfirmable);
                    second.header.code = MessageClass::Response(ResponseType::Content);
                    second.header.message_id = request.header.message_id.wrapping_add(1);
                    second.set_token(request.get_token().to_vec());
                    second.add_option(CoapOption::Observe, vec![2]);
                    echo_path(&request, &mut second);
                    second.payload = b"22.5".to_vec();
                    let bytes = second.to_bytes().expect("encode second notification");
                    let _ = socket.send_to(&bytes, peer).await;
                }
                _ => {}
            }
        }
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_and_observe_round_trip() {
    let port = pick_port();
    spawn_server(port).await;

    let config = CoapConfig::new("127.0.0.1", port).ack_timeout(Duration::from_millis(500));
    let mut transport = CoapTransport::new(config);
    transport.connect().await.expect("connect");

    // A confirmable PUT must be acknowledged by the server.
    transport
        .send("actuators/valve", b"open")
        .await
        .expect("put acknowledged");

    // Registering an observe must be acknowledged, then deliver notifications.
    transport
        .subscribe("sensors/1/temperature")
        .await
        .expect("observe registered");

    let first = tokio::time::timeout(Duration::from_secs(2), transport.recv())
        .await
        .expect("first recv timed out")
        .expect("first recv returned an error")
        .expect("server closed before the first notification");
    assert_eq!(first.topic, "sensors/1/temperature");
    assert_eq!(first.payload, b"22.0");

    let second = tokio::time::timeout(Duration::from_secs(2), transport.recv())
        .await
        .expect("second recv timed out")
        .expect("second recv returned an error")
        .expect("server closed before the second notification");
    assert_eq!(second.topic, "sensors/1/temperature");
    assert_eq!(second.payload, b"22.5");
}
