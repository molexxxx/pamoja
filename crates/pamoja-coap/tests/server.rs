//! The server against the client, and against raw datagrams for the parts of RFC
//! 7252 and RFC 7641 a client never exercises.

use std::net::SocketAddr;
use std::time::Duration;

use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType, ResponseType};
use pamoja_coap::{CoapConfig, CoapServer, CoapTransport, Reliability};
use pamoja_core::{Error, Receive, Transport};
use tokio::net::UdpSocket;

/// A server on a free port, taking readings under `sensors/`.
async fn gateway() -> (CoapServer, u16) {
    let mut server = CoapServer::new("127.0.0.1:0");
    server.connect().await.expect("bind");
    server.subscribe("sensors/#").await.expect("filter");
    let port = server.local_addr().expect("bound").port();
    (server, port)
}

/// A connected client of the server on `port`.
async fn node(port: u16, reliability: Reliability) -> CoapTransport {
    let mut client = CoapTransport::new(
        CoapConfig::new("127.0.0.1", port)
            .reliability(reliability)
            .ack_timeout(Duration::from_millis(200)),
    );
    client.connect().await.expect("connect");
    client
}

/// Waits a short while for the next message on a link.
async fn next(link: &mut impl Receive) -> Option<pamoja_core::Message> {
    tokio::time::timeout(Duration::from_secs(2), link.recv())
        .await
        .ok()?
        .expect("no error")
}

/// A request datagram with a path, a message id, and a token.
fn request(kind: MessageType, method: RequestType, path: &str, id: u16) -> Packet {
    let mut packet = Packet::new();
    packet.header.set_version(1);
    packet.header.set_type(kind);
    packet.header.code = MessageClass::Request(method);
    packet.header.message_id = id;
    packet.set_token(vec![0xA5, id as u8]);
    for segment in path.split('/') {
        packet.add_option(CoapOption::UriPath, segment.as_bytes().to_vec());
    }
    packet
}

/// Reads one datagram as a packet, or nothing within a short while.
async fn answer(socket: &UdpSocket) -> Option<Packet> {
    let mut buf = [0u8; 1500];
    let len = tokio::time::timeout(Duration::from_millis(500), socket.recv(&mut buf))
        .await
        .ok()?
        .ok()?;
    Packet::from_bytes(&buf[..len]).ok()
}

