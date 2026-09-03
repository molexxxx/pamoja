//! An HTTP/1.1 server that serves the dashboard over a pluggable byte transport.
//!
//! This is the host side of the local-first dashboard: it serves the static page, the
//! language-neutral `GET /state` snapshot, and a `GET /events` server-sent-event
//! stream for live updates, with no web framework and no async runtime. A thread per
//! connection keeps it simple and is ample for the handful of clients a node sees over
//! its own hotspot. The same code backs the [`Mock`](crate::Mock) in development and a
//! real node in the field, since both are just a [`StateSource`].
//!
//! ## Why HTTP/1.1, and the seam for more
//!
//! HTTP/1.1 is the baseline on purpose: it is exactly what a browser speaks to a
//! device over a plain `http://` hotspot, where there is no CA-trusted certificate and
//! so no HTTPS (and therefore no browser HTTP/2, which is only negotiated over TLS).
//! It also fits the smallest tiers, where a TLS plus HTTP/2 stack would not. The
//! request handling is generic over any [`Read`] + [`Write`] stream, and connections
//! arrive through a [`Transport`], so a capable tier can later supply a TLS transport
//! (which negotiates HTTP/2 for free in the browser) without touching the request
//! logic. [`TcpTransport`] is the plain-TCP baseline; a `rustls`-backed transport is
//! the intended Tier A addition.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Deserialize;

use crate::assets::Assets;
use crate::auth::Auth;
use crate::catalog::Catalog;
use crate::command::Command;
use crate::source::StateSource;

/// How connections reach the server: the seam that keeps the byte transport pluggable.
///
/// The baseline is [`TcpTransport`] (plain TCP, HTTP/1.1). A capable tier can
/// implement this over a TLS stream so the browser negotiates HTTPS, and with it
/// HTTP/2, while the request handling above stays unchanged.
pub trait Transport: Send + 'static {
    /// The connection type this transport yields, a bidirectional byte stream.
    type Conn: Read + Write + Send + 'static;

    /// Blocks until the next client connects.
    ///
    /// # Returns
    ///
    /// The accepted connection.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from the underlying accept call.
    fn accept(&self) -> std::io::Result<Self::Conn>;

    /// A human-readable address for the startup log line, such as a URL.
    ///
    /// # Returns
    ///
    /// The address to print when the server starts serving.
    fn describe(&self) -> String;
}

/// The plain-TCP, HTTP/1.1 transport: the baseline that works on any tier.
pub struct TcpTransport {
    listener: TcpListener,
}

impl TcpTransport {
    /// Binds a listener on `addr`.
    ///
    /// # Arguments
    ///
    /// * `addr` - the address to listen on, such as `"0.0.0.0:80"`.
    ///
    /// # Returns
    ///
    /// A transport ready to accept connections.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from binding if the address is unavailable.
    pub fn bind(addr: impl ToSocketAddrs) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
        })
    }
}

impl Transport for TcpTransport {
    type Conn = TcpStream;

    fn accept(&self) -> std::io::Result<Self::Conn> {
        self.listener.accept().map(|(stream, _)| stream)
    }

    fn describe(&self) -> String {
        match self.listener.local_addr() {
            Ok(addr) => format!("http://{addr}"),
            Err(_) => "http://?".to_owned(),
        }
    }
}

/// The dashboard HTTP server, generic over whatever produces its state.
///
/// Build one with [`Server::new`], optionally set the live-update cadence with
/// [`Server::with_push_interval`], then block in [`Server::run`] (plain TCP) or
/// [`Server::run_on`] (a custom `Transport`).
///
/// # Examples
///
/// ```no_run
/// use pamoja_dashboard::{Assets, Server, State, StateSource, Status};
///
/// // Any StateSource works; a real node produces its live State here, and the
/// // `mock` feature's `Mock` is the hardware-free stand-in for development.
/// struct Node;
/// impl StateSource for Node {
///     fn snapshot(&mut self) -> State {
///         State { orgs: Vec::new(), status: Status::Ok, uptime_secs: None, demo: false }
///     }
/// }
///
/// let server = Server::new(Node, Assets::Embedded);
/// server.run("127.0.0.1:8080").expect("serve");
/// ```
pub struct Server<S> {
    source: Arc<Mutex<S>>,
    assets: Assets,
    push_interval: Duration,
    auth: Arc<Auth>,
    catalog: Option<Arc<String>>,
}

