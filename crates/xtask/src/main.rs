//! Workspace task runner for pamoja.
//!
//! Run with `cargo xtask <task>`. Most tasks are placeholders that document the
//! intended build automation; `release` is implemented and publishes the
//! workspace crates to crates.io in dependency order, riding out the registry's
//! new-crate rate limit so a first-time publish of the whole workspace completes
//! in a single run.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::Duration;

mod docs;
mod footprint;
mod i18n;

/// The tasks xtask knows about, each paired with a one-line description.
const TASKS: &[(&str, &str)] = &[
    (
        "release",
        "publish the workspace crates to crates.io in dependency order",
    ),
    (
        "ros",
        "build the ROS 2 + Zenoh dev container and run the bridge tests inside it",
    ),
    (
        "sitl",
        "build an autopilot SITL image and run the MAVLink interop test (sitl [ardupilot|px4|all])",
    ),
    (
        "dashboard",
        "run the local-first dashboard dev server with mock data (dashboard dev [scenario])",
    ),
    (
        "docs",
        "regenerate every crate README and the API index from rustdoc (docs [--check])",
    ),
];

/// The tag for the ROS 2 + Zenoh dev image built from `.devcontainer/Dockerfile`.
const ROS_IMAGE: &str = "pamoja-ros2-dev";

/// Publishable crates in dependency order: a crate never appears before one it
/// depends on, so each is resolvable on crates.io by the time a dependent is
/// published. `xtask` and `examples` are not published and are absent.
const RELEASE_ORDER: &[&str] = &[
    "pamoja-core",
    "pamoja-codec",
    "pamoja-mqtt",
    "pamoja-coap",
    "pamoja-ffi",
    "pamoja-sync",
    "pamoja-loopback",
    "pamoja-bus",
    "pamoja-ladder",
    "pamoja-kit",
    "pamoja-power",
    "pamoja-profile",
    "pamoja-sim",
    "pamoja-security",
    "pamoja-session",
    "pamoja-sensors",
    "pamoja-actuators",
    "pamoja-audit",
    "pamoja-telemetry",
    // Depends on telemetry, profile, power, and session, all published above.
    "pamoja-dashboard",
    "pamoja-lora",
    "pamoja-modbus",
    "pamoja-mesh",
    "pamoja-lorawan",
    "pamoja-mavlink",
    "pamoja-routing",
    "pamoja-can",
    "pamoja-serial",
    "pamoja-gpio",
    "pamoja-zenoh",
    "pamoja-ros2",
];

/// Seconds to wait before retrying a crate that crates.io throttled. The
/// new-crate limit refills one crate every ten minutes, so the default waits a
/// little past that. Override with `PAMOJA_RELEASE_RETRY_SECS`.
const DEFAULT_RETRY_SECS: u64 = 660;

/// Attempts per crate before giving up, so a persistent failure cannot loop
/// forever.
const MAX_ATTEMPTS: u32 = 12;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(task) = args.next() else {
        help();
        return ExitCode::SUCCESS;
    };

    if task == "release" {
        return release(&args.collect::<Vec<_>>());
    }

    if task == "ros" {
        return ros(&args.collect::<Vec<_>>());
    }

    if task == "sitl" {
        return sitl(&args.collect::<Vec<_>>());
    }

    if task == "dashboard" {
        return dashboard(&args.collect::<Vec<_>>());
    }

    if task == "docs" {
        return docs::run(&args.collect::<Vec<_>>());
    }

    eprintln!("unknown task: {task}\n");
    help();
    ExitCode::FAILURE
}

