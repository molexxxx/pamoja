//! The crates.io release: the publish order derived from `cargo metadata`, a
//! whole-workspace dry run, and the publish loop that rides out the registry's
//! new-crate rate limit.

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::Duration;

/// Seconds to wait before retrying a crate that crates.io throttled. The
/// new-crate limit refills one crate every ten minutes, so the default waits a
/// little past that. Override with `PAMOJA_RELEASE_RETRY_SECS`.
const DEFAULT_RETRY_SECS: u64 = 660;

/// Attempts per crate before giving up, so a persistent failure cannot loop
/// forever.
const MAX_ATTEMPTS: u32 = 12;

/// A workspace member as `cargo metadata` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The crate name.
    pub name: String,
    /// Whether `cargo publish` may upload it (`publish = false` opts out).
    pub publishable: bool,
    /// The workspace members it needs at build time: its normal and build
    /// dependencies, optional ones included, since crates.io resolves them all.
    pub deps: Vec<String>,
}

/// Run `cargo xtask release [--plan | --dry-run]`.
///
/// `--plan` prints the publish order and exits. `--dry-run` packages and verifies
/// every publishable crate in one `cargo publish --workspace --dry-run`, the only
/// dry run that resolves each crate against its unpublished siblings. With no
/// flag, every crate is published in dependency order; a version already on
/// crates.io is skipped, and a crate throttled by the new-crate rate limit is
/// retried after a wait. Reads the token from `CARGO_REGISTRY_TOKEN`.
pub fn run(args: &[String]) -> ExitCode {
    let members = match workspace() {
        Ok(members) => members,
        Err(message) => {
            eprintln!("xtask release: {message}");
            return ExitCode::FAILURE;
        }
    };
    let order = match publish_order(&members) {
        Ok(order) => order,
        Err(message) => {
            eprintln!("xtask release: {message}");
            return ExitCode::FAILURE;
        }
    };
    let count = order.len();

    if args.iter().any(|a| a == "--plan") {
        println!("xtask release: {count} crates publish in this order\n");
        for (index, name) in order.iter().enumerate() {
            println!("{:>3}. {name}", index + 1);
        }
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "--dry-run") {
        println!("xtask release: dry run, packaging {count} crates without uploading\n");
        let mut cmd = Command::new("cargo");
        cmd.args(["publish", "--workspace", "--dry-run", "--allow-dirty"]);
        for member in members.iter().filter(|m| !m.publishable) {
            cmd.arg("--exclude").arg(&member.name);
        }
        return if super::run(&mut cmd) {
            println!("\nxtask release: all {count} crates package cleanly");
            ExitCode::SUCCESS
        } else {
            eprintln!("xtask release: the dry run failed");
            ExitCode::FAILURE
        };
    }

    let retry_secs = std::env::var("PAMOJA_RELEASE_RETRY_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RETRY_SECS);

    println!("xtask release: publishing {count} crates to crates.io\n");
    for name in &order {
        if !publish(name, retry_secs) {
            return ExitCode::FAILURE;
        }
    }

    println!("\nxtask release: all {count} crates are published");
    ExitCode::SUCCESS
}

/// Read the workspace members and their in-workspace dependencies from
/// `cargo metadata`.
///
/// # Errors
///
/// Returns the reason when cargo cannot be run or its output is not the
/// expected shape.
pub fn workspace() -> Result<Vec<Member>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|err| format!("could not run cargo metadata: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("cargo metadata output is not JSON: {err}"))?;
    members_of(&metadata)
}

/// Build the member list from a `cargo metadata --no-deps` document.
fn members_of(metadata: &serde_json::Value) -> Result<Vec<Member>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata output has no packages array")?;
    let names: BTreeSet<&str> = packages
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();

    let mut members = Vec::with_capacity(packages.len());
    for package in packages {
        let name = package["name"]
            .as_str()
            .ok_or("a package in cargo metadata has no name")?;
        // `publish` is null when unrestricted and a list of registries otherwise;
        // an empty list is how `publish = false` is reported.
        let publishable = match &package["publish"] {
            serde_json::Value::Array(registries) => !registries.is_empty(),
            _ => true,
        };
        let mut deps: Vec<String> = package["dependencies"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|dep| dep["kind"].as_str() != Some("dev"))
            .filter(|dep| !dep["path"].is_null())
            .filter_map(|dep| dep["name"].as_str())
            .filter(|dep| names.contains(dep))
            .map(str::to_owned)
            .collect();
        deps.sort();
        deps.dedup();
        members.push(Member {
            name: name.to_owned(),
            publishable,
            deps,
        });
    }
    Ok(members)
}