impl<S: StateSource + Send + 'static> Server<S> {
    /// Creates a server that renders `source` with `assets`.
    ///
    /// Control is authenticated against a freshly generated pairing secret that nobody
    /// holds yet, so no client can issue commands until [`with_pairing_secret`] sets the
    /// secret the device actually shows. Read-only viewing needs no pairing.
    ///
    /// [`with_pairing_secret`]: Server::with_pairing_secret
    ///
    /// # Arguments
    ///
    /// * `source` - the state source to serve, a real node or a `Mock`.
    /// * `assets` - where the page files come from, embedded or a directory.
    ///
    /// # Returns
    ///
    /// A server pushing live updates once a second by default.
    pub fn new(source: S, assets: Assets) -> Self {
        Self {
            source: Arc::new(Mutex::new(source)),
            assets,
            push_interval: Duration::from_secs(1),
            auth: Arc::new(Auth::new(Auth::generate_secret())),
            catalog: None,
        }
    }

    /// Serves a presentation [`Catalog`] at `GET /catalog` so the page can show the
    /// deployment's custom sensors, stats, and theme.
    ///
    /// Build the catalog from the profiles the deployment runs with
    /// [`Catalog::from_profiles`]. A catalog with nothing custom is not worth serving;
    /// skip this call and the page keeps its built-in defaults.
    ///
    /// # Arguments
    ///
    /// * `catalog` - the presentation catalog to serve.
    ///
    /// # Returns
    ///
    /// The server, for chaining.
    pub fn with_catalog(mut self, catalog: Catalog) -> Self {
        self.catalog = catalog
            .to_json()
            .ok()
            .filter(|_| !catalog.is_empty())
            .map(Arc::new);
        self
    }

    /// Sets the pairing secret a client must know to issue commands.
    ///
    /// The secret is shown out of band (the device's screen, a QR code, or the dev
    /// server's console) and never crosses the network.
    ///
    /// # Arguments
    ///
    /// * `secret` - the canonical pairing secret.
    ///
    /// # Returns
    ///
    /// The server, for chaining.
    pub fn with_pairing_secret(mut self, secret: impl Into<String>) -> Self {
        self.auth = Arc::new(Auth::new(secret));
        self
    }

    /// Sets how often the `GET /events` stream pushes a fresh snapshot.
    ///
    /// # Arguments
    ///
    /// * `interval` - the delay between pushes.
    ///
    /// # Returns
    ///
    /// The server, for chaining.
    pub fn with_push_interval(mut self, interval: Duration) -> Self {
        self.push_interval = interval;
        self
    }

    /// Binds `addr` over plain TCP and serves forever.
    ///
    /// # Arguments
    ///
    /// * `addr` - the address to listen on, such as `"0.0.0.0:80"`.
    ///
    /// # Returns
    ///
    /// Never returns on success; it serves until the process ends.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from binding the listener.
    pub fn run(self, addr: impl ToSocketAddrs) -> std::io::Result<()> {
        let transport = TcpTransport::bind(addr)?;
        self.run_on(transport)
    }

    /// Serves forever over a supplied `Transport`, one thread per connection.
    ///
    /// This is the seam for a non-default transport, such as a future TLS transport
    /// for a capable tier.
    ///
    /// # Arguments
    ///
    /// * `transport` - the source of connections.
    ///
    /// # Returns
    ///
    /// Never returns on success; it serves until the process ends.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] only if accepting fails unrecoverably; transient
    /// accept errors are skipped.
    pub fn run_on<T: Transport>(self, transport: T) -> std::io::Result<()> {
        println!("pamoja-dashboard: serving on {}", transport.describe());
        loop {
            let conn = match transport.accept() {
                Ok(conn) => conn,
                Err(_) => continue,
            };
            let source = Arc::clone(&self.source);
            let assets = self.assets.clone();
            let interval = self.push_interval;
            let auth = Arc::clone(&self.auth);
            let catalog = self.catalog.clone();
            thread::spawn(move || {
                let _ = handle(conn, source, assets, interval, auth, catalog);
            });
        }
    }
}