/// A raw socket pointed at the server on `port`.
async fn raw(port: u16) -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
    let server: SocketAddr = format!("127.0.0.1:{port}").parse().expect("address");
    socket.connect(server).await.expect("connect");
    socket
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reading_reaches_the_gateway_and_a_stray_path_is_refused() {
    let (mut gateway, port) = gateway().await;
    let mut confirmable = node(port, Reliability::Confirmable).await;
    let mut unconfirmed = node(port, Reliability::NonConfirmable).await;

    confirmable
        .send_text("sensors/1/temperature", "21.5")
        .await
        .expect("a 2.04 Changed");
    let reading = next(&mut gateway).await.expect("a reading");
    assert_eq!(reading.topic, "sensors/1/temperature");
    assert_eq!(reading.text().expect("text"), "21.5");

    unconfirmed
        .send_text("sensors/2/temperature", "19.0")
        .await
        .expect("the datagram leaves");
    assert_eq!(next(&mut gateway).await.expect("a reading").topic, "sensors/2/temperature");

    match confirmable.send_text("pumps/1/state", "on").await {
        Err(Error::Transport(reason)) => assert_eq!(reason, "the server answered 4.04 Not Found"),
        other => panic!("a path the gateway does not take is refused, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_observer_gets_the_current_state_then_each_change() {
    let (mut gateway, port) = gateway().await;
    gateway.send_text("commands/valve", "closed").await.expect("state");
    let mut first = node(port, Reliability::Confirmable).await;
    let mut second = node(port, Reliability::Confirmable).await;
    first.subscribe("commands/valve").await.expect("observe");
    second.subscribe("commands/valve").await.expect("observe");
    assert_eq!(gateway.observers("commands/valve"), 2);

    for observer in [&mut first, &mut second] {
        let current = next(observer).await.expect("the current state");
        assert_eq!(current.topic, "commands/valve");
        assert_eq!(current.text().expect("text"), "closed");
    }
    gateway.send_text("commands/valve", "open").await.expect("change");
    for observer in [&mut first, &mut second] {
        assert_eq!(next(observer).await.expect("the change").text().expect("text"), "open");
    }

    match first.subscribe("commands/pump").await {
        Err(Error::Transport(reason)) => assert_eq!(reason, "the server answered 4.04 Not Found"),
        other => panic!("a resource with no state cannot be observed, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_publisher_sends_commands_while_the_server_waits_for_readings() {
    let (mut gateway, port) = gateway().await;
    let commands = gateway.publisher();
    commands.publish("commands/heater", b"off").await.expect("state");
    let mut observer = node(port, Reliability::Confirmable).await;
    observer.subscribe("commands/heater").await.expect("observe");
    assert_eq!(next(&mut observer).await.expect("the state").payload, b"off");

    let waiting = tokio::spawn(async move {
        let reading = next(&mut gateway).await;
        (gateway, reading)
    });
    commands.publish("commands/heater", b"on").await.expect("change");
    assert_eq!(next(&mut observer).await.expect("the change").payload, b"on");
    assert_eq!(commands.observers("commands/heater"), 1);

    observer
        .send_text("sensors/4/temperature", "18.0")
        .await
        .expect("a reading");
    let (_gateway, reading) = waiting.await.expect("the task");
    assert_eq!(reading.expect("the reading").topic, "sensors/4/temperature");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gateway_keeps_serving_after_notifying_an_observer_that_is_gone() {
    let (mut gateway, port) = gateway().await;
    gateway.send_text("commands/lamp", "off").await.expect("state");
    {
        let socket = raw(port).await;
        let mut observe = request(MessageType::Confirmable, RequestType::Get, "commands/lamp", 21);
        observe.add_option(CoapOption::Observe, Vec::new());
        socket.send(&observe.to_bytes().expect("encode")).await.expect("send");
        answer(&socket).await.expect("the registration");
    }
    gateway.send_text("commands/lamp", "on").await.expect("a notification to a closed port");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut reporter = node(port, Reliability::Confirmable).await;
    reporter
        .send_text("sensors/5/temperature", "20.0")
        .await
        .expect("the gateway still answers");
    assert_eq!(next(&mut gateway).await.expect("a reading").topic, "sensors/5/temperature");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_keeps_listening_after_sending_to_a_port_with_nothing_on_it() {
    let port = {
        let spare = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
        spare.local_addr().expect("address").port()
    };
    let mut client = node(port, Reliability::NonConfirmable).await;
    client.send_text("sensors/6/temperature", "17.5").await.expect("the datagram leaves");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut gateway = CoapServer::new(format!("127.0.0.1:{port}"));
    gateway.connect().await.expect("bind the same port");
    gateway.send_text("commands/door", "shut").await.expect("state");
    client.subscribe("commands/door").await.expect("observe");
    assert_eq!(next(&mut client).await.expect("the state").payload, b"shut");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retransmitted_request_is_answered_again_and_taken_once() {
    let (mut gateway, port) = gateway().await;
    let socket = raw(port).await;
    let mut put = request(MessageType::Confirmable, RequestType::Put, "sensors/3/level", 7);
    put.payload = b"40".to_vec();
    let bytes = put.to_bytes().expect("encode");

    socket.send(&bytes).await.expect("send");
    let first = answer(&socket).await.expect("an acknowledgment");
    socket.send(&bytes).await.expect("send again");
    let second = answer(&socket).await.expect("the same acknowledgment");
    assert_eq!(first.header.get_type(), MessageType::Acknowledgement);
    assert_eq!(first.header.code, MessageClass::Response(ResponseType::Changed));
    assert_eq!(first.header.message_id, 7);
    assert_eq!(first.to_bytes().ok(), second.to_bytes().ok());

    assert_eq!(next(&mut gateway).await.expect("the reading").payload, b"40");
    assert!(
        tokio::time::timeout(Duration::from_millis(200), gateway.recv())
            .await
            .is_err(),
        "the retransmission was not taken twice"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ping_is_answered_with_a_reset() {
    let (_gateway, port) = gateway().await;
    let socket = raw(port).await;
    let mut ping = Packet::new();
    ping.header.set_version(1);
    ping.header.set_type(MessageType::Confirmable);
    ping.header.code = MessageClass::Empty;
    ping.header.message_id = 99;
    socket.send(&ping.to_bytes().expect("encode")).await.expect("send");

    let pong = answer(&socket).await.expect("a reset");
    assert_eq!(pong.header.get_type(), MessageType::Reset);
    assert_eq!(pong.header.message_id, 99);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_observer_that_resets_a_notification_is_dropped() {
    let (mut gateway, port) = gateway().await;
    gateway.send_text("commands/fan", "off").await.expect("state");
    let socket = raw(port).await;
    let mut observe = request(MessageType::Confirmable, RequestType::Get, "commands/fan", 11);
    observe.add_option(CoapOption::Observe, Vec::new());
    socket.send(&observe.to_bytes().expect("encode")).await.expect("send");
    let registered = answer(&socket).await.expect("the registration");
    assert!(registered.get_option(CoapOption::Observe).is_some());
    assert_eq!(gateway.observers("commands/fan"), 1);

    gateway.send_text("commands/fan", "on").await.expect("change");
    let notification = answer(&socket).await.expect("a notification");
    assert_eq!(notification.header.get_type(), MessageType::NonConfirmable);
    assert_eq!(notification.get_token(), observe.get_token());
    let mut reset = Packet::new();
    reset.header.set_version(1);
    reset.header.set_type(MessageType::Reset);
    reset.header.message_id = notification.header.message_id;
    socket.send(&reset.to_bytes().expect("encode")).await.expect("send");

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(gateway.observers("commands/fan"), 0);
}
