# Examples

Everything on this page runs in CI on every change, so an example here is never one that
worked once. The programs are complete: each has a `main`, reads top to bottom, and runs
with nothing plugged in, since a simulator or a loopback stands in for the hardware. The
guide examples are the same program in four languages, spliced into each guide from the
test file that runs it, so what a guide shows is exactly what ran.

<!-- table: examples -->
## Programs

Each one is a complete program with a `main`, written to be read top to bottom and run with nothing plugged in. The line beside it runs it.

<div class="pkgs">
<div class="pkg stack" id="example-batched_telemetry">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/batched_telemetry.rs">batched_telemetry</a><code class="pkg-import">examples/batched_telemetry.rs</code><p>Metered-link encoding: pack a batch of readings into a fraction of the bytes.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example batched_telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example batched_telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-conformance">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/conformance.rs">conformance</a><code class="pkg-import">examples/conformance.rs</code><p>The whole SDK in one run: a cold-chain node from sensor to gateway over loopback.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example conformance</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example conformance" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-conformance_vectors">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/conformance_vectors.rs">conformance_vectors</a><code class="pkg-import">examples/conformance_vectors.rs</code><p>Regenerates the cross-language conformance vectors.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example conformance_vectors</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example conformance_vectors" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-degraded_link">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/degraded_link.rs">degraded_link</a><code class="pkg-import">examples/degraded_link.rs</code><p>Offline-first survives a flaky link: buffer, retry over a degraded link, lose nothing.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example degraded_link</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example degraded_link" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-device_profile">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/device_profile.rs">device_profile</a><code class="pkg-import">examples/device_profile.rs</code><p>A device profile assembled into a ready-to-run cold-chain node, over loopback.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example device_profile</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example device_profile" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-lora_budget">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/lora_budget.rs">lora_budget</a><code class="pkg-import">examples/lora_budget.rs</code><p>LoRa airtime and duty cycle: what it costs to send a batch over a long-range link.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example lora_budget</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example lora_budget" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-mavlink_sitl">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/mavlink_sitl.rs">mavlink_sitl</a><code class="pkg-import">examples/mavlink_sitl.rs</code><p>Fly a mission on a MAVLink vehicle with no hardware.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example mavlink_sitl</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example mavlink_sitl" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-robot_waypoint">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/robot_waypoint.rs">robot_waypoint</a><code class="pkg-import">examples/robot_waypoint.rs</code><p>Drive a rover safely, dead-reckon where it is, steer to a waypoint, and speak ROS 2.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example robot_waypoint</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example robot_waypoint" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-signed_audit">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_audit.rs">signed_audit</a><code class="pkg-import">examples/signed_audit.rs</code><p>A tamper-evident cold-chain log: signed, hash-chained fridge readings.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_audit</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_audit" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-signed_telemetry">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_telemetry.rs">signed_telemetry</a><code class="pkg-import">examples/signed_telemetry.rs</code><p>Tamper-evident telemetry: a device signs each reading, a gateway verifies it.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-signed_update">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/signed_update.rs">signed_update</a><code class="pkg-import">examples/signed_update.rs</code><p>Updating a device in the field, including the update that goes wrong.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example signed_update</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example signed_update" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-store_and_forward">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/store_and_forward.rs">store_and_forward</a><code class="pkg-import">examples/store_and_forward.rs</code><p>Offline-first store-and-forward, end to end, with no hardware and no broker.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example store_and_forward</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example store_and_forward" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="example-telemetry">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/examples/telemetry.rs">telemetry</a><code class="pkg-import">examples/telemetry.rs</code><p>Observability that degrades gracefully on a metered link.</p></div>
<div class="pkg-get"><code class="cmd">cargo run -p pamoja-examples --example telemetry</code><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example telemetry" aria-label="Copy the install command">copy</button></div>
</div>
</div>
</div>

## Guide examples

