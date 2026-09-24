//! `pamoja-node` run as a user runs it: a wiring file on disk naming a shipped profile, with
//! a replayed or a simulated part in place of the hardware.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::json;

/// Returns the shipped profile the wiring files run.
fn catalog(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../profiles")
        .join(name)
}

/// Writes a wiring file into a folder of its own and runs the node on it.
fn run(name: &str, wiring: serde_json::Value, args: &[&str]) -> Output {
    let folder = std::env::temp_dir().join(format!("pamoja-node-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder for the wiring");
    let path = folder.join("site.json");
    std::fs::write(&path, serde_json::to_string_pretty(&wiring).unwrap())
        .expect("the wiring is written");
    let output = Command::new(env!("CARGO_BIN_EXE_pamoja-node"))
        .arg(&path)
        .args(args)
        .output()
        .expect("the runner starts");
    let _ = std::fs::remove_dir_all(&folder);
    output
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn a_replayed_morning_runs_the_brooder_profile_to_the_end() {
    let output = run(
        "replay",
        json!({
            "site": "coop-2",
            "profile": catalog("brooder-heater.json"),
            "sensor": { "part": "replay", "readings": [27.5, 31.8, 32.6] },
            "output": { "print": "heat lamp" },
            "link": { "print": true }
        }),
        &["--fast"],
    );
    let stdout = text(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        text(&output.stderr)
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "coop-2 runs brooder-heater: temperature in celsius from 3 replayed readings, the heat lamp printed, reporting on poultry/brooder/temperature over print",
            "-> poultry/brooder/temperature 27.5",
            "27.5 celsius, heat lamp on, alert OutOfRange",
            "-> poultry/brooder/temperature 31.8",
            "31.8 celsius, heat lamp on",
            "-> poultry/brooder/temperature 32.6",
            "32.6 celsius, heat lamp off",
            "the sensor has no more readings",
        ]
    );
}

#[test]
fn a_simulated_part_is_read_through_its_driver() {
    let output = run(
        "sim",
        json!({
            "site": "coop-3",
            "profile": catalog("brooder-heater.json"),
            "sensor": { "part": "bme280", "bus": "sim", "offset": 0.06 },
            "output": { "print": "heat lamp" },
            "link": { "print": true }
        }),
        &["--ticks", "2", "--fast"],
    );
    let stdout = text(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        text(&output.stderr)
    );
    assert!(stdout.contains("from a bme280 at 0x76 on sim"), "{stdout}");
    assert_eq!(
        stdout
            .matches("20.5 celsius, heat lamp on, alert OutOfRange")
            .count(),
        2,
        "the simulated part compensates to 20.44 C, and the wiring's offset adds 0.06: {stdout}"
    );
}

#[test]
fn a_wiring_that_cannot_run_the_profile_is_refused_before_any_part_is_opened() {
    let output = run(
        "soil",
        json!({
            "site": "bed-4",
            "profile": catalog("soil-moisture-valve.json"),
            "sensor": { "part": "bme280", "bus": "/dev/i2c-1" },
            "output": { "gpio": "/dev/gpiochip0", "line": 17 },
            "link": { "mqtt": "localhost" }
        }),
        &["--check"],
    );
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("reads soil_moisture, and a bme280 measures temperature, relative_humidity, or pressure"),
        "{stderr}"
    );

    let typo = run(
        "typo",
        json!({
            "site": "coop-2",
            "profile": catalog("brooder-heater.json"),
            "sensor": { "part": "replay", "readings": [30.0] },
            "outptu": { "print": "heat lamp" },
            "link": { "print": true }
        }),
        &["--check"],
    );
    assert!(!typo.status.success());
    assert!(
        text(&typo.stderr).contains("unknown field `outptu`, did you mean `output`?"),
        "{}",
        text(&typo.stderr)
    );
}

#[test]
fn check_says_what_would_run_without_opening_anything() {
    let output = run(
        "check",
        json!({
            "site": "coop-2",
            "profile": catalog("brooder-heater.json"),
            "sensor": { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x77" },
            "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
            "link": { "mqtt": "192.168.1.10" }
        }),
        &["--check"],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout).trim(),
        "coop-2 runs brooder-heater: temperature in celsius from a bme280 at 0x77 on /dev/i2c-1, the output on /dev/gpiochip0 line 17, reporting on poultry/brooder/temperature over mqtt 192.168.1.10:1883"
    );
}

#[test]
fn the_command_line_is_refused_with_its_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_pamoja-node"))
        .arg("--nonsense")
        .output()
        .expect("the runner starts");
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("usage: pamoja-node <wiring.json>"));
}