/// Order the publishable members so that each appears after every workspace
/// crate it depends on, with alphabetical order breaking ties.
///
/// # Errors
///
/// Returns the reason when a publishable crate depends on a member that cannot
/// be published, or when the dependencies form a cycle.
pub fn publish_order(members: &[Member]) -> Result<Vec<String>, String> {
    let publishable: BTreeSet<&str> = members
        .iter()
        .filter(|m| m.publishable)
        .map(|m| m.name.as_str())
        .collect();

    let mut waiting: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for member in members.iter().filter(|m| m.publishable) {
        let mut needs = BTreeSet::new();
        for dep in &member.deps {
            if !publishable.contains(dep.as_str()) {
                return Err(format!(
                    "{} depends on {dep}, which is not publishable",
                    member.name
                ));
            }
            needs.insert(dep.as_str());
        }
        waiting.insert(member.name.as_str(), needs);
    }

    let mut order = Vec::with_capacity(waiting.len());
    while !waiting.is_empty() {
        let Some(next) = waiting
            .iter()
            .find(|(_, needs)| needs.is_empty())
            .map(|(name, _)| *name)
        else {
            let names: Vec<&str> = waiting.keys().copied().collect();
            return Err(format!("dependency cycle among {}", names.join(", ")));
        };
        waiting.remove(next);
        for needs in waiting.values_mut() {
            needs.remove(next);
        }
        order.push(next.to_owned());
    }
    Ok(order)
}

/// Publish a single crate, retrying while crates.io reports its rate limit.
/// Returns `true` once the crate is published or already present, `false` on a
/// real failure.
fn publish(crate_name: &str, retry_secs: u64) -> bool {
    for attempt in 1..=MAX_ATTEMPTS {
        println!("==> {crate_name} (attempt {attempt})");

        let output = match Command::new("cargo")
            .args(["publish", "-p", crate_name])
            .output()
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, publishable: bool, deps: &[&str]) -> Member {
        Member {
            name: name.to_owned(),
            publishable,
            deps: deps.iter().map(|d| (*d).to_owned()).collect(),
        }
    }

    #[test]
    fn orders_dependencies_first_and_ties_alphabetically() {
        let members = [
            member("ffi", true, &["core", "codec", "mqtt"]),
            member("mqtt", true, &["core"]),
            member("codec", true, &["core"]),
            member("core", true, &[]),
            member("examples", false, &["core", "ffi"]),
            member("xtask", false, &[]),
        ];
        let order = publish_order(&members).unwrap();
        assert_eq!(order, ["core", "codec", "mqtt", "ffi"]);
    }

    #[test]
    fn reports_a_cycle() {
        let members = [
            member("a", true, &["b"]),
            member("b", true, &["c"]),
            member("c", true, &["a"]),
            member("root", true, &[]),
        ];
        let err = publish_order(&members).unwrap_err();
        assert_eq!(err, "dependency cycle among a, b, c");
    }

    #[test]
    fn rejects_a_dependency_on_an_unpublishable_member() {
        let members = [member("lib", true, &["tool"]), member("tool", false, &[])];
        let err = publish_order(&members).unwrap_err();
        assert_eq!(err, "lib depends on tool, which is not publishable");
    }

    #[test]
    fn reads_members_from_metadata() {
        let metadata = serde_json::json!({
            "packages": [
                {
                    "name": "core",
                    "publish": null,
                    "dependencies": [
                        { "name": "serde", "kind": null, "path": null }
                    ]
                },
                {
                    "name": "lib",
                    "publish": null,
                    "dependencies": [
                        { "name": "core", "kind": null, "path": "/w/core", "optional": true },
                        { "name": "core", "kind": "build", "path": "/w/core" },
                        { "name": "tool", "kind": "dev", "path": "/w/tool" }
                    ]
                },
                { "name": "tool", "publish": [], "dependencies": [] }
            ]
        });
        let members = members_of(&metadata).unwrap();
        assert_eq!(
            members,
            [
                member("core", true, &[]),
                member("lib", true, &["core"]),
                member("tool", false, &[]),
            ]
        );
    }

    #[test]
    fn every_workspace_crate_publishes_after_its_dependencies() {
        let members = workspace().unwrap();
        let order = publish_order(&members).unwrap();
        let publishable: Vec<&Member> = members.iter().filter(|m| m.publishable).collect();
        assert_eq!(order.len(), publishable.len());
        for member in publishable {
            let position = order
                .iter()
                .position(|name| *name == member.name)
                .expect("every publishable crate is ordered");
            for dep in &member.deps {
                let dep_position = order.iter().position(|name| name == dep).unwrap();
                assert!(
                    dep_position < position,
                    "{} must publish after {dep}",
                    member.name
                );
            }
        }
    }
}