Every guide carries the same example in Rust, TypeScript, Python, and C#, spliced from the file that runs it in CI. The buttons open those files; the guide explains them.

### Identity

<div class="pkgs">
<div class="pkg" id="guide-security">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/security.html">Device identity</a><ul class="pkg-proves"><li>A public key taken as 32 bytes verifies the reading the device signed, so a gateway needs nothing else from a device to check what it sends.</li><li>Signing is deterministic: the same reading signed twice gives the identical signature, so signing needs no entropy.</li><li>A reading altered after signing does not verify, which is what catches a value edited between the meter and the bill.</li><li>The same reading and signature offered under a second device's key do not verify either, so a signature does not carry over to another identity.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/security.rs">Rust <code>security.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/security.ts">TypeScript <code>security.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/security.py">Python <code>security.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SecurityGuide.cs">C# <code>SecurityGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/security.html">Guide</a></div>
</div>
</div>
</div>

### Codecs

<div class="pkgs">
<div class="pkg" id="guide-codec">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/codec.html">Codecs</a><ul class="pkg-proves"><li>A reading transcoded to CBOR comes back unchanged, and the CBOR is shorter than the JSON it came from.</li><li>Five samples that rise, fall, then jump to 900 pack into fewer bytes than the forty the raw values cost, so a negative difference and a large one both stay small.</li><li>The packed batch unpacks to the same five numbers, the jump included.</li><li>Quantized readings decode to within <code>0.01</code>, the precision a scale of <code>100</code> sets, and that error is what the packing trades for the bytes.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/codec.rs">Rust <code>codec.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/codec.ts">TypeScript <code>codec.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/codec.py">Python <code>codec.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs">C# <code>CodecGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/codec.html">Guide</a></div>
</div>
</div>
</div>

### Helpers

<div class="pkgs">
<div class="pkg" id="guide-kit">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/kit.html">Helpers</a><ul class="pkg-proves"><li>12 mA reads 50% and 4 mA reads 0%, because the span starts at 4 mA and not at zero; a map scaled from zero would put mid-scale at 60% and still be self-consistent.</li><li>0 mA reads -25%, off the bottom of the scale, which is what separates a broken loop from an empty tank.</li><li>One dropout among five samples leaves the filtered level at 50%, where a mean over the same window would be dragged down by it.</li><li>At the setpoint the pump stays off, and it starts only once the level falls below the deadband.</li><li>Once running it keeps running at a level back inside the deadband, and stops only above the top of it.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/kit.rs">Rust <code>kit.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/kit.ts">TypeScript <code>kit.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/kit.py">Python <code>kit.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs">C# <code>KitGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/kit.html">Guide</a></div>
</div>
</div>
</div>

### Field I/O

