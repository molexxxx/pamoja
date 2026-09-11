# LoRa radios

`pamoja-lora` works out what a LoRa link costs and how far it reaches, and a radio
chip has to be told all of it, one SPI command at a time. The Semtech SX1261,
SX1262, and SX1268, and the LLCC68 that shares their command set, sit on boards
such as the Heltec WiFi LoRa 32 V3 and the Wio-SX1262 for XIAO. pamoja builds the
bytes of every command they take and decodes every answer they give, chooses the
amplifier setting a regional EIRP ceiling allows behind a given antenna, and holds
the radio silent for the off time a duty-cycle limit owes.

In Rust, `pamoja-radios` also drives the chip over any `embedded-hal` SPI bus and
carries mesh frames on it as a pamoja transport. The other languages bind the
command set, the decoders, and the duty-cycle guard, so a host can plan a
transmission, check a logic-analyzer capture, or drive the chip over a bus of its
own.

## What the example does

It plans one transmission from an SX1262 node: a ten-byte reading on 868.1 MHz at
DR3 of the EU863-870 plan, sent through a 2.15 dBi whip on half a decibel of
pigtail. It picks the amplifier setting that keeps the EIRP under the plan's
ceiling, prints the bytes of the nine commands that configure the chip and start
the frame, decodes what the chip answers once the frame has gone and when a frame
arrives, and records the silence the frame owes.

The command bytes are pinned in the conformance vectors every binding checks
itself against, so this page prints each command beside its name instead of
asserting it. The only hex typed out is the chip's answers, which on a real node
come back over the same SPI bus.

It proves:

- The plan's 16 dBm EIRP ceiling, less 2.15 dBi of antenna gain and plus 0.5 dB of
  pigtail loss, leaves 14.35 dBm at the chip, so the amplifier is set to 14 dBm: a
  radio takes whole decibels, and rounding down keeps the ceiling.
- The link settings a data rate names turn straight into the SetModulationParams
  and SetPacketParams bytes, with no spreading factor or bandwidth code typed by
  hand.
- The transmit timeout is the frame's airtime plus a second, counted in the chip's
  15.625 microsecond steps.
- A GetIrqStatus answer of `00 01` is TxDone and nothing else, and a status byte of
  `0x2C` is the chip back in RC standby reporting a finished transmission.
- A GetPacketStatus answer turns into the RSSI and SNR a frame was heard at.
- A 1% sub-band owes ninety-nine times the frame's airtime in silence, and the
  guard refuses the next frame until that has passed.

## Run it

