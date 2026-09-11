# Shared profiles

One JSON manifest per profile, each a complete node written down as data: the topic
it publishes on, the control policy it applies to each reading, the sampling
schedule it keeps as its battery drains, and how a dashboard draws it. A device
loads one with `Profile::from_json` in any of the four languages, and the
[device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html) shows
the loop it runs.

The four presets the library ships (`vaccine-fridge-monitor`, `irrigation-node`,
`well-level`, `flood-sensor`) are here in the same form, written from the
constructors that build them, so the code stays the one source for those four. The
rest were shared by people who ran them.

Every file here is proved to run, not only to parse: a test loads each one with the
parser a device uses, feeds it readings that cross its own lines, and checks that
the output switches both ways, the alerts its policy promises fire, and the
sampling interval stretches as the battery drains.

## Adding one

1. Save the manifest as `<name>.json`, where the file name is the profile's `name`,
   with a `description` of a sentence or two.
2. Run `cargo xtask profiles`. It reads every file with the parser a device uses,
   checks it for what a hand-written manifest gets wrong, and rewrites it into the
   form `Profile::to_json` writes, so a diff shows a change of meaning and nothing
   else. `cargo xtask profiles --check` is what CI runs.
3. Run `cargo xtask docs`, which adds it to the
   [catalog page](https://pamoja.molex.cloud/docs/profiles.html), and open a pull
   request.

The [community page](https://pamoja.molex.cloud/docs/community.html#share-a-profile)
lists what the check enforces, and the issue form there takes a manifest from
anyone without a Rust toolchain.