<div class="pkgs">
<div class="pkg" id="guide-serial">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing</a><ul class="pkg-proves"><li>A payload carrying the end byte or the escape byte is stuffed rather than taken for a frame boundary, and it decodes back byte for byte with both values still in it.</li><li>Both framings cost bytes: each frame comes out longer than the payload that went into it.</li><li>A frame that ends inside an escape pair is discarded on its own, and the whole frames before and after it come out of the same chunk intact.</li><li>The dropped frame is counted, and the count stays at one for the rest of the chunk rather than climbing as the good bytes after it arrive, so a read loop can measure how noisy a link is.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/serial.rs">Rust <code>serial.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts">TypeScript <code>serial.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/serial.py">Python <code>serial.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs">C# <code>SerialGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/serial.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-modbus">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU</a><ul class="pkg-proves"><li>A request for three holding registers is eight bytes on the wire: the unit address, the function code, the two 16-bit fields and the checksum.</li><li>A reply validates its own checksum before any value is read out of it.</li><li>In TypeScript, Python and C# the reply reports the unit address it was sent to and no exception, so a served request is not read as a refused one.</li><li>The three 16-bit registers come back in the order the meter reported them.</li><li>A corrupted byte is caught rather than passed on as a plausible reading.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/modbus.rs">Rust <code>modbus.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts">TypeScript <code>modbus.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py">Python <code>modbus.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs">C# <code>ModbusGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/modbus.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-can">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939</a><ul class="pkg-proves"><li>A priority, a parameter group and a source address compose an identifier and decode back out of it unchanged.</li><li>The broadcast carries no destination, while a parameter group below the PDU1 limit is addressed, so those eight bits name a node instead of extending the group number.</li><li>A standard 11-bit identifier decodes to nothing, because J1939 does not use one.</li><li>Engine speed sits in bytes 4 and 5 of that group at 0.125 rpm per bit, so the eight-byte payload reads back as a thousand rpm.</li><li>The CAN-FD length encoding puts 32 bytes at data length code 13, while a classic frame still refuses a ninth byte.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/can.rs">Rust <code>can.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts">TypeScript <code>can.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py">Python <code>can.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs">C# <code>CanGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/can.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-gpio">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO</a><ul class="pkg-proves"><li>A device at <code>0x76</code> is written to as <code>0xEC</code> and read from as <code>0xED</code>, one byte either way, which is why a datasheet and a bus capture rarely print the same number.</li><li><code>0x76</code> is a device address and <code>0x78</code> is not, because <code>0x78</code> opens the block the specification keeps back for itself.</li><li>A 10-bit address takes two bytes on the wire where a 7-bit one takes a single byte, so a bus driver sends a different number of bytes depending on the address it holds.</li><li>Mode 3 is CPOL 1 with CPHA 1, and the pair maps back the other way: CPOL 1 with CPHA 0 is mode 2, not mode 3 again.</li><li>An active-low relay is energised by a low level, which that polarity reads back as asserted, and releasing it is a rising edge that a falling-edge trigger ignores.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/gpio.rs">Rust <code>gpio.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts">TypeScript <code>gpio.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py">Python <code>gpio.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs">C# <code>GpioGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/gpio.html">Guide</a></div>
</div>
</div>
</div>

### Sensing and actuation

<div class="pkgs">
<div class="pkg" id="guide-sensors">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers</a><ul class="pkg-proves"><li>25.0625 degrees Celsius at 12-bit resolution builds register <code>0x0191</code>, the row the DS18B20 temperature table publishes, and that register decodes back to the same temperature, exact in integer micro-degrees.</li><li>The same nine bytes report the resolution the configuration byte selects and both alarm thresholds, 75 and -10 degrees, written into them.</li><li>One flipped bit fails the CRC, so a read corrupted on a long 1-Wire run is repeated instead of logged as a temperature a couple of degrees off.</li><li>The 1-Wire checksum is CRC-8/MAXIM-DOW, which over the ASCII digits 1 to 9 produces the published check value <code>0xA1</code>.</li><li>1 mA per count across a 2 milliohm shunt calibrates to <code>0x5000</code>, the number the INA219 datasheet's design example works out, and the registers a monitor across that load reports decode back to 11.98 V, 10 A, and 119.8 W.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sensors.rs">Rust <code>sensors.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sensors.ts">TypeScript <code>sensors.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py">Python <code>sensors.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs">C# <code>SensorsGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-actuators">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers</a><ul class="pkg-proves"><li>50 Hz off the 25 MHz internal oscillator is prescale 121 (<code>0x79</code>), the value the datasheet's formula gives, so a divider that is wrong but round-trips consistently still fails.</li><li>Channel 3's registers begin at <code>0x12</code>, four along from each channel before it.</li><li>A centred 1500 microsecond pulse at 50 Hz goes low at count 307 of the 4096 counts in a period.</li><li>Fully off is its own encoding rather than a zero duty, which would still hold the output high for the first count of every period.</li><li>Half-step drive alternates one energised coil with two, <code>1000</code> then <code>1100</code> then <code>0100</code>, and eight steps wrap back to the pattern it started on.</li><li>A quarter turn of a 1.8-degree motor is 50 whole steps.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/actuators.rs">Rust <code>actuators.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/actuators.ts">TypeScript <code>actuators.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/actuators.py">Python <code>actuators.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs">C# <code>ActuatorsGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Guide</a></div>
</div>
</div>
</div>