The example below is a test that runs in CI, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo test -p pamoja-examples --test guides radios -- --nocapture" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo test -p pamoja-examples --test guides radios -- --nocapture</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run test:guides -- radios" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run test:guides -- radios</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/radios.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/radios.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- radios" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- radios</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/tests/guides/radios.rs#example -->
From [`examples/tests/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/radios.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx126x::command;
use pamoja_radios::sx126x::config::{
    self, LoraModulation, LoraPacket, PacketType, PowerAmplifier, RampTime, StandbyMode,
    TxPower,
};
use pamoja_radios::sx126x::irq::Irq;
use pamoja_radios::sx126x::status::{PacketStatus, Status};

let hex = |bytes: &[u8]| {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
};

// An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
// 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
// and pigtail decide how hard the amplifier may drive under that cap.
let eu868 = Region::Eu868.plan();
let frequency = 868_100_000;
let link = eu868.link_settings(3).expect("DR3 is a LoRa data rate");
let whip = LinkBudget {
    transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
    transmit_cable_loss_db: Decibels::from_tenths(5),
    ..LinkBudget::default()
};
let ceiling = eu868.max_eirp_dbm(frequency);
let power = TxPower::under_ceiling(
    PowerAmplifier::HighPower,
    &whip,
    Decibels::from_db(ceiling.into()),
);
println!(
    "power     {} dBm under a {ceiling} dBm EIRP ceiling",
    power.setting_dbm
);

// The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
// transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
let airtime = link.airtime_us(10);
let events = Irq::TX_DONE | Irq::TIMEOUT;
let modulation = LoraModulation::from_link(&link).expect("125 kHz is an SX126x bandwidth");
let commands = [
    ("standby", command::set_standby(StandbyMode::Rc)),
    ("packet type", command::set_packet_type(PacketType::Lora)),
    (
        "frequency",
        command::set_rf_frequency(config::frequency_word(frequency)),
    ),
    ("pa config", command::set_pa_config(power.pa)),
    (
        "tx params",
        command::set_tx_params(power.setting_dbm, RampTime::at_least(40)),
    ),
    (
        "modulation",
        command::set_lora_modulation_params(modulation),
    ),
    (
        "packet",
        command::set_lora_packet_params(LoraPacket::from_link(&link, 10, false)),
    ),
    (
        "irq",
        command::set_dio_irq_params(events, events, Irq::NONE, Irq::NONE),
    ),
    (
        "tx",
        command::set_tx(config::timeout_steps(airtime + 1_000_000)),
    ),
];
for (name, bytes) in &commands {
    println!("{name:<12}{}", hex(bytes.as_bytes()));
}

// Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
// chip back in standby.
let irq = Irq::from_bytes([0x00, 0x01]);
let sent = irq.contains(Irq::TX_DONE);
let timed_out = irq.contains(Irq::TIMEOUT);
println!("sent      tx done {sent}, timed out {timed_out}");
let status = Status::from_byte(0x2C);
println!(
    "status    {:?}, {:?}",
    status.chip_mode, status.command_status
);

// A frame that arrives later comes with the signal levels it was heard at.
let heard = PacketStatus::from_bytes([0xDB, 0xF6, 0xE0]);
let db = |value: Decibels| f64::from(value.hundredths()) / 100.0;
println!(
    "received  RSSI {} dBm, SNR {} dB",
    db(heard.rssi_dbm),
    db(heard.snr_db)
);

// The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
// ninety-nine times as long in silence before the next.
let permille = eu868
    .duty_cycle_permille(frequency)
    .expect("868.1 MHz is in a sub-band");
let mut guard = DutyCycle::new(permille);
let held = guard.transmitted(0, &link, 10);
println!(
    "airtime   {held} us, next frame after {} us",
    guard.wait_us(0)
);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/radios.ts#example -->
From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { LoraRegion, airtimeUs, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, sx126x } from '@pamoja/radios'

const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join(' ')

// An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
// 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
// and pigtail decide how hard the amplifier may drive under that cap.
const eu868 = planFor(LoraRegion.Eu868)
const frequency = 868_100_000
const link = eu868.linkSettings(3)!
const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
const ceiling = eu868.maxEirpDbm(frequency)
const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

// The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
// transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
const airtime = airtimeUs(link, 10)
const events = sx126x.Irq.TxDone | sx126x.Irq.Timeout
const commands: [string, Uint8Array][] = [
  ['standby', sx126x.setStandby()],
  ['packet type', sx126x.setPacketTypeLora()],
  ['frequency', sx126x.setRfFrequency(frequency)],
  ['pa config', sx126x.setPaConfig(power)],
  ['tx params', sx126x.setTxParams(power, 40)],
  ['modulation', sx126x.setLoraModulationParams(link)],
  ['packet', sx126x.setLoraPacketParams(link, 10, false)],
  ['irq', sx126x.setDioIrqParams(events, events)],
  ['tx', sx126x.setTx(airtime + 1_000_000)],
]
for (const [name, bytes] of commands) {
  console.log(`${name.padEnd(12)}${hex(bytes)}`)
}

// Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
// chip back in standby.
const irq = sx126x.irq(Buffer.from([0x00, 0x01]))
const sent = (irq & sx126x.Irq.TxDone) !== 0
const timedOut = (irq & sx126x.Irq.Timeout) !== 0
console.log(`sent      tx done ${sent}, timed out ${timedOut}`)
const status = sx126x.status(0x2c)
console.log(`status    ${status.chipMode}, ${status.commandStatus}`)

// A frame that arrives later comes with the signal levels it was heard at.
const heard = sx126x.packetStatus(Buffer.from([0xdb, 0xf6, 0xe0]))
console.log(`received  RSSI ${heard.rssiDbm} dBm, SNR ${heard.snrDb} dB`)

// The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
// ninety-nine times as long in silence before the next.
const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
const held = guard.transmitted(0, link, 10)
console.log(`airtime   ${held} us, next frame after ${guard.waitUs(0)} us`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/radios.py#example -->
From [`bindings/python/guides/radios.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/radios.py):

