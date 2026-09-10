# Community programs

Complete programs people have shared, held to the same bar as the ones beside them:
each has a `main`, reads top to bottom, runs with nothing plugged in because a
simulator or the loopback transport stands in for the hardware, and runs in CI on
every change. The [examples page](https://pamoja.molex.cloud/docs/examples.html)
lists them under "Community programs".

## Adding one

1. Write it as `<name>.rs` in this directory. The module doc's first paragraph is
   what the examples page shows; a line reading `Run with:` followed by the command
   in backticks is the line shown beside it. Say who wrote it in the doc.
2. Register it in `../Cargo.toml`:

   ```toml
   [[example]]
   name = "<name>"
   path = "community/<name>.rs"
   ```

3. Run it with `cargo run -p pamoja-examples --example <name>`; it must finish on
   its own and print what it found. Then `cargo xtask docs` lists it, and a pull
   request carries it.

The [community page](https://pamoja.molex.cloud/docs/community.html#share-an-example)
has the longer form, and an issue form for a program in another language.