### Radio and reach

<div class="pkgs">
<div class="pkg" id="guide-lora">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime</a><ul class="pkg-proves"><li>Data rate 0 in EU863-870 selects SF12, the slowest rate the band defines and the one that reaches furthest.</li><li>A ten-byte frame at those settings takes 991,232 microseconds on air, the published time on air for SF12 at 125 kHz, so a plan carrying the wrong bandwidth fails here rather than passing a round-trip against itself.</li><li>868.1 MHz sits in a sub-band capped at 1% of the time and 16 dBm, both read from the plan by frequency.</li><li>One percent of the time buys ninety-nine times the frame's own length in silence after it, which leaves thirty-six readings an hour.</li><li>A frequency inside no sub-band the plan describes reports no duty cycle rather than an unlimited one, because the limit on it is published elsewhere.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lora.rs">Rust <code>lora.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lora.ts">TypeScript <code>lora.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lora.py">Python <code>lora.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs">C# <code>LoraGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/lora.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-lorawan">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN</a><ul class="pkg-proves"><li>A device holding nothing but the root key verifies the accept and reads the address <code>0x26012E43</code> out of it, decrypted from the frame rather than configured on the device.</li><li>Neither side transmits a session key. The device derives its pair from the accept it decrypts, the network derives its pair from the grant, and a frame the device encrypts reads back at the network as <code>level=high</code>.</li><li>That uplink exercises both derived keys: the message integrity code verifies under the network session key and the payload decrypts under the application key, because the frame goes to a port above zero.</li><li>One byte flipped inside the accept fails the integrity check, so a device does not activate on a join it cannot attribute to its own network.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lorawan.rs">Rust <code>lorawan.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lorawan.ts">TypeScript <code>lorawan.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lorawan.py">Python <code>lorawan.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LorawanGuide.cs">C# <code>LorawanGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/lorawan.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-mesh">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames</a><ul class="pkg-proves"><li>A broadcast frame parses back out of the bytes that go on the air with its payload intact and its destination equal to <code>BROADCAST</code>.</li><li>A packet is identified as it floods by its source and sequence id, so the second copy to arrive is dropped instead of relayed again.</li><li>Relaying spends exactly one hop, and the forwarded bytes still parse and carry the same payload, because the checksum covers every byte except the hop limit.</li><li>A packet whose hops have run out is not relayed, which is what keeps a flood finite.</li><li>An inverted payload byte fails the checksum instead of arriving as a plausible reading.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mesh.rs">Rust <code>mesh.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts">TypeScript <code>mesh.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mesh.py">Python <code>mesh.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs">C# <code>MeshGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/mesh.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-routing">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/routing.html">Routing</a><ul class="pkg-proves"><li>One packet heard from the pump through the north relay teaches the way back to it, with no routing messages exchanged.</li><li>A cost-1 report through the east relay takes that route over and a cost-4 report through the south relay is refused, so the table holds the cheapest way it has heard and each observation says whether it changed anything.</li><li>Four observations of two nodes leave two routes, not four.</li><li>A packet for the gateway is delivered, one for the pump relays to the east relay, and one for the silo floods.</li><li>Forgetting the pump drops its route and leaves the tank's; packets for the pump flood again.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/routing.rs">Rust <code>routing.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/routing.ts">TypeScript <code>routing.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/routing.py">Python <code>routing.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs">C# <code>RoutingGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/routing.html">Guide</a></div>
</div>
</div>
</div>

### MAVLink