```python
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, sx126x

# An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
# 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
# and pigtail decide how hard the amplifier may drive under that cap.
eu868 = plan_for("EU868")
frequency = 868_100_000
link = eu868.link_settings(3)
whip = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
ceiling = eu868.max_eirp_dbm(frequency)
power = sx126x.tx_power_under_ceiling(sx126x.Amplifier.HIGH_POWER, whip, ceiling)
print(f"power     {power.setting_dbm} dBm under a {ceiling} dBm EIRP ceiling")

# The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
# transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
airtime = link.airtime_us(10)
events = sx126x.Irq.TX_DONE | sx126x.Irq.TIMEOUT
commands = [
    ("standby", sx126x.set_standby()),
    ("packet type", sx126x.set_packet_type_lora()),
    ("frequency", sx126x.set_rf_frequency(frequency)),
    ("pa config", sx126x.set_pa_config(power)),
    ("tx params", sx126x.set_tx_params(power, 40)),
    ("modulation", sx126x.set_lora_modulation_params(link)),
    ("packet", sx126x.set_lora_packet_params(link, 10, False)),
    ("irq", sx126x.set_dio_irq_params(events, events)),
    ("tx", sx126x.set_tx(airtime + 1_000_000)),
]
for name, data in commands:
    print(f"{name:<12}{data.hex(' ')}")

# Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
# chip back in standby.
irq = sx126x.irq(bytes([0x00, 0x01]))
sent = sx126x.Irq.TX_DONE in irq
timed_out = sx126x.Irq.TIMEOUT in irq
print(f"sent      tx done {sent}, timed out {timed_out}")
status = sx126x.status(0x2C)
print(f"status    {status.chip_mode}, {status.command_status}")

# A frame that arrives later comes with the signal levels it was heard at.
heard = sx126x.packet_status(bytes([0xDB, 0xF6, 0xE0]))
print(f"received  RSSI {heard.rssi_dbm} dBm, SNR {heard.snr_db} dB")

# The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
# ninety-nine times as long in silence before the next.
guard = DutyCycle(eu868.duty_cycle_permille(frequency))
held = guard.transmitted(0, link, 10)
print(f"airtime   {held} us, next frame after {guard.wait_us(0)} us")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs):

```csharp
// An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz,
// through a 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP
// there, and the antenna and pigtail decide how hard the amplifier may drive
// under that cap.
using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
const uint Frequency = 868_100_000;
LoraLink link = eu868.LinkSettings(3)!;
var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
sbyte ceiling = eu868.MaxEirpDbm(Frequency);
Sx126xTxPower power = Sx126x.TxPowerUnderCeiling(Sx126xAmplifier.HighPower, whip, ceiling);
Console.WriteLine($"power     {power.SettingDbm} dBm under a {ceiling} dBm EIRP ceiling");

// The commands in the order section 14.2 of the datasheet gives, each sent in its
// own SPI transaction once BUSY is low. The chip gives up on the frame a second
// after its airtime.
ulong airtime = link.AirtimeMicros(10);
Sx126xIrq events = Sx126xIrq.TxDone | Sx126xIrq.Timeout;
(string Name, byte[] Bytes)[] commands =
[
    ("standby", Sx126x.SetStandby()),
    ("packet type", Sx126x.SetPacketTypeLora()),
    ("frequency", Sx126x.SetRfFrequency(Frequency)),
    ("pa config", Sx126x.SetPaConfig(power)),
    ("tx params", Sx126x.SetTxParams(power, 40)),
    ("modulation", Sx126x.SetLoraModulationParams(link)),
    ("packet", Sx126x.SetLoraPacketParams(link, 10, false)),
    ("irq", Sx126x.SetDioIrqParams(events, events)),
    ("tx", Sx126x.SetTx(airtime + 1_000_000)),
];
foreach ((string name, byte[] bytes) in commands)
{
    Console.WriteLine($"{name,-12}{string.Join(" ", bytes.Select(b => b.ToString("x2")))}");
}

// Once the frame has left, GetIrqStatus answers with TxDone, and the status byte
// shows the chip back in standby.
Sx126xIrq irq = Sx126x.Irq([0x00, 0x01]);
bool sent = irq.HasFlag(Sx126xIrq.TxDone);
bool timedOut = irq.HasFlag(Sx126xIrq.Timeout);
Console.WriteLine($"sent      tx done {sent}, timed out {timedOut}");
Sx126xStatus status = Sx126x.Status(0x2C);
Console.WriteLine($"status    {status.ChipMode}, {status.CommandStatus}");

// A frame that arrives later comes with the signal levels it was heard at.
Sx126xPacketStatus heard = Sx126x.PacketStatus([0xDB, 0xF6, 0xE0]);
Console.WriteLine($"received  RSSI {heard.RssiDbm} dBm, SNR {heard.SnrDb} dB");

// The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime
// buys ninety-nine times as long in silence before the next.
using var guard = new RadioDutyCycle(eu868.DutyCyclePermille(Frequency)!.Value);
ulong held = guard.Transmitted(0, link, 10);
Console.WriteLine($"airtime   {held} us, next frame after {guard.WaitMicros(0)} us");
```
<!-- end -->

## Reference

<!-- table: reference radios -->
- Rust: [`pamoja-radios`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios)
- TypeScript: [`@pamoja/radios`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios)
- Python: [`pamoja.radios`](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios)
- C#: [`Pamoja.Radios`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios)
- Hardware: [SX1262](https://pamoja.molex.cloud/docs/hardware.html#sx1262), [LLCC68](https://pamoja.molex.cloud/docs/hardware.html#llcc68)
<!-- end -->
