//! A connection over TLS, with the broker's certificate checked, and a client certificate
//! presented to a broker that asks for one.
//!
//! The embedded broker speaks plain MQTT, so a TLS terminator built on rustls stands in
//! front of it, holding a certificate issued for `localhost` by an authority generated for
//! the test.

#![cfg(feature = "tls")]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{connect_with_retry, listening, pick_port, spawn_broker};
use pamoja_core::{Error, Receive, Transport};
use pamoja_mqtt::{MqttConfig, MqttTransport, Tls};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair};
use rustls::crypto::ring::default_provider;
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

/// An authority and the certificates it issued, in PEM and DER.
struct Issued {
    authority_pem: String,
    authority_der: CertificateDer<'static>,
    server: (CertificateDer<'static>, PrivateKeyDer<'static>),
    client_pem: (String, String),
}

fn issue(names: &str) -> Issued {
    let authority_key = KeyPair::generate().expect("an authority key");
    let mut authority = CertificateParams::new(Vec::<String>::new()).expect("params");
    authority.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    authority
        .distinguished_name
        .push(DnType::CommonName, format!("{names} test authority"));
    let authority = authority.self_signed(&authority_key).expect("self-signed");

    let server_key = KeyPair::generate().expect("a server key");
    let server = CertificateParams::new(vec!["localhost".to_owned()])
        .expect("params")
        .signed_by(&server_key, &authority, &authority_key)
        .expect("issued");

    let client_key = KeyPair::generate().expect("a client key");
    let mut client = CertificateParams::new(Vec::<String>::new()).expect("params");
    client
        .distinguished_name
        .push(DnType::CommonName, "field-node-7");
    let client = client
        .signed_by(&client_key, &authority, &authority_key)
        .expect("issued");

    Issued {
        authority_pem: authority.pem(),
        authority_der: authority.der().clone(),
        server: (
            server.der().clone(),
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
        ),
        client_pem: (client.pem(), client_key.serialize_pem()),
    }
}

/// Terminates TLS on `tls_port` and passes the plain bytes to the broker on `plain_port`.
async fn terminate(issued: &Issued, tls_port: u16, plain_port: u16, verify_clients: bool) {
    let provider = Arc::new(default_provider());
    let builder = ServerConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .expect("protocol versions");
    let builder = if verify_clients {
        let mut roots = RootCertStore::empty();
        roots.add(issued.authority_der.clone()).expect("authority");
        builder.with_client_cert_verifier(
            WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider)
                .build()
                .expect("a client verifier"),
        )
    } else {
        builder.with_no_client_auth()
    };
    let config = builder
        .with_single_cert(vec![issued.server.0.clone()], issued.server.1.clone_key())
        .expect("the server certificate");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(("127.0.0.1", tls_port))
        .await
        .expect("bind the TLS port");
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut secured) = acceptor.accept(socket).await else {
                    return;
                };
                let Ok(mut broker) = TcpStream::connect(("127.0.0.1", plain_port)).await else {
                    return;
                };
                let _ = tokio::io::copy_bidirectional(&mut secured, &mut broker).await;
            });
        }
    });
}

fn over_tls(port: u16, id: &str, tls: Tls) -> MqttConfig {
    MqttConfig::new(id, "localhost", port)
        .keep_alive(Duration::from_secs(5))
        .tls(tls)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_message_crosses_a_connection_secured_with_the_brokers_authority() {
    let (plain, secure) = (pick_port(), pick_port());
    spawn_broker(plain);
    listening(plain).await;
    let issued = issue("trusted");
    terminate(&issued, secure, plain, false).await;

    let trust = || Tls::with_ca_pem(issued.authority_pem.clone());
    let mut subscriber = connect_with_retry(over_tls(secure, "tls-sub", trust())).await;
    subscriber.subscribe("secure/#").await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut publisher = connect_with_retry(over_tls(secure, "tls-pub", trust())).await;
    publisher
        .send("secure/reading", b"21.5")
        .await
        .expect("publish");

    let received = tokio::time::timeout(Duration::from_secs(5), subscriber.recv())
        .await
        .expect("a message in time")
        .expect("no error")
        .expect("a message");
    assert_eq!(received.payload, b"21.5");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_broker_whose_certificate_another_authority_issued_is_refused() {
    let (plain, secure) = (pick_port(), pick_port());
    spawn_broker(plain);
    listening(plain).await;
    let issued = issue("real");
    terminate(&issued, secure, plain, false).await;
    listening(secure).await;

    let stranger = issue("stranger");
    let mut transport = MqttTransport::new(over_tls(
        secure,
        "tls-stranger",
        Tls::with_ca_pem(stranger.authority_pem),
    ));
    match transport.connect().await {
        Err(Error::Transport(reason)) => assert!(
            reason.to_lowercase().contains("certificate"),
            "the reason names the certificate: {reason}"
        ),
        other => panic!("an untrusted broker is refused, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_broker_that_asks_for_a_client_certificate_admits_one_it_issued() {
    let (plain, secure) = (pick_port(), pick_port());
    spawn_broker(plain);
    listening(plain).await;
    let issued = issue("mutual");
    terminate(&issued, secure, plain, true).await;
    listening(secure).await;

    let (certificate, key) = issued.client_pem.clone();
    let signed =
        Tls::with_ca_pem(issued.authority_pem.clone()).client_certificate(certificate, key);
    let admitted = connect_with_retry(over_tls(secure, "tls-mutual", signed)).await;
    assert!(admitted.is_connected());

    let mut anonymous = MqttTransport::new(over_tls(
        secure,
        "tls-anonymous",
        Tls::with_ca_pem(issued.authority_pem.clone()),
    ));
    assert!(
        anonymous.connect().await.is_err(),
        "a client with no certificate is turned away"
    );
}

#[test]
fn a_ca_file_that_is_not_pem_is_refused_with_the_reason() {
    for (file, reason) in [
        (&b"not a certificate"[..], "holds no CERTIFICATE block"),
        (
            &b"-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n"[..],
            "is not PEM",
        ),
    ] {
        let config = over_tls(8883, "bad-ca", Tls::with_ca_pem(file));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        match runtime.block_on(MqttTransport::new(config).connect()) {
            Err(Error::Transport(said)) => assert!(said.contains(reason), "{said}"),
            other => panic!("a broken CA file is refused before connecting, got {other:?}"),
        }
    }
}

#[test]
fn a_private_key_is_left_out_of_the_debug_form() {
    let issued = issue("hidden");
    let (certificate, key) = issued.client_pem;
    let tls = Tls::with_ca_pem(issued.authority_pem).client_certificate(certificate, key.clone());
    let shown = format!("{tls:?}");
    assert!(!shown.contains("PRIVATE KEY"), "{shown}");
    assert!(shown.contains("client_certificate: true"), "{shown}");
}