<div class="pkgs">
<div class="pkg" id="guide-mavlink">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/mavlink.html">MAVLink</a><ul class="pkg-proves"><li>Fed noise and a copy whose checksum fails, the parser still recovers the frame behind them, and it decodes back to the vehicle's heartbeat: Rust compares every field, the other three the type it reports.</li><li>The recovered frame carries the message id the dialect gives <code>HEARTBEAT</code>, so the header agrees with the payload it wraps.</li><li>The first arm request goes out with confirmation <code>0</code>, and a timeout hands back <code>1</code> for the resend, so the vehicle can tell a retry from a second, deliberate command.</li><li>An acknowledgement for <code>NAV_TAKEOFF</code> comes back unrelated, so another command's answer leaves this exchange still waiting.</li><li>The acknowledgement naming <code>COMPONENT_ARM_DISARM</code> ends the exchange and hands back the result the vehicle sent.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mavlink.rs">Rust <code>mavlink.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts">TypeScript <code>mavlink.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py">Python <code>mavlink.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs">C# <code>MavlinkGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/mavlink.html">Guide</a></div>
</div>
</div>
</div>

### Trust and operation

<div class="pkgs">
<div class="pkg" id="guide-audit">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/audit.html">Audit log</a><ul class="pkg-proves"><li>The two records verify in order against nothing but the public half of the device's key.</li><li>The second record's link is the digest of the first, so the chain fixes the order as well as the contents.</li><li>A record edited in storage still parses and still carries the device's signature, but the digest recomputed from its fields no longer matches it, so verification fails.</li><li>A log missing its first record is rejected as well: the survivor's index and its link both say a record came before it.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/audit.rs">Rust <code>audit.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/audit.ts">TypeScript <code>audit.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/audit.py">Python <code>audit.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/AuditGuide.cs">C# <code>AuditGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/audit.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-session">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/session.html">Secured session</a><ul class="pkg-proves"><li>The gateway opens what the node sealed, so both ends reached the same key from opposite roles without either of them sending it.</li><li>What leaves the node is not the reading: the ciphertext differs from <code>flow=41.2</code>.</li><li>Those nine bytes come back exactly, in Rust out of the same buffer that held the ciphertext a moment earlier.</li><li>A frame the gateway has already accepted is refused when it arrives again, so a message captured off the air cannot be delivered twice.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/session.rs">Rust <code>session.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/session.ts">TypeScript <code>session.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/session.py">Python <code>session.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SessionGuide.cs">C# <code>SessionGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/session.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-update">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/update.html">Signed updates</a><ul class="pkg-proves"><li>Verifying the envelope hands back the manifest, so the device learns which slot the release is for from the signature rather than from whoever sent it.</li><li>The digest in the manifest is the one the library computes over the image, so a publisher that hashed the wrong bytes cannot produce a release that stages.</li><li>Staging completes only because every byte the manifest declared arrived and hashed to that digest, which the device recomputes as the pieces come in.</li><li>The release lands in the slot the device is not running from, so the working image is never overwritten.</li><li>The first boot into the staged image is a trial, and confirming it is what leaves the slot confirmed rather than reverting on the next boot.</li><li>A release signed by a key the device is not anchored to is refused, even though the manifest inside it is the one that was just accepted, because the signature is checked before anything in the manifest is read.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/update.rs">Rust <code>update.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/update.ts">TypeScript <code>update.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/update.py">Python <code>update.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/UpdateGuide.cs">C# <code>UpdateGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/update.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-power">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/power.html">Power</a><ul class="pkg-proves"><li>With the default thresholds, 80% charge is active, 35% is saver and 12% is critical.</li><li>The interval follows the mode, so a battery at 12% is asked for one reading an hour where a healthy one gives sixty.</li><li>A delivering panel eases the governor off by one mode and no further, so the flat battery reports on the saver cadence rather than the active one.</li><li>Two seconds of work is one part in thirty at the minute cadence and one part in 1800 at the hourly one, the sixtyfold cut in average draw the stretch buys.</li><li>The fraction is the awake share of the whole period, so two seconds awake and 58 asleep is one in thirty, not one in twenty-nine.</li><li><code>from_fraction</code> divides the period it is given, so a quarter-duty second is 250ms awake and 750ms asleep.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/power.rs">Rust <code>power.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/power.ts">TypeScript <code>power.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/power.py">Python <code>power.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs">C# <code>PowerGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/power.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-telemetry">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Telemetry</a><ul class="pkg-proves"><li>A link cost sets the bar: <code>Metered</code> puts it at <code>Info</code> and <code>Expensive</code> raises it to <code>Warn</code>, so how much a node says follows what the link costs and not the level it was built with.</li><li>The same <code>reading.ok</code> is handed back on the metered link and held back on the satellite one, so each event is judged against the bar in force when it is recorded.</li><li>A shipped event comes back with its code and its measurement, <code>battery.low</code> at <code>0.18</code>, so a transport has the number that triggered it.</li><li>A held-back event is still counted: two readings were recorded at <code>Info</code> even though only the first one went out.</li><li>Five events recorded reconcile as three shipped and two dropped, so thinning the stream loses nothing from the totals.</li><li><code>Offline</code> is the last rung, holding back everything below <code>Error</code>, so a node with no link keeps its failures and nothing else.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/telemetry.rs">Rust <code>telemetry.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/telemetry.ts">TypeScript <code>telemetry.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/telemetry.py">Python <code>telemetry.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TelemetryGuide.cs">C# <code>TelemetryGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/telemetry.html">Guide</a></div>
</div>
</div>
</div>

