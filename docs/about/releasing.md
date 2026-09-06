# Releasing

Every crate, package, and binding shares one version and goes out together, so
`0.1.15` of any one of them wraps `0.1.15` of every other. A release is a tag on
main; everything after that is automatic.

## Why a publish gets checked first

crates.io, npm, PyPI, and NuGet all refuse to re-release a version. A bad publish
cannot be withdrawn, only superseded by another version, and a yanked crate stays
in every lockfile that already resolved it. So the release workflows publish
nothing until `release-preflight` passes, which turns three permanent mistakes
into a failed job:

- **A tag whose version is not the tree's.** `cargo xtask version --check <v>`
  reads every manifest, lockfile, and generated loader structurally, so a
  manifest added since the last release cannot be missed, and requires
  `CHANGELOG.md` to have that version's entry.
- **A tag that is not on main.** A tag on a branch, or on a commit that was
  force-pushed away, would publish code that never landed.
- **A tag whose tests never ran.** A green tick on a pull request is not a green
  tick on the merge commit, so the check asks for a completed successful run of
  `ci`, `node`, `python`, and `dotnet` on that exact commit.

## Cutting one

```sh
git switch -c release/0.1.15
cargo xtask version 0.1.15          # every manifest, lockfile, and loader
# write the version's entry in CHANGELOG.md
cargo run -p xtask -- version --check 0.1.15
cargo run -p xtask -- docs --check
cargo xtask release --dry-run       # resolves every crate against its siblings
```

Open that as a pull request labelled `release`, merge it when green, and wait for
`rust (fmt, clippy, test)` and the three binding jobs to finish on the merge
commit. Then tag it:

```sh
git tag -a v0.1.15 -m "pamoja 0.1.15" <merge sha>
git push origin v0.1.15
```

The tag starts five workflows. `release-github` publishes the notes, and the
other four publish to crates.io, npm, PyPI, and NuGet. Each runs the preflight
first, so a tag that should not have been pushed costs a red job rather than a
version.

## When one stalls

Every release workflow also takes a version by hand, so a run that failed
partway can be restarted without inventing a new tag. crates.io publishes new
crates at one per ten minutes, and `cargo xtask release` waits that out and skips
what is already published, so a rerun continues rather than starting over.

PyPI caps how many new projects an account may create in a window, and a release
that introduces a project per capability runs past it; retrying inside the window
creates nothing. So the upload goes in dependency order, the compiled engine
first, and stops at the first refusal, and the `pypi-backfill` workflow runs
every six hours, builds only what the latest release still lacks, and uploads it
until the cap answers again. It can also be dispatched by hand. Once every project
exists a release only adds files to existing projects, which the cap never
touches.

## The notes

`release-github` puts the changelog's entry for the version first, then the pull
requests that went into it, grouped by label. The entry says what changed and why
it matters; the list says which pull requests carried it. Labels come from the
files a pull request touches, so nobody has to remember one while merging.
