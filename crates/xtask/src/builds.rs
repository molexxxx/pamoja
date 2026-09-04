//! What a build carries: the crates a named feature set of the `pamoja` crate compiles.
//!
//! The framework's design claim is that a build pays only for the capabilities it names.
//! This measures that claim instead of asserting it. Each row below is a feature set a
//! consumer would plausibly ask for, resolved with `cargo tree`, which applies the same
//! feature unification the compiler does; the resolve graph in `cargo metadata` does not,
//! and reports optional crates a build never compiles.
//!
//! The counts are resolved for one fixed target, [`TARGET`], because a dependency graph is
//! platform-specific: the same feature set resolves to 108 crates on Windows and 107 on
//! Linux, which would make a committed table fail its own check on another machine. With
//! the target pinned the counts are lockfile-derived, identical everywhere, and can be
//! committed. `cargo xtask docs` renders them into a `<!-- table: builds -->` region and
//! `cargo xtask docs --check` fails when they drift, so the published numbers cannot rot.
//! The compiled engine each binding ships is measured separately by
//! [`report`](crate::builds::report), because an artifact's size depends on the platform
//! and toolchain that produced it.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// The target the counts are resolved for. A dependency graph varies by platform, so the
/// table pins one rather than reporting whatever the machine running it happens to be. The
/// target does not need to be installed; this is resolution, not compilation.
const TARGET: &str = "x86_64-unknown-linux-gnu";

/// One build the table reports: what a consumer asks for, and what it costs.
struct Build {
    /// How the row is labelled.
    label: &'static str,
    /// What the consumer writes, shown verbatim in the table.
    invocation: &'static str,
    /// The features to resolve with; empty selects the default set.
    features: &'static str,
    /// Whether the default features are on.
    default: bool,
}

/// The builds the table reports, widest first, so the progression is visible.
const BUILDS: &[Build] = &[
    Build {
        label: "Every capability",
        invocation: "cargo add pamoja",
        features: "",
        default: true,
    },
    Build {
        label: "Codecs and identity",
        invocation: "--features codec,security",
        features: "std,codec,security",
        default: false,
    },
    Build {
        label: "Field I/O",
        invocation: "--features field-io",
        features: "std,field-io",
        default: false,
    },
    Build {
        label: "One capability",
        invocation: "--features modbus",
        features: "std,modbus",
        default: false,
    },
    Build {
        label: "Bare metal, no `std`",
        invocation: "--features modbus,sensors,lora",
        features: "modbus,sensors,lora",
        default: false,
    },
];

/// What one feature set compiles, split into the framework's own crates and everything else.
struct Counts {
    /// Crates from this workspace.
    pamoja: usize,
    /// Crates from outside it.
    external: usize,
}

impl Counts {
    fn total(&self) -> usize {
        self.pamoja + self.external
    }
}

/// Render the `builds` table: what each named feature set compiles.
///
/// # Arguments
///
/// * `root` - the repository root, used as the working directory for `cargo tree`.
///
/// # Returns
///
/// The Markdown table body, without a trailing newline.
///
/// # Errors
///
/// Returns the reason when `cargo tree` cannot be run or its output cannot be read.
pub fn table(root: &Path) -> Result<String, String> {
    let mut out = String::from(
        "| Build | What you write | Crates compiled | From this workspace | External |\n| --- | --- | --- | --- | --- |\n",
    );
    for build in BUILDS {
        let counts = resolve(root, build)?;
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            build.label,
            build.invocation,
            counts.total(),
            counts.pamoja,
            counts.external,
        ));
    }
    Ok(out.trim_end().to_owned())
}

/// Print the table and, when the artifacts have been built, the size of the compiled engine
/// each binding ships.
///
/// # Arguments
///
/// * `root` - the repository root.
///
/// # Returns
///
/// Success when every feature set resolved.
///
/// # Errors
///
/// Returns the reason when a feature set could not be resolved.
pub fn report(root: &Path) -> Result<String, String> {
    let mut out = format!(
        "{}\n\nCompiled engine, as built on this machine:\n",
        table(root)?
    );
    let mut any = false;
    for (label, path) in engine_artifacts() {
        let full = root.join(path);
        if let Ok(meta) = std::fs::metadata(&full) {
            any = true;
            out.push_str(&format!(
                "  {label}: {:.2} MB ({path})\n",
                meta.len() as f64 / (1024.0 * 1024.0)
            ));
        }
    }
    if !any {
        out.push_str("  none built yet; `cargo build --release -p pamoja-ffi` and the binding builds produce them\n");
    }
    Ok(out)
}

// Where each binding's compiled engine lands in a release build. A missing one is skipped,
// since a machine rarely has all three built at once.
fn engine_artifacts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("C ABI, used by .NET", "target/release/pamoja_ffi.dll"),
        ("C ABI, used by .NET", "target/release/libpamoja_ffi.so"),
        ("C ABI, used by .NET", "target/release/libpamoja_ffi.dylib"),
    ]
}

// The crates one feature set compiles. `cargo tree` is the source of truth here: it applies
// the same feature unification as a build, so a weak dependency a feature only mentions
// (`pamoja-lora?/std`) is correctly absent. Crates are counted by name and version, because
// two versions of one crate are two compilations.
fn resolve(root: &Path, build: &Build) -> Result<Counts, String> {
    let mut command = Command::new("cargo");
    command.current_dir(root).args([
        "tree", "-p", "pamoja", "--edges", "normal", "--prefix", "none", "--target", TARGET,
    ]);
    if !build.default {
        command.arg("--no-default-features");
    }
    if !build.features.is_empty() {
        command.args(["--features", build.features]);
    }
    let output = command
        .output()
        .map_err(|err| format!("running cargo tree for `{}`: {err}", build.label))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed for `{}`: {}",
            build.label,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|err| {
        format!(
            "cargo tree output for `{}` is not UTF-8: {err}",
            build.label
        )
    })?;
    Ok(count(&text))
}

// Counts the distinct crates in `cargo tree --prefix none` output. Each line is
// `name version [source]`, with a repeated subtree marked `(*)`.
fn count(tree: &str) -> Counts {
    let mut seen = BTreeSet::new();
    for line in tree.lines() {
        let mut words = line.split_whitespace();
        let (Some(name), Some(version)) = (words.next(), words.next()) else {
            continue;
        };
        seen.insert((name.to_owned(), version.to_owned()));
    }
    let pamoja = seen
        .iter()
        .filter(|(name, _)| name == "pamoja" || name.starts_with("pamoja-"))
        .count();
    Counts {
        pamoja,
        external: seen.len() - pamoja,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_distinct_crates_by_name_and_version() {
        let tree = "pamoja v0.1.14 (/w/crates/pamoja)\npamoja-core v0.1.14 (/w/crates/pamoja-core)\nsyn v2.0.119\nsyn v3.0.4\nsyn v2.0.119 (*)\n";
        let counts = count(tree);
        assert_eq!(counts.pamoja, 2);
        assert_eq!(
            counts.external, 2,
            "two versions of syn are two compilations"
        );
        assert_eq!(counts.total(), 4);
    }

    #[test]
    fn ignores_blank_and_partial_lines() {
        let counts = count("\npamoja v0.1.14\nnoversion\n\n");
        assert_eq!(counts.total(), 1);
    }

    #[test]
    fn every_build_names_a_capability_or_the_default_set() {
        for build in BUILDS {
            assert!(
                build.default || !build.features.is_empty(),
                "{} selects nothing",
                build.label
            );
        }
    }
}