### Transports and testing

<div class="pkgs">
<div class="pkg" id="guide-mqtt">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/mqtt.html">MQTT</a><ul class="pkg-proves"><li>A subscription with a <code>+</code> in it takes a reading published under a concrete name, so a gateway follows every node's temperature without naming one.</li><li>What arrives is the topic the node published to, <code>sensors/1/temperature</code>, rather than the <code>sensors/+/temperature</code> filter that matched it, and the payload is the bytes the node sent.</li><li>Both clients default to at least once, and the one setting covers the subscription and the publish alike, so a reading travels under a guarantee the broker acknowledges rather than fire and forget.</li><li>A client that has disconnected reports itself disconnected, so code deciding whether to reconnect is not reading a stale flag.</li><li>A broker that is not there fails the connect and leaves the client not connected, which is what a retry loop tests.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mqtt.rs">Rust <code>mqtt.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mqtt.ts">TypeScript <code>mqtt.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mqtt.py">Python <code>mqtt.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MqttGuide.cs">C# <code>MqttGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/mqtt.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-coap">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/coap.html">CoAP</a><ul class="pkg-proves"><li>Connecting a CoAP endpoint binds a local socket and nothing else: it reports itself connected with nothing on the far side.</li><li>A non-confirmable send succeeds without an acknowledgement, which is the mode for a reading whose loss costs nothing.</li><li>A confirmable send to that same address fails once its retransmissions run out. Both endpoints point at the same dead port, so the delivery guarantee, not the destination, decides the outcome.</li><li>The failure arrives as an error the caller handles rather than a silent success: each example catches it and prints why the command gave up, so a command is never assumed to have landed.</li><li>Disconnecting releases the socket and the endpoint reports itself closed.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/coap.rs">Rust <code>coap.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/coap.ts">TypeScript <code>coap.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/coap.py">Python <code>coap.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs">C# <code>CoapGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/coap.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-loopback">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/loopback.html">Loopback</a><ul class="pkg-proves"><li>A payload published on one link arrives on another carrying the topic it was sent to, with no port bound and no broker process running.</li><li><code>+</code> matches exactly one level, so the filter takes the temperature topic and leaves the <code>/raw</code> reading a level below it, even though that one went out first.</li><li><code>#</code> matches the levels that remain, so the second filter takes the deeper topic the single-level one passed over.</li><li>A link can join a broker that has already routed traffic, take a filter of its own, and receive a reading published after it connects.</li><li>A disconnected link fails the send rather than accepting a reading it has no way to deliver.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/loopback.rs">Rust <code>loopback.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/loopback.ts">TypeScript <code>loopback.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/loopback.py">Python <code>loopback.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoopbackGuide.cs">C# <code>LoopbackGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/loopback.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-sync">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/sync.html">Store and forward</a><ul class="pkg-proves"><li>Peek returns the oldest record, <code>20.1</code>, and leaves all three readings queued, so a send that fails part-way loses nothing.</li><li>The queue drains oldest first, <code>20.1</code> then <code>20.4</code> then <code>20.2</code>, the order the readings were taken.</li><li>Popping until it returns nothing leaves the queue empty.</li><li>A full store refuses the third append and still holds two records, so the caller is told to back off rather than have the oldest reading dropped to make room.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sync.rs">Rust <code>sync.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sync.ts">TypeScript <code>sync.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sync.py">Python <code>sync.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SyncGuide.cs">C# <code>SyncGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/sync.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-ladder">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/ladder.html">Transport ladder</a><ul class="pkg-proves"><li>Rungs are tried in the order they were added, and a refusing rung falls through to the next.</li><li>The first reading arrives on the backhaul's subscriber carrying <code>21.5</code>, so which link was used is observable rather than assumed.</li><li>With every rung down, a send is buffered rather than lost, and the ladder reports the one record it is holding.</li><li>A flush while both links are down forwards nothing and leaves that record waiting in the queue.</li><li>The next flush forwards one, the gateway receives <code>21.6</code>, and the queue drops to zero, so the backlog went out exactly once.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/ladder.rs">Rust <code>ladder.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ladder.ts">TypeScript <code>ladder.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ladder.py">Python <code>ladder.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LadderGuide.cs">C# <code>LadderGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/ladder.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-bus">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/bus.html">Event bus</a><ul class="pkg-proves"><li>One publish reaches every subscriber, and each reads its own copy of <code>battery.low</code> off its own queue.</li><li>A subscriber taken later starts at the next event, so its first read is <code>link.up</code> and what went out before it existed is gone for good.</li><li>The logger still has <code>battery.low</code> waiting after control has read it, so an event is not consumed by whoever reads first.</li><li>Five events into a buffer of two leave the reader at <code>3</code>, so a reader that falls behind loses the oldest events, not the newest.</li><li>All five publishes return with nothing draining the buffer, so a slow subscriber costs itself rather than the publisher.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/bus.rs">Rust <code>bus.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/bus.ts">TypeScript <code>bus.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/bus.py">Python <code>bus.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs">C# <code>BusGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/bus.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-transport">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/transport.html">Engine surface</a><ul class="pkg-proves"><li>A fault injector sits in the ladder where a plain link would, and the send it refuses comes back as <code>Buffered</code> rather than an error, so the reading is held instead of lost.</li><li>The reading taken next is buffered too, even though the link would carry it now, and the ladder counts both as queued.</li><li>A flush forwards the whole backlog and leaves nothing queued behind it.</li><li>The subscriber reads <code>20.1</code> and then <code>20.4</code>, so the far end sees the readings in the order they were taken, not the order the link became willing to carry them.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/transport.rs">Rust <code>transport.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/transport.ts">TypeScript <code>transport.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/transport.py">Python <code>transport.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/TransportGuide.cs">C# <code>TransportGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/transport.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-sim">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/sim.html">Simulators</a><ul class="pkg-proves"><li>The replay hands back exactly the series it was given: 4 m, 3 m, 1.5 m and 0.5 m, in that order.</li><li>The recording actuator keeps every command the loop issued: three at one metre per second, then a zero once the 0.5 m reading falls under the metre of clearance the rule drives on.</li><li>Those three half-second commands dead-reckon to 1.5 m along x and nothing along y, so a straight run stays straight.</li><li>The turn on the spot puts the heading at 0.5 rad and leaves x at 1.5 m; an integrator that translated on a pure rotation would carry the rover past that and still look self-consistent.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sim.rs">Rust <code>sim.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sim.ts">TypeScript <code>sim.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sim.py">Python <code>sim.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs">C# <code>SimGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/sim.html">Guide</a></div>
</div>
</div>
</div>

