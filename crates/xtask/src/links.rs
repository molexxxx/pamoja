//! Fetch every source the hardware reference cites and fail on anything that is not 200.
//!
//! The page's claim is that each figure comes from the manufacturer's own document or the
//! standard that defines it, so a link that has rotted takes the claim with it. `curl` does
//! the fetching: it is present on the runners and on the development host, and it keeps the
//! task runner free of an HTTP dependency.
//!
//! A few vendors' sites hold a scripted connection open without answering, or refuse it
//! outright, whatever agent string it carries. An entry may say so in `manual_check`, with
//! the reason; those sources are listed for a person to open rather than fetched, and are
//! never counted as failures.

use std::path::Path;
use std::process::{Command, ExitCode};

use crate::hardware::Hardware;

// Vendor and standards-body sites routinely refuse a request with no user agent.
const AGENT: &str =
    "Mozilla/5.0 (compatible; pamoja-link-check/1.0; +https://github.com/molexxxx/pamoja)";

/// Check every hardware source resolves.
///
/// # Arguments
///
/// * `root` - the repository root, holding `docs/hardware.toml`.
///
/// # Returns
///
/// Success when every source returned 200.
pub fn run(root: &Path) -> ExitCode {
    let hardware = match Hardware::load(root) {
        Ok(hardware) => hardware,
        Err(message) => {
            eprintln!("xtask links: {message}");
            return ExitCode::FAILURE;
        }
    };

    let sources = hardware.sources();
    println!("checking {} sources\n", sources.len());

    let mut failed = Vec::new();
    let mut by_hand = Vec::new();
    for (key, name, url, manual) in sources {
        if !manual.is_empty() {
            println!("  hand {key:<16} {name}");
            by_hand.push((key, url, manual));
            continue;
        }
        match status(url) {
            Ok(200) => println!("  200  {key:<16} {name}"),
            Ok(code) => {
                println!("  {code}  {key:<16} {url}");
                failed.push((key, url, code.to_string()));
            }
            Err(err) => {
                println!("  ---  {key:<16} {url}");
                failed.push((key, url, err));
            }
        }
    }

    if !by_hand.is_empty() {
        println!(
            "\n{} source(s) are checked by a person, because the site refuses scripted clients:",
            by_hand.len()
        );
        for (key, url, why) in &by_hand {
            println!("  {key}: {url}\n      {why}");
        }
    }

    if failed.is_empty() {
        println!("\nevery fetched source resolves");
        return ExitCode::SUCCESS;
    }

    eprintln!("\n{} source(s) did not return 200:", failed.len());
    for (key, url, why) in &failed {
        eprintln!("  {key}: {url} ({why})");
    }
    eprintln!(
        "\nReplace the link with the document's current home on the same vendor's or standards \
         body's own domain. Do not swap in a mirror or a reseller."
    );
    ExitCode::FAILURE
}

// The HTTP status curl observes, following redirects. The body is discarded, but it is still
// requested: a HEAD is refused by enough vendor sites to be useless here.
fn status(url: &str) -> Result<u32, String> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "45",
            "--retry",
            "3",
            "--retry-delay",
            "5",
            "--user-agent",
            AGENT,
            "--header",
            "Accept: text/html,application/xhtml+xml,application/pdf;q=0.9,*/*;q=0.8",
            "--header",
            "Accept-Language: en-US,en;q=0.9",
            "--output",
            null_device(),
            "--write-out",
            "%{http_code}",
            url,
        ])
        .output()
        .map_err(|err| format!("running curl: {err}"))?;

    let code = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    code.parse().map_err(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if stderr.is_empty() {
            "no response".to_owned()
        } else {
            stderr
        }
    })
}

const fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}
