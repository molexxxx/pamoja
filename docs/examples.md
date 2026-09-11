# Examples

Everything on this page runs in CI on every change, so an example here is never one that
worked once. The programs are complete: each has a `main`, reads top to bottom, and runs
with nothing plugged in, since a simulator or a loopback stands in for the hardware.

A guide's example is a different thing: the same program in four languages, spliced into
that guide from the test file that runs it, with the line that runs it and what it proves
on the guide's own page. The guides are in the sidebar, grouped the way the capability map
groups them.

<!-- table: examples -->
## Programs

Each one is a complete program with a `main`, written to be read top to bottom and run with nothing plugged in. The line beside it runs it.

<div class="pkgs">
<div class="pkg stack program" id="example-batched_telemetry">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 1</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/batched_telemetry.rs">batched_telemetry</a><code class="pkg-import">examples/batched_telemetry.rs</code><p>Metered-link encoding: pack a batch of readings into a fraction of the bytes.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html"><code>pamoja-sim</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example batched_telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example batched_telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-brooder_node">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 2</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/brooder_node.rs">brooder_node</a><code class="pkg-import">examples/brooder_node.rs</code><p>A poultry brooder, built from a shared profile and run through a cold night.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html"><code>pamoja-gpio</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html"><code>pamoja-hal</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html"><code>pamoja-kit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html"><code>pamoja-ladder</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html"><code>pamoja-loopback</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html"><code>pamoja-power</code></a></li><li class="uses-more"><a href="https://github.com/molexxxx/pamoja/blob/main/examples/brooder_node.rs">and 3 more</a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example brooder_node</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example brooder_node" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-conformance">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 3</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/conformance.rs">conformance</a><code class="pkg-import">examples/conformance.rs</code><p>The whole SDK in one run: a cold-chain node from sensor to gateway over loopback.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html"><code>pamoja-profile</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example conformance</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example conformance" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-conformance_vectors">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 4</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/conformance_vectors.rs">conformance_vectors</a><code class="pkg-import">examples/conformance_vectors.rs</code><p>Regenerates the cross-language conformance vectors.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html"><code>pamoja-actuators</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html"><code>pamoja-audit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html"><code>pamoja-can</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html"><code>pamoja-gpio</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html"><code>pamoja-kit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html"><code>pamoja-ladder</code></a></li><li class="uses-more"><a href="https://github.com/molexxxx/pamoja/blob/main/examples/conformance_vectors.rs">and 20 more</a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example conformance_vectors</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example conformance_vectors" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-degraded_link">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 5</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/degraded_link.rs">degraded_link</a><code class="pkg-import">examples/degraded_link.rs</code><p>Offline-first survives a flaky link: buffer, retry over a degraded link, lose nothing.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example degraded_link</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example degraded_link" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-device_profile">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 6</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/device_profile.rs">device_profile</a><code class="pkg-import">examples/device_profile.rs</code><p>A device profile assembled into a ready-to-run cold-chain node, over loopback.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html"><code>pamoja-loopback</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html"><code>pamoja-power</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html"><code>pamoja-profile</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html"><code>pamoja-sim</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example device_profile</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example device_profile" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-lora_budget">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 7</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/lora_budget.rs">lora_budget</a><code class="pkg-import">examples/lora_budget.rs</code><p>LoRa airtime and duty cycle: what it costs to send a batch over a long-range link.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><code>pamoja-lora</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html"><code>pamoja-sim</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example lora_budget</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example lora_budget" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-mavlink_sitl">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 8</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/mavlink_sitl.rs">mavlink_sitl</a><code class="pkg-import">examples/mavlink_sitl.rs</code><p>Fly a mission on a MAVLink vehicle with no hardware.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html"><code>pamoja-mavlink</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example mavlink_sitl</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example mavlink_sitl" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-robot_waypoint">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 9</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/robot_waypoint.rs">robot_waypoint</a><code class="pkg-import">examples/robot_waypoint.rs</code><p>Drive a rover safely, dead-reckon where it is, steer to a waypoint, and speak ROS 2.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html"><code>pamoja-kit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html"><code>pamoja-ros2</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example robot_waypoint</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example robot_waypoint" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-signed_audit">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 10</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_audit.rs">signed_audit</a><code class="pkg-import">examples/signed_audit.rs</code><p>A tamper-evident cold-chain log: signed, hash-chained fridge readings.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html"><code>pamoja-audit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html"><code>pamoja-security</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_audit</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_audit" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-signed_telemetry">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 11</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_telemetry.rs">signed_telemetry</a><code class="pkg-import">examples/signed_telemetry.rs</code><p>Tamper-evident telemetry: a device signs each reading, a gateway verifies it.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html"><code>pamoja-loopback</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html"><code>pamoja-security</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-signed_update">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 12</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_update.rs">signed_update</a><code class="pkg-import">examples/signed_update.rs</code><p>Updating a device in the field, including the update that goes wrong.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_audit/index.html"><code>pamoja-audit</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_security/index.html"><code>pamoja-security</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_update/index.html"><code>pamoja-update</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_update</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_update" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-store_and_forward">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 13</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/store_and_forward.rs">store_and_forward</a><code class="pkg-import">examples/store_and_forward.rs</code><p>Offline-first store-and-forward, end to end, with no hardware and no broker.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html"><code>pamoja-core</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_loopback/index.html"><code>pamoja-loopback</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html"><code>pamoja-sync</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example store_and_forward</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example store_and_forward" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack program" id="example-telemetry">
<div class="pkg-head">
<div class="pkg-what"><p class="program-id">Program 14</p><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/telemetry.rs">telemetry</a><code class="pkg-import">examples/telemetry.rs</code><p>Observability that degrades gracefully on a metered link.</p><ul class="uses"><li class="uses-head">Imports</li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html"><code>pamoja-codec</code></a></li><li><a href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_telemetry/index.html"><code>pamoja-telemetry</code></a></li></ul>
</div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
</div>

## Community programs

Programs people have shared, held to the same bar: complete, run in CI with nothing plugged in, and credited in the file. The [community page](community.md#share-an-example) says how to add one.

<p>None yet. The first one is yours to add.</p>
<!-- end -->
