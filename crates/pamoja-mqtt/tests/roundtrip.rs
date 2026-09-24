//! End-to-end transport test against an in-process MQTT broker.
//!
//! An embedded `rumqttd` broker is started on an ephemeral port so the full
//! publish/subscribe path is exercised with no external infrastructure.

mod common;

use std::time::Duration;

use common::{client_of, connect_with_retry, pick_port, spawn_broker};
use pamoja_core::{Error, Receive, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport};

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

/// Waits a short while for a message, so a test of silence does not hang.
async fn next(
    link: &mut MqttTransport,
) -> Option<pamoja_core::Result<Option<pamoja_core::Message>>> {
    tokio::time::timeout(Duration::from_secs(2), link.recv())
        .await
        .ok()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_send_over_the_packet_limit_is_refused_and_the_connection_stays_up() {
    let port = pick_port();
    spawn_broker(port);
    let mut subscriber = connect_with_retry(client_of(port, "limit-sub")).await;
    subscriber.subscribe("limit/#").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut publisher = connect_with_retry(client_of(port, "limit-pub")).await;

    match publisher.send("limit/big", &[b'x'; 20_000]).await {
        Err(Error::Transport(reason)) => assert!(reason.contains("limit"), "{reason}"),
        other => panic!("an oversized send must be refused, got {other:?}"),
    }
    assert!(matches!(
        publisher.send("limit/+/x", b"1").await,
        Err(Error::Transport(_))
    ));
    assert!(
        publisher.is_connected(),
        "neither refusal ends the connection"
    );

    publisher
        .send("limit/small", b"1")
        .await
        .expect("a small send");
    let received = next(&mut subscriber)
        .await
        .expect("a message in time")
        .expect("no error")
        .expect("a message");
    assert_eq!(received.topic, "limit/small");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_packet_over_the_receivers_limit_ends_its_connection_and_says_why() {
    let port = pick_port();
    spawn_broker(port);
    let mut subscriber =
        connect_with_retry(client_of(port, "small-sub").max_packet_size(1_024)).await;
    subscriber.subscribe("big/#").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut publisher =
        connect_with_retry(client_of(port, "big-pub").max_packet_size(16_384)).await;
    publisher
        .send("big/1", &[b'x'; 4_000])
        .await
        .expect("within the publisher's limit");

    match next(&mut subscriber).await.expect("an answer in time") {
        Err(Error::Transport(reason)) => assert!(reason.contains("ended"), "{reason}"),
        other => panic!("the subscriber must say why its connection ended, got {other:?}"),
    }
    assert!(matches!(next(&mut subscriber).await, Some(Ok(None))));
    assert!(!subscriber.is_connected());
    assert!(matches!(
        subscriber.send("big/2", b"1").await,
        Err(Error::Closed)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_client_with_the_same_id_ends_the_first_connection() {
    let port = pick_port();
    spawn_broker(port);
    let mut first = connect_with_retry(client_of(port, "twin")).await;
    first.subscribe("twin/1").await.expect("subscribe");
    let _second = connect_with_retry(client_of(port, "twin")).await;

    match next(&mut first).await.expect("an answer in time") {
        Err(Error::Transport(reason)) => assert!(reason.contains("ended"), "{reason}"),
        other => panic!("the first client must say its connection ended, got {other:?}"),
    }
    assert!(!first.is_connected());
    assert!(matches!(
        first.send("twin/1", b"1").await,
        Err(Error::Closed)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reconnect_starts_a_clean_session() {
    let port = pick_port();
    spawn_broker(port);
    let mut subscriber = connect_with_retry(client_of(port, "clean-sub")).await;
    subscriber.subscribe("clean/1").await.expect("subscribe");
    subscriber.disconnect().await.expect("disconnect");
    assert!(!subscriber.is_connected());
    subscriber.connect().await.expect("reconnect");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut publisher = connect_with_retry(client_of(port, "clean-pub")).await;
    publisher.send("clean/1", b"1").await.expect("send");
    assert!(
        next(&mut subscriber).await.is_none(),
        "the subscription did not survive the reconnect"
    );
}
