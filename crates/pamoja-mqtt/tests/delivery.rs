//! Acknowledged delivery, retained messages, the last will, and signing in, against an
//! in-process broker.

mod common;

use std::time::Duration;

use common::{client_of, connect_with_retry, pick_port, spawn_broker, spawn_broker_with_user};
use pamoja_core::{Error, Receive, Transport};
use pamoja_mqtt::{MqttTransport, PublishOptions, QualityOfService, Will};

async fn next(
    link: &mut MqttTransport,
) -> Option<pamoja_core::Result<Option<pamoja_core::Message>>> {
    tokio::time::timeout(Duration::from_secs(3), link.recv())
        .await
        .ok()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_delivery_settles_when_the_broker_acknowledges_at_every_level() {
    let port = pick_port();
    spawn_broker(port);
    let mut subscriber = connect_with_retry(client_of(port, "ack-sub")).await;
    subscriber.subscribe("ack/#").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut publisher = connect_with_retry(client_of(port, "ack-pub")).await;

    for (at, qos) in [
        QualityOfService::AtMostOnce,
        QualityOfService::AtLeastOnce,
        QualityOfService::ExactlyOnce,
    ]
    .into_iter()
    .enumerate()
    {
        let topic = format!("ack/{at}");
        let delivery = publisher
            .publish(&topic, b"level", PublishOptions::new().qos(qos))
            .await
            .expect("queued");
        tokio::time::timeout(Duration::from_secs(3), delivery.confirmed())
            .await
            .expect("the broker answers in time")
            .unwrap_or_else(|error| panic!("{qos:?} is acknowledged: {error}"));
        let received = next(&mut subscriber)
            .await
            .expect("a message in time")
            .expect("no error")
            .expect("a message");
        assert_eq!(received.topic, topic);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn many_deliveries_in_flight_settle_each_on_its_own_acknowledgment() {
    let port = pick_port();
    spawn_broker(port);
    let mut publisher = connect_with_retry(client_of(port, "burst-pub")).await;
    let mut deliveries = Vec::new();
    for at in 0..40u8 {
        let qos = if at % 2 == 0 {
            QualityOfService::AtLeastOnce
        } else {
            QualityOfService::ExactlyOnce
        };
        deliveries.push(
            publisher
                .publish("burst/readings", &[at], PublishOptions::new().qos(qos))
                .await
                .expect("queued"),
        );
    }
    for delivery in deliveries {
        tokio::time::timeout(Duration::from_secs(5), delivery.confirmed())
            .await
            .expect("every acknowledgment arrives")
            .expect("and settles its own delivery");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_delivery_still_waiting_when_the_connection_is_taken_over_fails() {
    let port = pick_port();
    spawn_broker(port);
    let mut first = connect_with_retry(client_of(port, "orphan")).await;
    first.subscribe("orphan/#").await.expect("subscribe");
    let mut deliveries = Vec::new();
    for _ in 0..20 {
        match first.publish("orphan/1", b"x", PublishOptions::new()).await {
            Ok(delivery) => deliveries.push(delivery),
            Err(_) => break,
        }
    }
    let _second = connect_with_retry(client_of(port, "orphan")).await;
    let _ = next(&mut first).await;
    for delivery in deliveries {
        match tokio::time::timeout(Duration::from_secs(3), delivery.confirmed())
            .await
            .expect("every delivery settles one way or the other")
        {
            Ok(()) | Err(Error::Transport(_)) => {}
            Err(other) => panic!("a cut-off delivery says the connection ended, got {other}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_task_waiting_on_the_inbox_leaves_the_transport_free_to_publish() {
    let port = pick_port();
    spawn_broker(port);
    let mut node = connect_with_retry(client_of(port, "inbox-node")).await;
    node.subscribe("inbox/commands").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let inbox = node.inbox();
    let waiting = tokio::spawn(async move { inbox.recv().await });
    node.publish("inbox/readings", b"21.5", PublishOptions::new())
        .await
        .expect("queued while the inbox waits")
        .confirmed()
        .await
        .expect("and acknowledged");
    node.send("inbox/commands", b"stop").await.expect("send");

    let command = tokio::time::timeout(Duration::from_secs(3), waiting)
        .await
        .expect("the command arrives in time")
        .expect("the waiting task finished")
        .expect("no error")
        .expect("a message");
    assert_eq!(command.payload, b"stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retained_message_reaches_a_later_subscriber_until_it_is_cleared() {
    let port = pick_port();
    spawn_broker(port);
    let mut publisher = connect_with_retry(client_of(port, "keep-pub")).await;
    publisher
        .publish("keep/tank", b"73", PublishOptions::new().retained())
        .await
        .expect("queued")
        .confirmed()
        .await
        .expect("held by the broker");

    let mut late = connect_with_retry(client_of(port, "keep-late")).await;
    late.subscribe("keep/tank").await.expect("subscribe");
    let kept = next(&mut late)
        .await
        .expect("the retained message arrives on subscribing")
        .expect("no error")
        .expect("a message");
    assert_eq!(kept.payload, b"73");

    publisher
        .publish("keep/tank", b"", PublishOptions::new().retained())
        .await
        .expect("queued")
        .confirmed()
        .await
        .expect("the clearing message is held");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut later = connect_with_retry(client_of(port, "keep-later")).await;
    later.subscribe("keep/tank").await.expect("subscribe");
    match next(&mut later).await {
        None => {}
        Some(Ok(Some(message))) => assert!(
            message.payload.is_empty(),
            "a cleared topic hands a new subscriber nothing, got {:?}",
            message.payload
        ),
        Some(other) => panic!("a cleared topic hands a new subscriber nothing, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_will_is_published_when_a_client_drops_without_disconnecting() {
    let port = pick_port();
    spawn_broker(port);
    let mut watcher = connect_with_retry(client_of(port, "will-watch")).await;
    watcher
        .subscribe("sites/+/status")
        .await
        .expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let will = Will::new("sites/pump-3/status", b"offline").qos(QualityOfService::AtLeastOnce);
    let pump = connect_with_retry(client_of(port, "pump-3").last_will(will)).await;
    drop(pump);

    let said = next(&mut watcher)
        .await
        .expect("the broker publishes the will")
        .expect("no error")
        .expect("a message");
    assert_eq!(said.topic, "sites/pump-3/status");
    assert_eq!(said.payload, b"offline");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_will_is_not_published_when_a_client_disconnects() {
    let port = pick_port();
    spawn_broker(port);
    let mut watcher = connect_with_retry(client_of(port, "calm-watch")).await;
    watcher.subscribe("calm/status").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut polite =
        connect_with_retry(client_of(port, "calm").last_will(Will::new("calm/status", b"gone")))
            .await;
    polite.disconnect().await.expect("disconnect");
    assert!(
        next(&mut watcher).await.is_none(),
        "a goodbye leaves no will"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_will_on_a_wildcard_topic_is_refused_before_connecting() {
    let mut transport = MqttTransport::new(
        client_of(pick_port(), "wild").last_will(Will::new("sites/+/status", b"offline")),
    );
    match transport.connect().await {
        Err(Error::Transport(reason)) => assert!(reason.contains("wildcard"), "{reason}"),
        other => panic!("a will topic with a wildcard is refused, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_broker_that_checks_credentials_admits_the_right_ones_alone() {
    let port = pick_port();
    spawn_broker_with_user(port, "field-node", "hunter2");
    common::listening(port).await;

    let admitted =
        connect_with_retry(client_of(port, "signed-in").credentials("field-node", "hunter2")).await;
    assert!(admitted.is_connected());

    // rumqttd closes the connection on a wrong password rather than answering with a
    // CONNACK return code, so the refusal is all a client can observe here.
    let mut refused =
        MqttTransport::new(client_of(port, "wrong-password").credentials("field-node", "guess"));
    assert!(
        refused.connect().await.is_err(),
        "a wrong password is refused"
    );
    assert!(!refused.is_connected());
}

#[test]
fn a_password_is_left_out_of_the_debug_form() {
    let config = client_of(1883, "secret").credentials("field-node", "hunter2");
    let shown = format!("{config:?}");
    assert!(shown.contains("field-node"), "{shown}");
    assert!(!shown.contains("hunter2"), "{shown}");
    assert_eq!(config.username(), Some("field-node"));
}
