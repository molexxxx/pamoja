//! Regenerates the committed C header from the crate's `extern "C"` surface.
//!
//! `cbindgen` parses this crate and writes `include/pamoja.h`. The header is
//! checked into the tree and drift-checked in CI so it can never fall behind the
//! Rust source.
//!
//! The header is only refreshed when the crate is built from its own workspace
//! checkout, never when it is consumed as a published dependency, where the source
//! lives in a registry cache that must not be mutated. The write is also
//! best-effort, so a read-only source tree (for example on docs.rs) cannot fail
//! the build. CI is the gate that enforces a fresh, committed header.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/actuators.rs");
    println!("cargo:rerun-if-changed=src/audit.rs");
    println!("cargo:rerun-if-changed=src/bus.rs");
    println!("cargo:rerun-if-changed=src/can.rs");
    println!("cargo:rerun-if-changed=src/coap.rs");
    println!("cargo:rerun-if-changed=src/codec.rs");
    println!("cargo:rerun-if-changed=src/gpio.rs");
    println!("cargo:rerun-if-changed=src/kit.rs");
    println!("cargo:rerun-if-changed=src/ladder.rs");
    println!("cargo:rerun-if-changed=src/loopback.rs");
    println!("cargo:rerun-if-changed=src/lora.rs");
    println!("cargo:rerun-if-changed=src/lora_region.rs");
    println!("cargo:rerun-if-changed=src/lorawan.rs");
    println!("cargo:rerun-if-changed=src/mavlink.rs");
    println!("cargo:rerun-if-changed=src/mesh.rs");
    println!("cargo:rerun-if-changed=src/modbus.rs");
    println!("cargo:rerun-if-changed=src/mqtt.rs");
    println!("cargo:rerun-if-changed=src/power.rs");
    println!("cargo:rerun-if-changed=src/profile.rs");
    println!("cargo:rerun-if-changed=src/ros2.rs");
    println!("cargo:rerun-if-changed=src/routing.rs");
    println!("cargo:rerun-if-changed=src/security.rs");
    println!("cargo:rerun-if-changed=src/session.rs");
    println!("cargo:rerun-if-changed=src/sim.rs");
    println!("cargo:rerun-if-changed=src/sensors.rs");
    println!("cargo:rerun-if-changed=src/serial.rs");
    println!("cargo:rerun-if-changed=src/sync.rs");
    println!("cargo:rerun-if-changed=src/telemetry.rs");
    println!("cargo:rerun-if-changed=src/transport.rs");
    println!("cargo:rerun-if-changed=src/update.rs");
    println!("cargo:rerun-if-changed=src/zenoh.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Skip entirely unless this is the workspace checkout that owns the committed
    // header. Cargo does not pass CARGO_PRIMARY_PACKAGE to a build script, only to
    // the rustc invocations it drives, so the workspace root is what distinguishes
    // a source-tree build from a build of the published crate.
    if !in_workspace_checkout(&crate_dir) {
        return;
    }

    let config =
        cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).expect("read cbindgen.toml");

    let bindings = match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => bindings,
        Err(error) => {
            println!("cargo:warning=cbindgen could not generate the header: {error}");
            return;
        }
    };

    let mut rendered = Vec::new();
    bindings.write(&mut rendered);

    // Write only when the contents change, and treat any IO failure (such as a
    // read-only source tree) as a warning rather than a hard error.
    let header_path = crate_dir.join("include").join("pamoja.h");
    let unchanged = fs::read(&header_path)
        .map(|existing| existing == rendered)
        .unwrap_or(false);
    if unchanged {
        return;
    }
    if let Err(error) = fs::create_dir_all(header_path.parent().expect("include directory"))
        .and_then(|()| fs::write(&header_path, &rendered))
    {
        println!(
            "cargo:warning=could not write {}: {error}",
            header_path.display()
        );
    }
}

/// Reports whether `crate_dir` sits inside the pamoja workspace checkout.
///
/// # Arguments
///
/// * `crate_dir` - this crate's manifest directory.
///
/// # Returns
///
/// `true` when the grandparent directory holds the workspace manifest, which is
/// the layout of the git checkout and never that of an extracted registry package.
fn in_workspace_checkout(crate_dir: &Path) -> bool {
    let Some(root) = crate_dir.parent().and_then(Path::parent) else {
        return false;
    };
    fs::read_to_string(root.join("Cargo.toml"))
        .map(|manifest| manifest.contains("[workspace]"))
        .unwrap_or(false)
}