// One parsed request line: the method, the path, the raw query string, any body, and
// whether the client accepts a gzip-encoded response.
struct Request {
    method: String,
    path: String,
    query: String,
    body: Vec<u8>,
    accept_gzip: bool,
}

// A client's proof that it derived the session key during pairing.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    session_id: String,
    mac: String,
}

// An authenticated command: the session, its replay counter, the exact command string
// that was signed, and the MAC over (counter, command).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandRequest {
    session_id: String,
    counter: u64,
    cmd: String,
    mac: String,
}

fn handle<S: StateSource, C: Read + Write>(
    mut conn: C,
    source: Arc<Mutex<S>>,
    assets: Assets,
    interval: Duration,
    auth: Arc<Auth>,
    catalog: Option<Arc<String>>,
) -> std::io::Result<()> {
    let request = match read_request(&mut conn)? {
        Some(request) => request,
        None => return Ok(()),
    };

    // A `?scenario=` parameter is a dev affordance: it asks the source to switch view.
    if let Some(scenario) = query_value(&request.query, "scenario") {
        if let Ok(mut source) = source.lock() {
            source.select(&scenario);
        }
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/state") => {
            let json = snapshot_json(&source);
            write_response(
                &mut conn,
                200,
                "OK",
                "application/json; charset=utf-8",
                json.as_bytes(),
            )
        }
        ("GET", "/catalog") => match &catalog {
            Some(json) => write_json(&mut conn, 200, "OK", json),
            None => write_response(
                &mut conn,
                204,
                "No Content",
                "application/json; charset=utf-8",
                b"",
            ),
        },
        ("GET", "/locales") => {
            // The locales this build actually embeds, so the page offers only the languages a
            // Tier B firmware kept in flash. A static host has no device and the page keeps its
            // full built-in list.
            let tags = crate::assets::embedded_locales();
            let json = format!(
                "[{}]",
                tags.iter()
                    .map(|tag| format!("\"{tag}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            write_json(&mut conn, 200, "OK", &json)
        }
        ("GET", "/lite") => {
            // The no-JavaScript floor: a status table built once on the device, refreshed by
            // a meta tag. The embedded floor page bounces here when scripting is off.
            let html = match source.lock() {
                Ok(mut source) => crate::lite::render_lite(&source.snapshot()),
                Err(_) => crate::lite::render_unavailable(),
            };
            write_response(
                &mut conn,
                200,
                "OK",
                "text/html; charset=utf-8",
                html.as_bytes(),
            )
        }
        ("GET", "/events") => stream_events(&mut conn, &source, interval),
        ("GET", "/pair/challenge") => {
            let challenge = auth.challenge();
            let json = format!(
                r#"{{"sessionId":"{}","nonce":"{}"}}"#,
                challenge.session_id, challenge.nonce
            );
            write_json(&mut conn, 200, "OK", &json)
        }
        ("POST", "/pair/confirm") => {
            match serde_json::from_slice::<ConfirmRequest>(&request.body) {
                Ok(confirm) => match auth.confirm(&confirm.session_id, &confirm.mac) {
                    Ok(()) => write_json(&mut conn, 200, "OK", "{}"),
                    Err(error) => write_json(
                        &mut conn,
                        401,
                        "Unauthorized",
                        &format!(r#"{{"error":"{}"}}"#, error.code()),
                    ),
                },
                Err(_) => write_json(&mut conn, 400, "Bad Request", r#"{"error":"bad_request"}"#),
            }
        }
        ("POST", "/command") => handle_command(&mut conn, &source, &auth, &request.body),
        ("GET", path) => match assets.get(path) {
            Some((content_type, bytes)) => {
                write_asset(&mut conn, content_type, &bytes, request.accept_gzip)
            }
            None => write_response(&mut conn, 404, "Not Found", "text/plain", b"not found"),
        },
        _ => write_response(
            &mut conn,
            405,
            "Method Not Allowed",
            "text/plain",
            b"method not allowed",
        ),
    }
}

// Authenticates a command, dispatches it to the source, and writes the result.
fn handle_command<S: StateSource, W: Write>(
    conn: &mut W,
    source: &Arc<Mutex<S>>,
    auth: &Arc<Auth>,
    body: &[u8],
) -> std::io::Result<()> {
    let request: CommandRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => return write_json(conn, 400, "Bad Request", r#"{"error":"bad_request"}"#),
    };
    if let Err(error) = auth.verify_command(
        &request.session_id,
        request.counter,
        &request.cmd,
        &request.mac,
    ) {
        return write_json(
            conn,
            401,
            "Unauthorized",
            &format!(r#"{{"error":"{}"}}"#, error.code()),
        );
    }
    let command: Command = match serde_json::from_str(&request.cmd) {
        Ok(command) => command,
        Err(_) => return write_json(conn, 400, "Bad Request", r#"{"error":"bad_request"}"#),
    };
    let outcome = match source.lock() {
        Ok(mut source) => source.command(&command),
        Err(_) => {
            return write_json(
                conn,
                500,
                "Internal Server Error",
                r#"{"error":"internal"}"#,
            )
        }
    };
    match outcome {
        Ok(()) => write_json(conn, 200, "OK", "{}"),
        Err(error) => write_json(
            conn,
            422,
            "Unprocessable Entity",
            &format!(r#"{{"error":"{}"}}"#, error.code()),
        ),
    }
}

// Reads and parses the request line and drains the headers and any body. Returns
// `None` on an empty or malformed request.
fn read_request<C: Read>(conn: &mut C) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(conn);

    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut parts = line.split_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        return Ok(None);
    };
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), query.to_owned()),
        None => (target.to_owned(), String::new()),
    };

    // Drain the headers, noting a body length so a POST can be consumed politely and
    // whether the client accepts a gzip-encoded response.
    let mut content_length = 0usize;
    let mut accept_gzip = false;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        // Header names are case-insensitive; clients send these in any case.
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("accept-encoding") {
                accept_gzip = value.to_ascii_lowercase().contains("gzip");
            }
        }
    }
    let mut body = Vec::new();
    if content_length > 0 {
        body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;
    }

    Ok(Some(Request {
        method: method.to_owned(),
        path,
        query,
        body,
        accept_gzip,
    }))
}