### Profiles and robotics

<div class="pkgs">
<div class="pkg" id="guide-profile">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/profile.html">Device profiles</a><ul class="pkg-proves"><li>A manifest parses into the name, topic, setpoint policy and sampling schedule the node runs on, with <code>cooling</code> set false marking the output a heater.</li><li><code>saver_below</code> never appears in the manifest and still reads <code>0.5</code>, the documented default, rather than nothing.</li><li>A reading below the deadband switches the lamp on and raises <code>OutOfRange</code>, so the excursion is reported as well as acted on.</li><li>A reading inside the safe band raises nothing, so an alert tracks the band rather than firing on every sample.</li><li>Serializing writes the defaulted threshold out by name, so the shared text names <code>saver_below</code> even though the manifest never did.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/profile.rs">Rust <code>profile.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts">TypeScript <code>profile.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py">Python <code>profile.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs">C# <code>ProfileGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/profile.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-ros2">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/ros2.html">ROS 2 rules</a><ul class="pkg-proves"><li>A token may hold letters, digits and underscores but may not begin with a digit, so <code>/2foo</code> is rejected where <code>/robot1/camera_left/image_raw</code> passes.</li><li>A leading slash is what makes a name fully qualified, and <code>chatter</code> without one is relative.</li><li>A topic goes out under <code>rt</code>, a service request under <code>rq</code> and its response under <code>rr</code>, so a request and its response keep one ROS name and still never collide in a DDS partition.</li><li><code>std_msgs/msg/String</code> becomes <code>std_msgs::msg::dds_::String_</code>, the <code>dds_</code> namespace and the trailing underscore included, since a peer matches the whole string.</li><li>A malformed type name maps to nothing rather than to something that looks plausible.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/ros2.rs">Rust <code>ros2.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts">TypeScript <code>ros2.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py">Python <code>ros2.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs">C# <code>Ros2Guide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/ros2.html">Guide</a></div>
</div>
</div>
<div class="pkg" id="guide-zenoh">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Zenoh keys</a><ul class="pkg-proves"><li><code>*</code> stands for exactly one chunk, so <code>fleet/*/battery</code> covers <code>fleet/n7/battery</code> and not <code>fleet/n7/rack/battery</code>.</li><li><code>**</code> stands for any number of chunks, so <code>fleet/**</code> covers <code>fleet/n7/rack/battery</code>, and <code>fleet/**/battery</code> covers <code>fleet/battery</code>, where it stands for none at all.</li><li>A repeated wildcard is not canonical. <code>fleet/**/**/battery</code> canonizes to <code>fleet/**/battery</code>, so a router compares subscriptions in that form rather than as written.</li><li>An empty chunk makes <code>fleet//battery</code> invalid, and canonizing it yields nothing rather than a repaired expression.</li></ul></div>
</div>
<div class="pkg-foot">
<div class="pkg-btns"><a class="pkg-btn rust" href="https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/zenoh.rs">Rust <code>zenoh.rs</code></a><a class="pkg-btn node" href="https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts">TypeScript <code>zenoh.ts</code></a><a class="pkg-btn python" href="https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py">Python <code>zenoh.py</code></a><a class="pkg-btn dotnet" href="https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs">C# <code>ZenohGuide.cs</code></a><a class="pkg-btn" href="https://pamoja.molex.cloud/docs/guides/zenoh.html">Guide</a></div>
</div>
</div>
</div>
<!-- end -->
