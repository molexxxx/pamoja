## What this changes

<!-- One paragraph. What the code does now that it did not before, or what it stops
doing. Link the issue if there is one. -->

## Why

<!-- The problem this solves. If it changes behaviour anyone depends on, say so here. -->

## How it was tested

<!-- The commands you ran and what they showed. For a parser or a driver, name the
reference vector, datasheet figure, or specification example the test is anchored to.
A round-trip on its own does not catch an implementation that is wrong but
self-consistent. -->

## Checks

- [ ] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` are clean
- [ ] `cargo test --workspace` passes, including doctests
- [ ] Public items are documented, and any crate README was regenerated with `cargo xtask docs` rather than hand-edited
- [ ] A documentation example that changed was edited in its test and re-spliced, so `cargo xtask docs --check` passes
- [ ] A capability that reaches the bindings reaches all of them, with a conformance vector where it decodes something

<!-- Delete a line that does not apply to this change. Labels are applied
automatically from the files you touched, so you do not need to add one. -->