// Pulls one value out of a `key=value&...` query string.
fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| value.to_owned())
    })
}

fn snapshot_json<S: StateSource>(source: &Arc<Mutex<S>>) -> String {
    match source.lock() {
        Ok(mut source) => source
            .snapshot()
            .to_json()
            .unwrap_or_else(|_| "{}".to_owned()),
        Err(_) => "{}".to_owned(),
    }
}

// Streams a fresh snapshot as a server-sent event on a repeating cadence until the
// client disconnects, which surfaces as a write error.
fn stream_events<S: StateSource, W: Write>(
    conn: &mut W,
    source: &Arc<Mutex<S>>,
    interval: Duration,
) -> std::io::Result<()> {
    let headers = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\r\n";
    conn.write_all(headers.as_bytes())?;
    conn.flush()?;
    loop {
        let json = snapshot_json(source);
        if conn
            .write_all(format!("data: {json}\n\n").as_bytes())
            .is_err()
        {
            break;
        }
        if conn.flush().is_err() {
            break;
        }
        thread::sleep(interval);
    }
    Ok(())
}

// Serves a static asset, gzip-encoded when the client accepts it. The one-time asset load
// is the dominant transfer over a weak hotspot link, so compressing it is the main win; a
// client that does not accept gzip still gets the identity bytes.
fn write_asset<W: Write>(
    conn: &mut W,
    content_type: &str,
    bytes: &[u8],
    gzip: bool,
) -> std::io::Result<()> {
    if !gzip {
        return write_response(conn, 200, "OK", content_type, bytes);
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes)?;
    let compressed = encoder.finish()?;
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Encoding: gzip\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\r\n",
        len = compressed.len(),
    );
    conn.write_all(header.as_bytes())?;
    conn.write_all(&compressed)?;
    conn.flush()
}