/// Publish every crate in `RELEASE_ORDER`. A version already on crates.io is
/// skipped, and a crate throttled by the new-crate rate limit is retried after a
/// wait, so the whole workspace publishes in one run even though new crates are
/// limited to one every ten minutes. Pass `--dry-run` to package and verify each
/// crate without uploading. Reads the token from `CARGO_REGISTRY_TOKEN`.
fn release(args: &[String]) -> ExitCode {
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let retry_secs = std::env::var("PAMOJA_RELEASE_RETRY_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RETRY_SECS);

    let count = RELEASE_ORDER.len();
    if dry_run {
        println!("xtask release: dry run, packaging {count} crates without uploading\n");
    } else {
        println!("xtask release: publishing {count} crates to crates.io\n");
    }

    for crate_name in RELEASE_ORDER {
        if !publish(crate_name, dry_run, retry_secs) {
            return ExitCode::FAILURE;
        }
    }

    println!("\nxtask release: all {count} crates are published");
    ExitCode::SUCCESS
}

/// Publish a single crate, retrying while crates.io reports its rate limit.
/// Returns `true` once the crate is published or already present, `false` on a
/// real failure.
fn publish(crate_name: &str, dry_run: bool, retry_secs: u64) -> bool {
    for attempt in 1..=MAX_ATTEMPTS {
        println!("==> {crate_name} (attempt {attempt})");

        let mut cmd = Command::new("cargo");
        cmd.arg("publish").arg("-p").arg(crate_name);
        if dry_run {
            cmd.arg("--dry-run").arg("--allow-dirty");
        }

        let output = match cmd.output() {
            Ok(output) => output,
            Err(err) => {
                eprintln!("could not run cargo for {crate_name}: {err}");
                return false;
            }
        };

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        print!("{combined}");

        if output.status.success() {
            println!("published {crate_name}\n");
            return true;
        }

        let report = combined.to_lowercase();
        if report.contains("already uploaded") || report.contains("already exists") {
            println!("skipping {crate_name}: this version is already on crates.io\n");
            return true;
        }

        if report.contains("rate limit") || report.contains("too many") || report.contains("429") {
            println!(
                "{crate_name} hit the crates.io rate limit; waiting {retry_secs}s before retry\n"
            );
            sleep(Duration::from_secs(retry_secs));
            continue;
        }

        eprintln!("failed to publish {crate_name}");
        return false;
    }

    eprintln!("gave up on {crate_name} after {MAX_ATTEMPTS} attempts");
    false
}

/// Build the ROS 2 + Zenoh dev image and run the bridge crates' tests inside it. The host has no
/// ROS 2, so this is how `pamoja-ros2` and `pamoja-zenoh` are exercised on a real ROS 2 + Zenoh
/// install. Any extra `args` are appended to the in-container `cargo test`, for example
/// `--features bridge` once the live layer lands. Requires Docker Desktop.
fn ros(args: &[String]) -> ExitCode {
    let repo = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("xtask ros: cannot determine the working directory: {err}");
            return ExitCode::FAILURE;
        }
    };

    if !run(Command::new("docker").arg("--version")) {
        eprintln!("xtask ros: Docker is required (Docker Desktop); install it and retry.");
        return ExitCode::FAILURE;
    }

    println!("xtask ros: building the {ROS_IMAGE} image from .devcontainer\n");
    let built = run(Command::new("docker").args([
        "build",
        "-t",
        ROS_IMAGE,
        "-f",
        ".devcontainer/Dockerfile",
        ".devcontainer",
    ]));
    if !built {
        eprintln!("xtask ros: image build failed");
        return ExitCode::FAILURE;
    }

    // With no extra args, run the pure-logic tests and then both live feature suites; with extra
    // args, run exactly those against the two crates so the task is also a general escape hatch.
    let tests = if args.is_empty() {
        // The pure-logic tests, then each live feature suite, then the rmw_zenoh cross-interop
        // proof, which needs the Zenoh RMW and peer discovery and so is ignored by default.
        "cargo test -p pamoja-zenoh -p pamoja-ros2; \
         cargo test -p pamoja-zenoh --features runtime; \
         cargo test -p pamoja-ros2 --features bridge; \
         RMW_IMPLEMENTATION=rmw_zenoh_cpp ZENOH_ROUTER_CHECK_ATTEMPTS=-1 \
         ZENOH_CONFIG_OVERRIDE='scouting/multicast/enabled=true' \
         cargo test -p pamoja-ros2 --features bridge ros2_twist_is_received_over_zenoh -- --ignored"
            .to_string()
    } else {
        format!(
            "cargo test -p pamoja-zenoh -p pamoja-ros2 {}",
            args.join(" ")
        )
    };
    // Source ROS 2 so the bridge layer can find the client libraries, confirm the Zenoh RMW is
    // installed for the live path, then run the tests.
    let script = format!(
        "set -e; \
         source /opt/ros/jazzy/setup.bash; \
         echo \"ROS_DISTRO=$ROS_DISTRO\"; rustc --version; \
         (ros2 pkg list | grep -q rmw_zenoh_cpp && echo 'rmw_zenoh: present') \
            || echo 'rmw_zenoh: MISSING'; \
         {tests}"
    );
    let mount = format!("{}:/work", repo.display());

    println!("\nxtask ros: running the bridge tests in the container\n");
    // Persistent volumes cache the cargo registry and the Linux build, so repeat runs are fast and
    // the container's artifacts never collide with the Windows `target/`.
    let passed = run(Command::new("docker").args([
        "run",
        "--rm",
        "-v",
        &mount,
        "-v",
        "pamoja-cargo-registry:/usr/local/cargo/registry",
        "-v",
        "pamoja-cargo-git:/usr/local/cargo/git",
        "-v",
        "pamoja-ros-target:/tmp/target",
        "-w",
        "/work",
        ROS_IMAGE,
        "bash",
        "-lc",
        &script,
    ]));
    if passed {
        ExitCode::SUCCESS
    } else {
        eprintln!("xtask ros: tests failed");
        ExitCode::FAILURE
    }
}

