//! The hardware-free dashboard dev server.
//!
//! Runs the real dashboard backed by the [`Mock`], so the whole UI, every locale, and
//! every alarm state can be built and debugged with no hardware. Assets are read from
//! the crate's `web/` directory on each request by default, so editing the page and
//! reloading shows the change with no recompile. Run it through `cargo xtask dashboard
//! dev` or directly:
//!
//! ```text
//! cargo run -p pamoja-dashboard --example dev -- alarm --addr 0.0.0.0:8787
//! ```
//!
//! Arguments:
//! - a scenario key (`normal`, `alarm`, `sensor-fault`, `low-battery`, `link-lost`,
//!   `cold-start`); defaults to `normal`.
//! - `--addr <host:port>` to change the bind address (default `127.0.0.1:8787`).
//! - `--embedded` to serve the baked-in bundle instead of the live `web/` directory.
//! - `--interval-ms <n>` to change the live-update cadence (default 1000).

use std::path::PathBuf;
use std::time::Duration;

use pamoja_dashboard::{Assets, Auth, Mock, Scenario, Server};

fn main() -> std::process::ExitCode {
    let mut scenario = Scenario::Normal;
    let mut addr = "127.0.0.1:8787".to_owned();
    let mut embedded = false;
    let mut interval = Duration::from_millis(2000);

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--addr" => {
                if let Some(value) = args.next() {
                    addr = value;
                }
            }
            "--embedded" => embedded = true,
            "--interval-ms" => {
                if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                    interval = Duration::from_millis(value);
                }
            }
            "-h" | "--help" => {
                print_help();
                return std::process::ExitCode::SUCCESS;
            }
            key => match Scenario::from_key(key) {
                Some(parsed) => scenario = parsed,
                None => {
                    eprintln!("unknown argument or scenario: {key}\n");
                    print_help();
                    return std::process::ExitCode::FAILURE;
                }
            },
        }
    }

    let assets = if embedded {
        Assets::Embedded
    } else {
        Assets::Dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web"))
    };

    println!(
        "pamoja-dashboard dev: scenario={}, assets={}",
        scenario.key(),
        if embedded {
            "embedded"
        } else {
            "web/ (hot reload)"
        }
    );
    println!("switch scenarios, locale, theme, and tier live from the page or with ?scenario=&locale=&theme=&tier=");

    // The field device shows this on its own screen or as a QR; the dev server prints it
    // to stand in for that. Enter it in the dashboard to unlock control.
    let secret = Auth::generate_secret();
    println!(
        "pairing code (unlock control with this): {}",
        group(&secret)
    );

    let server = Server::new(Mock::new(scenario), assets)
        .with_catalog(Mock::catalog())
        .with_push_interval(interval)
        .with_pairing_secret(secret);
    match server.run(&addr) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("pamoja-dashboard dev: could not serve on {addr}: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

// Groups a hex code into dash-separated quads so it is easier to read and type. The
// page strips the dashes again when deriving the key.
fn group(code: &str) -> String {
    code.as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-")
}

fn print_help() {
    println!("pamoja-dashboard dev server");
    println!("usage: cargo run -p pamoja-dashboard --example dev -- [scenario] [--addr host:port] [--embedded] [--interval-ms n]");
    println!("\nscenarios:");
    for scenario in Scenario::ALL {
        println!("  {}", scenario.key());
    }
}