// Writes a JSON response with the given status.
fn write_json<W: Write>(conn: &mut W, code: u16, reason: &str, json: &str) -> std::io::Result<()> {
    write_response(
        conn,
        code,
        reason,
        "application/json; charset=utf-8",
        json.as_bytes(),
    )
}

fn write_response<W: Write>(
    conn: &mut W,
    code: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {code} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\r\n",
        len = body.len(),
    );
    conn.write_all(header.as_bytes())?;
    conn.write_all(body)?;
    conn.flush()
}

// The server tests drive the handler with the demo fleet as their state source, so they
// build only with the `mock` feature; CI runs them with `--features mock`.
#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::{Mock, Scenario};

    #[test]
    fn query_value_extracts_a_named_parameter() {
        assert_eq!(
            query_value("scenario=alarm&locale=sw", "scenario").as_deref(),
            Some("alarm")
        );
        assert_eq!(query_value("locale=sw", "scenario"), None);
        assert_eq!(query_value("", "scenario"), None);
    }

    fn handle_request(request: &[u8], auth: Arc<Auth>) -> String {
        let mut conn = MemConn::new(request);
        let source = Arc::new(Mutex::new(Mock::new(Scenario::Normal)));
        handle(
            &mut conn,
            source,
            Assets::Embedded,
            Duration::from_millis(0),
            auth,
            None,
        )
        .expect("handled");
        String::from_utf8_lossy(&conn.output).into_owned()
    }

    #[test]
    fn a_get_state_request_is_served_as_json_over_any_stream() {
        // The handler runs over an in-memory stream with no socket, proving it is
        // transport-agnostic: this is exactly what a TLS transport would plug into.
        let mut conn = MemConn::new(b"GET /state HTTP/1.1\r\nHost: x\r\n\r\n");
        let source = Arc::new(Mutex::new(Mock::new(Scenario::Alarm)));
        handle(
            &mut conn,
            source,
            Assets::Embedded,
            Duration::from_millis(0),
            Arc::new(Auth::new("secret")),
            None,
        )
        .expect("handled");
        let written = String::from_utf8_lossy(&conn.output);
        assert!(written.contains("200 OK"));
        assert!(written.contains("\"status\":\"alarm\""));
    }

    #[test]
    fn an_unknown_path_is_a_404() {
        let written = handle_request(b"GET /nope HTTP/1.1\r\n\r\n", Arc::new(Auth::new("secret")));
        assert!(written.contains("404 Not Found"));
    }

    #[test]
    fn a_get_catalog_serves_the_presentation_catalog_when_one_is_set() {
        use pamoja_profile::{ElementSpec, Presentation, Profile, Viz};

        let profile = Profile::well_level().with_presentation(Presentation::new().with_element(
            ElementSpec::new("water_turbidity", "ntu", "Turbidity", Viz::Gauge).with_band(0.0, 5.0),
        ));
        let catalog = Catalog::from_profiles(&[&profile])
            .to_json()
            .expect("serialize catalog");

        let mut conn = MemConn::new(b"GET /catalog HTTP/1.1\r\n\r\n");
        handle(
            &mut conn,
            Arc::new(Mutex::new(Mock::new(Scenario::Normal))),
            Assets::Embedded,
            Duration::from_millis(0),
            Arc::new(Auth::new("s")),
            Some(Arc::new(catalog)),
        )
        .expect("handled");

        let written = String::from_utf8_lossy(&conn.output);
        assert!(written.contains("200 OK"));
        assert!(written.contains("water_turbidity"));
        assert!(written.contains("\"viz\":\"radial\""));
    }

    #[test]
    fn a_get_catalog_is_no_content_without_a_catalog() {
        let written = handle_request(b"GET /catalog HTTP/1.1\r\n\r\n", Arc::new(Auth::new("s")));
        assert!(written.contains("204 No Content"));
    }

    #[cfg(not(feature = "tier-c"))]
    #[test]
    fn a_get_locales_lists_the_embedded_locales() {
        let written = handle_request(b"GET /locales HTTP/1.1\r\n\r\n", Arc::new(Auth::new("s")));
        assert!(written.contains("200 OK"));
        // English is always embedded, so it is always listed; the page narrows to this set.
        assert!(written.contains("\"en\""));
    }

    #[test]
    fn a_get_lite_serves_a_no_script_status_table() {
        let written = handle_request(b"GET /lite HTTP/1.1\r\n\r\n", Arc::new(Auth::new("s")));
        assert!(written.contains("200 OK"));
        assert!(written.contains("text/html"));
        assert!(written.contains("http-equiv=\"refresh\""));
        assert!(!written.contains("<script"));
    }

    #[test]
    fn a_challenge_then_confirm_pairs_over_http() {
        use pamoja_session::{hkdf_sha256, hmac_sha256};

        let auth = Arc::new(Auth::new("s3cret"));

        // The challenge returns a session id and nonce as JSON.
        let challenge = handle_request(b"GET /pair/challenge HTTP/1.1\r\n\r\n", Arc::clone(&auth));
        assert!(challenge.contains("200 OK"));
        let body = challenge.split("\r\n\r\n").nth(1).expect("body");
        let session_id = field(body, "sessionId");
        let nonce = field(body, "nonce");

        // The client derives the key from the known secret and the nonce, then proves it.
        let mut key = [0u8; 32];
        hkdf_sha256(
            nonce.as_bytes(),
            b"s3cret",
            b"pamoja/dashboard/cmd v1",
            &mut key,
        );
        let mac = hmac_sha256(&key, format!("confirm\n{session_id}").as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let confirm_body = format!(r#"{{"sessionId":"{session_id}","mac":"{mac}"}}"#);
        let request = format!(
            "POST /pair/confirm HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            confirm_body.len(),
            confirm_body
        );
        let confirm = handle_request(request.as_bytes(), Arc::clone(&auth));
        assert!(confirm.contains("200 OK"), "confirm response: {confirm}");
    }

    // Pulls a string field out of a small flat JSON object.
    fn field(json: &str, key: &str) -> String {
        let needle = format!("\"{key}\":\"");
        let start = json.find(&needle).expect("key present") + needle.len();
        let rest = &json[start..];
        rest[..rest.find('"').expect("closing quote")].to_owned()
    }

    #[test]
    fn an_asset_is_gzipped_when_the_client_accepts_it() {
        use flate2::read::GzDecoder;
        use std::io::Read as _;

        // The page shell at `/` is present in every tier, so this stays tier-agnostic.
        let mut conn = MemConn::new(b"GET / HTTP/1.1\r\nAccept-Encoding: gzip, deflate\r\n\r\n");
        handle(
            &mut conn,
            Arc::new(Mutex::new(Mock::new(Scenario::Normal))),
            Assets::Embedded,
            Duration::from_millis(0),
            Arc::new(Auth::new("secret")),
            None,
        )
        .expect("handled");

        let split = conn
            .output
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("headers end")
            + 4;
        let head = String::from_utf8_lossy(&conn.output[..split]);
        assert!(head.contains("Content-Encoding: gzip"), "head: {head}");

        let mut decoded = Vec::new();
        GzDecoder::new(&conn.output[split..])
            .read_to_end(&mut decoded)
            .expect("gunzip");
        let (_, original) = Assets::Embedded.get("/").expect("asset");
        assert_eq!(decoded, original, "gunzipped body matches the source asset");
    }

    #[test]
    fn an_asset_is_identity_without_accept_encoding() {
        let written = handle_request(b"GET / HTTP/1.1\r\n\r\n", Arc::new(Auth::new("s")));
        assert!(written.contains("200 OK"));
        assert!(!written.contains("Content-Encoding"));
    }

    // An in-memory connection: reads from a fixed request, collects the response.
    struct MemConn {
        input: std::io::Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl MemConn {
        fn new(request: &[u8]) -> Self {
            Self {
                input: std::io::Cursor::new(request.to_vec()),
                output: Vec::new(),
            }
        }
    }

    impl Read for MemConn {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for MemConn {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