/// The autopilots `sitl` can build and test against, each with a `sitl/<target>.Dockerfile` and
/// a `sitl/run-<target>.sh`.
const SITL_TARGETS: &[&str] = &["ardupilot", "px4"];

/// Build an autopilot SITL image and run the MAVLink interop test inside it. The host has no
/// autopilot, so this is how `pamoja-mavlink` is proven against a real ArduPilot or PX4 flight
/// stack, mirroring the ROS 2 dev-container pattern. `sitl ardupilot`, `sitl px4`, or `sitl all`
/// select the target(s); the default is ArduPilot. Requires Docker Desktop.
fn sitl(args: &[String]) -> ExitCode {
    let which = args.first().map(String::as_str).unwrap_or("ardupilot");
    let targets: Vec<&str> = match which {
        "all" => SITL_TARGETS.to_vec(),
        "ardupilot" => vec!["ardupilot"],
        "px4" => vec!["px4"],
        other => {
            eprintln!("xtask sitl: unknown target {other}; use ardupilot, px4, or all");
            return ExitCode::FAILURE;
        }
    };

    if !run(Command::new("docker").arg("--version")) {
        eprintln!("xtask sitl: Docker is required (Docker Desktop); install it and retry.");
        return ExitCode::FAILURE;
    }

    let repo = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("xtask sitl: cannot determine the working directory: {err}");
            return ExitCode::FAILURE;
        }
    };

    for target in targets {
        if !run_sitl(&repo, target) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Build one autopilot's SITL image and run the interop test inside it. Persistent volumes cache
/// the cargo registry and the Linux build so repeat runs are fast and the container's artifacts
/// never collide with the Windows `target/`.
fn run_sitl(repo: &Path, target: &str) -> bool {
    let image = format!("pamoja-sitl-{target}");
    let dockerfile = format!("sitl/{target}.Dockerfile");

    println!("xtask sitl: building the {image} image\n");
    if !run(Command::new("docker").args(["build", "-t", &image, "-f", &dockerfile, "sitl"])) {
        eprintln!("xtask sitl: building {image} failed");
        return false;
    }

    let mount = format!("{}:/work", repo.display());
    let script = format!("sitl/run-{target}.sh");
    println!("\nxtask sitl: launching {target} SITL and running the interop test\n");
    let passed = run(Command::new("docker").args([
        "run",
        "--rm",
        "-v",
        &mount,
        "-v",
        "pamoja-cargo-registry:/usr/local/cargo/registry",
        "-v",
        "pamoja-cargo-git:/usr/local/cargo/git",
        "-v",
        "pamoja-sitl-target:/tmp/target",
        "-w",
        "/work",
        &image,
        "bash",
        &script,
    ]));
    if !passed {
        eprintln!("xtask sitl: the {target} interop test failed");
    }
    passed
}

/// Run a `dashboard` subcommand: `i18n` validates the locale bundles, `footprint` checks
/// the gzipped transfer budget, anything else runs the mock-backed dev server.
///
/// `cargo xtask dashboard i18n` checks the per-locale JSON bundles (key, placeholder, and
/// metadata parity). `cargo xtask dashboard footprint` sums the gzipped page-load bundle and
/// enforces each tier's budget (add `--tier <a|b|c>` for one tier). `cargo xtask dashboard
/// dev alarm` serves the alarm scenario; a
/// leading `dev` word is optional and dropped, and any other arguments (a scenario key,
/// `--addr`, `--embedded`, `--interval-ms`) pass straight through to the dev binary.
fn dashboard(args: &[String]) -> ExitCode {
    if args.first().map(String::as_str) == Some("i18n") {
        return i18n::run(&args[1..]);
    }

    if args.first().map(String::as_str) == Some("footprint") {
        return footprint::run(&args[1..]);
    }

    let forwarded: Vec<&String> = args
        .iter()
        .skip_while(|arg| arg.as_str() == "dev")
        .collect();

    // The demo fleet is off by default, so the dev server opts into it explicitly.
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "-p",
        "pamoja-dashboard",
        "--features",
        "mock",
        "--example",
        "dev",
        "--",
    ]);
    cmd.args(&forwarded);

    if run(&mut cmd) {
        ExitCode::SUCCESS
    } else {
        eprintln!("xtask dashboard: dev server exited with an error");
        ExitCode::FAILURE
    }
}

/// Run a command, streaming its output, and report whether it succeeded.
fn run(command: &mut Command) -> bool {
    match command.status() {
        Ok(status) => status.success(),
        Err(err) => {
            eprintln!("could not run {:?}: {err}", command.get_program());
            false
        }
    }
}

fn help() {
    println!("pamoja xtask");
    println!("usage: cargo xtask <task>\n");
    println!("tasks:");
    for (name, description) in TASKS {
        println!("  {name:<10} {description}");
    }
}
