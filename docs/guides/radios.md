# LoRa radios

`pamoja-lora` works out what a LoRa link costs and how far it reaches, and a radio
chip has to be told all of it over SPI. The Semtech SX1261, SX1262, and SX1268, and
the LLCC68 that shares their command set, sit on boards such as the Heltec WiFi
LoRa 32 V3 and the Wio-SX1262 for XIAO, and take commands. The older SX1276 family,
inside the RFM95W, is driven through registers instead. pamoja drives both families
behind the same calls: tune the chip from a carrier, a link, and an output power,
then transmit, receive, and listen. It chooses the amplifier setting a regional
EIRP ceiling allows behind a given antenna, and holds the radio silent for the off
time a duty-cycle limit owes.

The same radio runs in every language. On a Linux board it opens a module through
the kernel's spidev and GPIO character devices. Anywhere else, and in tests, a
simulated chip stands in for the module: the driver talks to it exactly as it talks
to a real one, the program says what arrives on the air, and the chip reports what
it was tuned to and what it sent. Underneath, the command set, the register map,
and every decoder are there for a program that drives the chip over a bus of its
own.

## What the example does

It sends one reading from an SX1262 node: ten bytes on 868.1 MHz at DR3 of the
EU863-870 plan, through a 2.15 dBi whip on half a decibel of pigtail. It picks the
amplifier setting that keeps the EIRP under the plan's ceiling, tunes a simulated
SX1262 with it, and reads back what the chip was told. The reading goes out, the
duty-cycle guard records the silence it owes, a gateway's answer arrives from the
edge of range, and then nothing does, and then a frame whose CRC fails.

The second part does the same on an RFM95W, whose SX1276 is tuned through
registers. The chip's own synthesizer shows in what it reports: it steps in 61 Hz,
so it lands 24 Hz under 868.1 MHz. It gives a frame's strength in whole decibels,
and the part ends by asking whether an LLCC68 could carry the same data rates.

The third part opens a real radio: the same RFM95W on a Raspberry Pi's SPI bus. It
prints the channel and the power it would use, then opens the chip, which with
nothing wired says so instead of pretending.

It proves:

- The plan's 16 dBm EIRP ceiling, less 2.15 dBi of antenna gain and plus 0.5 dB of
  pigtail loss, leaves 14.35 dBm at the chip, so the amplifier is set to 14 dBm: a
  radio takes whole decibels, and rounding down keeps the ceiling.
- The driver tunes the chip to exactly what the plan's data rate names: 868.1 MHz,
  SF9 at 125 kHz, and 14 dBm, read back from the chip rather than from the program's
  own settings.
- Ten bytes at DR3 hold the air for 144,384 microseconds, and a 1% sub-band makes
  the next frame wait until a hundred times that has passed since this one started.
- An answer heard at -109 dBm with an SNR of -2.5 dB comes back with those levels.
- A reception with nothing on the air ends with a timeout, and a frame whose CRC
  fails is reported as corrupt and dropped, never handed over.
- An RFM95W behind the same whip takes the same 14 dBm on its PA_BOOST amplifier,
  and its carrier lands on the 61 Hz step nearest 868.1 MHz.
- The SX1276 reports the same answer at -109 dBm, and works out the signal itself
  at -111.5 dBm, 2.5 dB under, because it arrived below the noise.
- An LLCC68 carries DR3, at SF9, but not DR2, at SF10, the first rate it gives up
  at 125 kHz.
- Opening a radio resets the chip and reads its version back, so a swapped MISO and
  MOSI is caught there rather than on the first frame, and where no radio is wired
  it says so in a line.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example radios" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example radios</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- radios" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- radios</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/radios.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/radios.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- radios" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- radios</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-radios` holds each family in its own module, `sx126x` and
`sx127x`: the commands or registers, their decoders, the settings a link turns into,
and an `embedded-hal` driver. `radio::Radio` drives either family behind one set of
calls, configured from a `RadioConfig` of carrier, link, and output power, and
answers a reception with a `Reception`. `sim::Chip`, with the `sim` feature, is a
simulated chip whose `radio` is that same driver; `linux::open_sx126x` and
`open_sx127x`, with the `linux` feature, open a module on a board. Each family's
`config::TxPower::under_ceiling` picks the amplifier setting, and `duty::DutyCycle`
guards the air. Calls block for a transmission's airtime or a reception's timeout.

<!-- snippet: examples/guides/radios.rs#example -->
From [`examples/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/radios.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::radio::{RadioConfig, Reception};
use pamoja_radios::sim::Chip;
use pamoja_radios::sx126x::config::{PowerAmplifier, TxPower};
use pamoja_radios::sx126x::Board;

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

// A simulated SX1262 stands in for the chip on the node's board, driven by the same code
// that drives a real one, and it reports what that code told it.
let chip = Chip::sx126x(Board::new(PowerAmplifier::HighPower));
let mut radio = chip.radio();
radio.init()?;
radio.configure(RadioConfig::new(frequency, link, power.setting_dbm))?;
let tuned = chip.tuning();
println!(
    "tuned     {:.1} MHz, SF{} at {} kHz, {} dBm",
    f64::from(tuned.frequency_hz) / 1e6,
    tuned.link.spreading_factor(),
    tuned.link.bandwidth_hz() / 1000,
    tuned.output_dbm
);

// The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band
// that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long
// in silence before the next.
let reading = b"level=0.42";
let airtime = radio.transmit(reading)?;
println!(
    "sent      {} bytes, {airtime} us on air",
    chip.sent()[0].payload.len()
);
let permille = eu868
    .duty_cycle_permille(frequency)
    .expect("868.1 MHz is in a sub-band");
let mut guard = DutyCycle::new(permille);
guard.transmitted(0, &link, reading.len());
println!(
    "silence   the next frame starts {} us after this one did",
    guard.wait_us(0)
);

// A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
chip.hear(b"ack", Decibels::from_db(-109), Decibels::from_tenths(-25));
let mut buffer = [0u8; 255];
if let Reception::Frame { len, levels } = radio.receive(&mut buffer, 1_000_000)? {
    println!(
        "received  {} at {} dBm, SNR {} dB",
        String::from_utf8_lossy(&buffer[..len]),
        levels.rssi_dbm,
        levels.snr_db
    );
}

// With nothing on the air the reception times out, and a frame whose CRC fails is dropped
// rather than handed over.
let quiet = radio.receive(&mut buffer, 1_000_000)?;
chip.hear_corrupt(Decibels::from_db(-121), Decibels::from_db(-12));
let broken = radio.receive(&mut buffer, 1_000_000)?;
println!("then      {quiet:?}, then {broken:?}");
```
<!-- end -->

The same reading from an RFM95W:

<!-- snippet: examples/guides/radios.rs#rfm95w -->
From [`examples/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/radios.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::radio::{RadioConfig, Reception};
use pamoja_radios::sim::Chip;
use pamoja_radios::sx126x::config::{llcc68_supports, LoraModulation as Sx126xModulation};
use pamoja_radios::sx127x::config::{PaOutput, TxPower};
use pamoja_radios::sx127x::Board;

// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
// same 16 dBm ceiling leave it the same 14 dBm.
let band = Region::Eu868.plan();
let channel = 868_100_000;
let dr3 = band.link_settings(3).expect("DR3 is a LoRa data rate");
let antenna = LinkBudget {
    transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
    transmit_cable_loss_db: Decibels::from_tenths(5),
    ..LinkBudget::default()
};
let limit = Decibels::from_db(band.max_eirp_dbm(channel).into());
let rfm95w = TxPower::under_ceiling(PaOutput::PaBoost, &antenna, limit);

// The same driver calls tune it, through registers this time. Its synthesizer steps in
// 61 Hz, so the carrier lands on the step nearest the one asked for.
let chip = Chip::sx127x(Board::new(PaOutput::PaBoost));
let mut radio = chip.radio();
radio.init()?;
radio.configure(RadioConfig::new(channel, dr3, rfm95w.output_dbm))?;
let tuned = chip.tuning();
println!(
    "rfm95w    {} dBm on PA_BOOST, carrier {} Hz, {} Hz from {channel}",
    tuned.output_dbm,
    tuned.frequency_hz,
    channel.abs_diff(tuned.frequency_hz)
);

// The SX1276 gives a packet's strength in whole decibels, and works out the strength of
// the signal itself from the SNR when it arrived under the noise.
chip.hear(b"ack", Decibels::from_db(-109), Decibels::from_tenths(-25));
let mut buffer = [0u8; 255];
if let Reception::Frame { levels, .. } = radio.receive(&mut buffer, 1_000_000)? {
    println!(
        "received  RSSI {} dBm, SNR {} dB, signal {} dBm",
        levels.rssi_dbm, levels.snr_db, levels.signal_rssi_dbm
    );
}

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
let carries = |data_rate: u8| {
    let fits = band
        .link_settings(data_rate)
        .and_then(|link| Sx126xModulation::from_link(&link))
        .is_some_and(|modulation| {
            llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
        });
    if fits {
        "carries"
    } else {
        "cannot carry"
    }
};
println!("llcc68    {} DR3 and {} DR2", carries(3), carries(2));
```
<!-- end -->

The same radio opened on a Linux board:

<!-- snippet: examples/guides/radios.rs#hardware -->
From [`examples/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/radios.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::linux::{self, Wiring};
use pamoja_radios::radio::RadioConfig;
use pamoja_radios::sx127x::config::{PaOutput, TxPower};
use pamoja_radios::sx127x::Board;

// An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin
// on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
println!(
    "radio     an RFM95W on {}, reset on GPIO{}",
    wiring.spi.display(),
    wiring.reset_line
);

// The channel and the power the same whip leaves under the same ceiling, now as the number
// the radio is set to rather than the registers it goes into.
let band = Region::Eu868.plan();
let channel = 868_100_000;
let dr3 = band.link_settings(3).expect("DR3 is a LoRa data rate");
let antenna = LinkBudget {
    transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
    transmit_cable_loss_db: Decibels::from_tenths(5),
    ..LinkBudget::default()
};
let ceiling = Decibels::from_db(i32::from(band.max_eirp_dbm(channel)));
let rfm95w = TxPower::under_ceiling(PaOutput::PaBoost, &antenna, ceiling);
println!(
    "plan      {channel} Hz at DR3, {} dBm on PA_BOOST",
    rfm95w.output_dbm
);

// Opening resets the chip and reads its version back, so a wiring mistake is caught here
// rather than on the first frame. With no radio wired, this is the line that prints.
match linux::open_sx127x(&wiring, Board::new(PaOutput::PaBoost)) {
    Ok(mut radio) => {
        radio
            .configure(RadioConfig::new(channel, dr3, rfm95w.output_dbm))
            .expect("the chip takes the settings");
        let airtime_us = radio.transmit(b"21.5").expect("the frame goes out");
        println!("sent      a reading in {airtime_us} us on air");
    }
    Err(_) => println!("absent    no radio answered, so nothing went out"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/radios` exports `LoraRadio`, whose calls return promises and
wait on a worker thread rather than the event loop, and `SimulatedLoraChip`, whose
`radio()` gives the same class on any platform. `LoraRadio.openSx126x` and
`openSx127x` open a module on Linux. A configuration is a plain object,
`{ frequencyHz, link, outputDbm, syncWord? }`, and `receive` resolves an object whose
`outcome` is `Frame`, `Timeout`, or `Corrupt`, with the payload and levels of a
frame. The `sx126x` and `sx127x` namespaces hold `txPowerUnderCeiling`, the command
and register builders, and the decoders; `DutyCycle` guards the air, and `close`
releases a radio.

<!-- snippet: bindings/node/guides/radios.ts#example -->
From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { LoraRegion, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, SimulatedLoraChip, sx126x } from '@pamoja/radios'

async function onTheBench() {
  // An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
  // 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the
  // antenna and pigtail decide how hard the amplifier may drive under that cap.
  const eu868 = planFor(LoraRegion.Eu868)
  const frequency = 868_100_000
  const link = eu868.linkSettings(3)!
  const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const ceiling = eu868.maxEirpDbm(frequency)
  const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
  console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

  // A simulated SX1262 stands in for the chip on the node's board, driven by the same code
  // that drives a real one, and it reports what that code told it.
  const chip = SimulatedLoraChip.sx126x({ amplifier: sx126x.Amplifier.HighPower })
  const radio = chip.radio()
  await radio.configure({ frequencyHz: frequency, link, outputDbm: power.settingDbm })
  const tuned = chip.tuning()
  console.log(
    `tuned     ${(tuned.frequencyHz / 1e6).toFixed(1)} MHz, SF${tuned.link.spreadingFactor} ` +
      `at ${tuned.link.bandwidthHz / 1000} kHz, ${tuned.outputDbm} dBm`,
  )

  // The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band
  // that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long
  // in silence before the next.
  const reading = Buffer.from('level=0.42')
  const airtime = await radio.transmit(reading)
  console.log(`sent      ${chip.sent()[0].payload.length} bytes, ${airtime} us on air`)
  const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
  guard.transmitted(0, link, reading.length)
  console.log(`silence   the next frame starts ${guard.waitUs(0)} us after this one did`)

  // A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
  chip.hear(Buffer.from('ack'), -109, -2.5)
  const heard = await radio.receive(1_000_000)
  if (heard.outcome === 'Frame') {
    console.log(
      `received  ${heard.payload!.toString()} at ${heard.rssiDbm!.toFixed(2)} dBm, ` +
        `SNR ${heard.snrDb!.toFixed(2)} dB`,
    )
  }

  // With nothing on the air the reception times out, and a frame whose CRC fails is dropped
  // rather than handed over.
  const quiet = (await radio.receive(1_000_000)).outcome
  chip.hearCorrupt(-121, -12)
  const broken = (await radio.receive(1_000_000)).outcome
  console.log(`then      ${quiet}, then ${broken}`)
  radio.close()
  return { power, tuned, airtime, link, reading, guard, chip, quiet, broken }
}
```
<!-- end -->

The same reading from an RFM95W:

<!-- snippet: bindings/node/guides/radios.ts#rfm95w -->
From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { sx127x } from '@pamoja/radios'

async function anRfm95w() {
  // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
  // same 16 dBm ceiling leave it the same 14 dBm.
  const band = planFor(LoraRegion.Eu868)
  const channel = 868_100_000
  const dr3 = band.linkSettings(3)!
  const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const rfm95w = sx127x.txPowerUnderCeiling(
    sx127x.PaOutput.PaBoost,
    antenna,
    band.maxEirpDbm(channel),
  )

  // The same driver calls tune it, through registers this time. Its synthesizer steps in
  // 61 Hz, so the carrier lands on the step nearest the one asked for.
  const chip = SimulatedLoraChip.sx127x({ output: sx127x.PaOutput.PaBoost })
  const radio = chip.radio()
  await radio.configure({ frequencyHz: channel, link: dr3, outputDbm: rfm95w.outputDbm })
  const tuned = chip.tuning()
  const off = Math.abs(channel - tuned.frequencyHz)
  console.log(
    `rfm95w    ${tuned.outputDbm} dBm on PA_BOOST, carrier ${tuned.frequencyHz} Hz, ` +
      `${off} Hz from ${channel}`,
  )

  // The SX1276 gives a packet's strength in whole decibels, and works out the strength of
  // the signal itself from the SNR when it arrived under the noise.
  chip.hear(Buffer.from('ack'), -109, -2.5)
  const heard = await radio.receive(1_000_000)
  if (heard.outcome === 'Frame') {
    console.log(
      `received  RSSI ${heard.rssiDbm!.toFixed(2)} dBm, SNR ${heard.snrDb!.toFixed(2)} dB, ` +
        `signal ${heard.signalRssiDbm!.toFixed(2)} dBm`,
    )
  }
  radio.close()

  // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
  const carries = (dataRate: number): string =>
    sx126x.llcc68Supports(band.linkSettings(dataRate)!) ? 'carries' : 'cannot carry'
  console.log(`llcc68    ${carries(3)} DR3 and ${carries(2)} DR2`)
  return { rfm95w, tuned, channel, carries }
}
```
<!-- end -->

The same radio opened on a Linux board:

<!-- snippet: bindings/node/guides/radios.ts#hardware -->
From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { LoraRadio } from '@pamoja/radios'

async function onALinuxBoard(): Promise<void> {
  // An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin
  // on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
  const wiring = { spi: '/dev/spidev0.0', gpioChip: '/dev/gpiochip0', resetLine: 25 }
  console.log(`radio     an RFM95W on ${wiring.spi}, reset on GPIO${wiring.resetLine}`)

  // The channel and the power the same whip leaves under the same ceiling, now as the number
  // the radio is set to rather than the registers it goes into.
  const band = planFor(LoraRegion.Eu868)
  const channel = 868_100_000
  const dr3 = band.linkSettings(3)!
  const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const ceiling = band.maxEirpDbm(channel)
  const rfm95w = sx127x.txPowerUnderCeiling(sx127x.PaOutput.PaBoost, antenna, ceiling)
  console.log(`plan      ${channel} Hz at DR3, ${rfm95w.outputDbm} dBm on PA_BOOST`)

  // Opening resets the chip and reads its version back, so a wiring mistake is caught here
  // rather than on the first frame. With no radio wired, this is the line that prints.
  let radio
  try {
    radio = LoraRadio.openSx127x(wiring, { output: sx127x.PaOutput.PaBoost })
  } catch {
    console.log('absent    no radio answered, so nothing went out')
    return
  }
  try {
    await radio.configure({ frequencyHz: channel, link: dr3, outputDbm: rfm95w.outputDbm })
    const airtimeUs = await radio.transmit(Buffer.from('21.5'))
    console.log(`sent      a reading in ${airtimeUs} us on air`)
  } finally {
    radio.close()
  }
}
```
<!-- end -->

## Python

In Python, `pamoja.radios` exports `LoraRadio` and `SimulatedLoraChip`. A radio's
calls block, releasing the interpreter lock while the chip works, and a radio closes
with `close` or at the end of a `with` block. `configure` takes the carrier, the
link, and the output power, with the sync word and the IQ polarity as keywords, and
`receive` returns a `LoraReception` whose `outcome` is `"Frame"`, `"Timeout"`, or
`"Corrupt"`. The `sx126x` and `sx127x` modules hold `tx_power_under_ceiling`, the
builders, and the decoders, and `DutyCycle` guards the air. Amplifier and output
names are strings or the modules' enums.

<!-- snippet: bindings/python/guides/radios.py#example -->
From [`bindings/python/guides/radios.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/radios.py):

```python
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, SimulatedLoraChip, sx126x

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

# A simulated SX1262 stands in for the chip on the node's board, driven by the same code that
# drives a real one, and it reports what that code told it.
chip = SimulatedLoraChip.sx126x(sx126x.Amplifier.HIGH_POWER)
bench = chip.radio()
bench.configure(frequency, link, power.setting_dbm)
tuned = chip.tuning()
print(
    f"tuned     {tuned.frequency_hz / 1e6:.1f} MHz, SF{tuned.link.spreading_factor} "
    f"at {tuned.link.bandwidth_hz // 1000} kHz, {tuned.output_dbm} dBm"
)

# The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band that
# holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long in silence
# before the next.
reading = b"level=0.42"
airtime = bench.transmit(reading)
print(f"sent      {len(chip.sent()[0].payload)} bytes, {airtime} us on air")
guard = DutyCycle(eu868.duty_cycle_permille(frequency))
guard.transmitted(0, link, len(reading))
print(f"silence   the next frame starts {guard.wait_us(0)} us after this one did")

# A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
chip.hear(b"ack", -109, -2.5)
heard = bench.receive(1_000_000)
if heard.outcome == "Frame":
    print(
        f"received  {heard.payload.decode()} at {heard.rssi_dbm:.2f} dBm, "
        f"SNR {heard.snr_db:.2f} dB"
    )

# With nothing on the air the reception times out, and a frame whose CRC fails is dropped
# rather than handed over.
quiet = bench.receive(1_000_000).outcome
chip.hear_corrupt(-121, -12)
broken = bench.receive(1_000_000).outcome
print(f"then      {quiet}, then {broken}")
bench.close()
```
<!-- end -->

The same reading from an RFM95W:

<!-- snippet: bindings/python/guides/radios.py#rfm95w -->
From [`bindings/python/guides/radios.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/radios.py):

```python
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import SimulatedLoraChip, sx126x, sx127x

# An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
# 16 dBm ceiling leave it the same 14 dBm.
band = plan_for("EU868")
channel = 868_100_000
dr3 = band.link_settings(3)
antenna = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
rfm95w = sx127x.tx_power_under_ceiling(
    sx127x.PaOutput.PA_BOOST, antenna, band.max_eirp_dbm(channel)
)

# The same driver calls tune it, through registers this time. Its synthesizer steps in 61 Hz,
# so the carrier lands on the step nearest the one asked for.
module = SimulatedLoraChip.sx127x(sx127x.PaOutput.PA_BOOST)
radio = module.radio()
radio.configure(channel, dr3, rfm95w.output_dbm)
carrier = module.tuning()
print(
    f"rfm95w    {carrier.output_dbm} dBm on PA_BOOST, carrier {carrier.frequency_hz} Hz, "
    f"{abs(channel - carrier.frequency_hz)} Hz from {channel}"
)

# The SX1276 gives a packet's strength in whole decibels, and works out the strength of the
# signal itself from the SNR when it arrived under the noise.
module.hear(b"ack", -109, -2.5)
packet = radio.receive(1_000_000)
if packet.outcome == "Frame":
    print(
        f"received  RSSI {packet.rssi_dbm:.2f} dBm, SNR {packet.snr_db:.2f} dB, "
        f"signal {packet.signal_rssi_dbm:.2f} dBm"
    )
radio.close()


def carries(data_rate: int) -> str:
    """Whether an LLCC68 in the RFM95W's place could carry a data rate: DR3, but not DR2,
    which is SF10 at 125 kHz."""
    return "carries" if sx126x.llcc68_supports(band.link_settings(data_rate)) else "cannot carry"


print(f"llcc68    {carries(3)} DR3 and {carries(2)} DR2")
```
<!-- end -->

The same radio opened on a Linux board:

<!-- snippet: bindings/python/guides/radios.py#hardware -->
From [`bindings/python/guides/radios.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/radios.py):

```python
from pamoja.core import PamojaError
from pamoja.radios import LoraRadio

# An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin on
# GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
spi, gpio_chip, reset_line = "/dev/spidev0.0", "/dev/gpiochip0", 25
print(f"radio     an RFM95W on {spi}, reset on GPIO{reset_line}")
print(f"plan      {channel} Hz at DR3, {rfm95w.output_dbm} dBm on PA_BOOST")

# Opening resets the chip and reads its version back, so a wiring mistake is caught here rather
# than on the first frame. With no radio wired, this is the line that prints.
try:
    radio = LoraRadio.open_sx127x(spi, gpio_chip, reset_line, sx127x.PaOutput.PA_BOOST)
except PamojaError:
    radio = None

if radio is None:
    print("absent    no radio answered, so nothing went out")
else:
    with radio:
        radio.configure(channel, dr3, rfm95w.output_dbm)
        airtime_us = radio.transmit(b"21.5")
        print(f"sent      a reading in {airtime_us} us on air")
```
<!-- end -->

## C#

In C#, `Pamoja.Radios` holds `LoraRadio` and `SimulatedLoraChip`, both disposable.
`LoraRadio.OpenSx126x` and `OpenSx127x` open a module on Linux and throw
`PlatformNotSupportedException` elsewhere, and `SimulatedLoraChip.Radio` gives the
same class on any platform. `Configure` takes a `LoraRadioConfig` record with the
sync word and IQ polarity as init properties, and `Receive` returns a
`LoraReception` with its `LoraReceptionOutcome`. The static `Sx126x` and `Sx127x`
classes hold `TxPowerUnderCeiling`, the builders, and the decoders, and
`RadioDutyCycle` guards the air.

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

// A simulated SX1262 stands in for the chip on the node's board, driven by the same
// code that drives a real one, and it reports what that code told it.
using SimulatedLoraChip chip = SimulatedLoraChip.Sx126x(new Sx126xBoard(Sx126xAmplifier.HighPower));
using LoraRadio bench = chip.Radio();
bench.Configure(new LoraRadioConfig(Frequency, link, power.SettingDbm));
LoraTuning tuned = chip.Tuning();
Console.WriteLine(Invariant(
    $"tuned     {tuned.FrequencyHz / 1e6:F1} MHz, SF{tuned.Link.SpreadingFactor} at {tuned.Link.BandwidthHz / 1000} kHz, {tuned.OutputDbm} dBm"));

// The reading goes out, and the airtime comes back for the duty-cycle guard. The
// sub-band that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine
// times as long in silence before the next.
byte[] reading = "level=0.42"u8.ToArray();
ulong airtime = bench.Transmit(reading);
Console.WriteLine($"sent      {chip.Sent()[0].Payload.Length} bytes, {airtime} us on air");
using var guard = new RadioDutyCycle(eu868.DutyCyclePermille(Frequency)!.Value);
guard.Transmitted(0, link, reading.Length);
Console.WriteLine($"silence   the next frame starts {guard.WaitMicros(0)} us after this one did");

// A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
chip.Hear("ack"u8, -109, -2.5);
LoraReception heard = bench.Receive(TimeSpan.FromSeconds(1));
if (heard.Outcome == LoraReceptionOutcome.Frame)
{
    Console.WriteLine(Invariant(
        $"received  {Encoding.UTF8.GetString(heard.Payload!)} at {heard.RssiDbm:F2} dBm, SNR {heard.SnrDb:F2} dB"));
}

// With nothing on the air the reception times out, and a frame whose CRC fails is
// dropped rather than handed over.
LoraReceptionOutcome quiet = bench.Receive(TimeSpan.FromSeconds(1)).Outcome;
chip.HearCorrupt(-121, -12);
LoraReceptionOutcome broken = bench.Receive(TimeSpan.FromSeconds(1)).Outcome;
Console.WriteLine($"then      {quiet}, then {broken}");
```
<!-- end -->

The same reading from an RFM95W:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs#rfm95w -->
From [`bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs):

```csharp
// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and
// the same 16 dBm ceiling leave it the same 14 dBm.
using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
const uint Channel = 868_100_000;
LoraLink dr3 = band.LinkSettings(3)!;
var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
    Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));

// The same driver calls tune it, through registers this time. Its synthesizer steps
// in 61 Hz, so the carrier lands on the step nearest the one asked for.
using SimulatedLoraChip module = SimulatedLoraChip.Sx127x(new Sx127xBoard(Sx127xPaOutput.PaBoost));
using LoraRadio radio = module.Radio();
radio.Configure(new LoraRadioConfig(Channel, dr3, rfm95w.OutputDbm));
LoraTuning carrier = module.Tuning();
uint off = carrier.FrequencyHz > Channel ? carrier.FrequencyHz - Channel : Channel - carrier.FrequencyHz;
Console.WriteLine(
    $"rfm95w    {carrier.OutputDbm} dBm on PA_BOOST, carrier {carrier.FrequencyHz} Hz, {off} Hz from {Channel}");

// The SX1276 gives a packet's strength in whole decibels, and works out the strength
// of the signal itself from the SNR when it arrived under the noise.
module.Hear("ack"u8, -109, -2.5);
LoraReception packet = radio.Receive(TimeSpan.FromSeconds(1));
if (packet.Outcome == LoraReceptionOutcome.Frame)
{
    Console.WriteLine(Invariant(
        $"received  RSSI {packet.RssiDbm:F2} dBm, SNR {packet.SnrDb:F2} dB, signal {packet.SignalRssiDbm:F2} dBm"));
}

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
string Carries(byte dataRate) =>
    Sx126x.Llcc68Supports(band.LinkSettings(dataRate)!) ? "carries" : "cannot carry";
Console.WriteLine($"llcc68    {Carries(3)} DR3 and {Carries(2)} DR2");
```
<!-- end -->

The same radio opened on a Linux board:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs#hardware -->
From [`bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs):

```csharp
// An RFM95W on a Raspberry Pi: the header's first chip select, with the module's
// reset pin on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
var wiring = new LoraRadioWiring("/dev/spidev0.0", "/dev/gpiochip0", 25);
Console.WriteLine($"radio     an RFM95W on {wiring.Spi}, reset on GPIO{wiring.ResetLine}");

// The channel and the power the same whip leaves under the same ceiling, now as the
// number the radio is set to rather than the registers it goes into.
using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
const uint Channel = 868_100_000;
LoraLink dr3 = band.LinkSettings(3)!;
var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
    Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));
Console.WriteLine($"plan      {Channel} Hz at DR3, {rfm95w.OutputDbm} dBm on PA_BOOST");

// Opening resets the chip and reads its version back, so a wiring mistake is caught
// here rather than on the first frame. With no radio wired, this line prints.
LoraRadio? radio = null;
try
{
    radio = LoraRadio.OpenSx127x(wiring, new Sx127xBoard(Sx127xPaOutput.PaBoost));
}
catch (PlatformNotSupportedException)
{
}
catch (PamojaException)
{
}

if (radio is null)
{
    Console.WriteLine("absent    no radio answered, so nothing went out");
    return;
}

using (radio)
{
    radio.Configure(new LoraRadioConfig(Channel, dr3, rfm95w.OutputDbm));
    ulong airtimeUs = radio.Transmit("21.5"u8);
    Console.WriteLine($"sent      a reading in {airtimeUs} us on air");
}
```
<!-- end -->

## Values at a glance

**The two families:**

| | SX126x | SX127x |
| --- | --- | --- |
| Chips | SX1261, SX1262, SX1268, LLCC68 | SX1276, SX1277, SX1278, SX1279 |
| Modules | Heltec WiFi LoRa 32 V3, Wio-SX1262 for XIAO | RFM95W |
| Driven by | commands | registers |
| BUSY line | yes, the wiring names it | none |
| Amplifier | low power, -17 to +14 dBm (SX1261); high power, -9 to +22 dBm | RFO, -4 to +15 dBm; PA_BOOST, +2 to +20 dBm |
| Spreading factors | SF5 to SF12 | SF6 to SF12, SF6 with an implicit header |
| Carrier step | under a hertz | 61 Hz |
| A frame's strength | to half a decibel | to a decibel |

An LLCC68 takes the SX1262's commands but not all its rates: up to SF9 at 125 kHz,
SF10 at 250 kHz, and SF11 at 500 kHz.

**A radio's calls,** the same for either family:

| Call | What it does |
| --- | --- |
| configure | tunes the chip to a carrier, a link, and an output power, with the sync word and IQ polarity |
| transmit | sends one frame and returns its airtime once it has left |
| receive | listens for one frame for up to a timeout |
| listen, then take a frame | listens frame after frame, handing each over when asked |
| detect | listens a few symbols for a preamble, as a relay's scan does |
| random | draws thirty-two bits from the receiver's noise |
| standby, sleep | stops the chip, or puts it to sleep until the next call |
| read and write a register | reaches anything the calls above do not |

**What a reception ends with:**

| Outcome | Means |
| --- | --- |
| Frame | a frame checked, with its payload, RSSI, SNR, and the signal's own strength |
| Timeout | nothing arrived before the timeout |
| Corrupt | a frame arrived whose header or CRC failed, and was dropped |
| Nothing | a listening radio has nothing new to hand over |

**The sync words:**

| Byte | For |
| --- | --- |
| 0x34 | a public network, such as LoRaWAN |
| 0x12 | a private network, and both families' value out of reset |

**A simulated chip** takes what a module takes and answers as it would, with no
timing: a transmission is done as soon as it starts, and a reception with a timeout
ends at once when nothing waits on the air.

| To | Call |
| --- | --- |
| make one | a chip of either family from its board, out of reset |
| drive it | its radio, the same class a module opens as |
| put a frame on the air | hear, with the strength and SNR it arrives at; hear a corrupt one |
| check the program | the tuning it holds now, and every frame it sent with the tuning of the moment |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| simulate a chip | `sim::Chip::sx126x(board)`, `Chip::sx127x(board)`, then `radio()` |
| open a module | `linux::open_sx126x(&wiring, board)`, `linux::open_sx127x(&wiring, board)` |
| tune it | `configure(RadioConfig::new(hz, link, dbm))`, `with_sync_word`, `with_inverted_iq`, `lorawan_device()` |
| send and listen | `transmit(payload)`, `receive(&mut buffer, timeout_us)`, `listen()`, `take_frame(&mut buffer)` |
| pick the power | `sx126x::config::TxPower::under_ceiling(amplifier, &budget, ceiling)`, `sx127x::config::TxPower::under_ceiling(output, ..)` |
| guard the air | `DutyCycle::new(permille)`, `transmitted(start_us, &link, len)`, `wait_us(now_us)` |
| watch the chip | `hear(payload, rssi, snr)`, `hear_corrupt(rssi, snr)`, `tuning()`, `sent()` |

### TypeScript

| To | Call |
| --- | --- |
| simulate a chip | `SimulatedLoraChip.sx126x({ amplifier })`, `SimulatedLoraChip.sx127x({ output })`, then `radio()` |
| open a module | `LoraRadio.openSx126x(wiring, board)`, `LoraRadio.openSx127x(wiring, board)` |
| tune it | `await configure({ frequencyHz, link, outputDbm, syncWord? })` |
| send and listen | `await transmit(payload)`, `await receive(timeoutUs)`, `await listen()`, `await takeFrame()` |
| pick the power | `sx126x.txPowerUnderCeiling(amplifier, budget, ceiling)`, `sx127x.txPowerUnderCeiling(output, budget, ceiling)` |
| guard the air | `new DutyCycle(permille)`, `transmitted(startUs, link, len)`, `waitUs(nowUs)` |
| watch the chip | `hear(payload, rssiDbm, snrDb)`, `hearCorrupt(rssiDbm, snrDb)`, `tuning()`, `sent()` |

### Python

| To | Call |
| --- | --- |
| simulate a chip | `SimulatedLoraChip.sx126x(amplifier)`, `SimulatedLoraChip.sx127x(output)`, then `radio()` |
| open a module | `LoraRadio.open_sx126x(spi, gpio_chip, busy_line, reset_line, amplifier)`, `LoraRadio.open_sx127x(spi, gpio_chip, reset_line, output)` |
| tune it | `configure(frequency_hz, link, output_dbm, sync_word=None)` |
| send and listen | `transmit(payload)`, `receive(timeout_us)`, `listen()`, `take_frame()` |
| pick the power | `sx126x.tx_power_under_ceiling(amplifier, budget, ceiling)`, `sx127x.tx_power_under_ceiling(output, budget, ceiling)` |
| guard the air | `DutyCycle(permille)`, `transmitted(start_us, link, len)`, `wait_us(now_us)` |
| watch the chip | `hear(payload, rssi_dbm, snr_db)`, `hear_corrupt(rssi_dbm, snr_db)`, `tuning()`, `sent()` |

### C#

| To | Call |
| --- | --- |
| simulate a chip | `SimulatedLoraChip.Sx126x(board)`, `SimulatedLoraChip.Sx127x(board)`, then `Radio()` |
| open a module | `LoraRadio.OpenSx126x(wiring, board)`, `LoraRadio.OpenSx127x(wiring, board)` |
| tune it | `Configure(new LoraRadioConfig(hz, link, dbm) { SyncWord = ... })` |
| send and listen | `Transmit(payload)`, `Receive(timeout)`, `Listen()`, `TakeFrame()` |
| pick the power | `Sx126x.TxPowerUnderCeiling(amplifier, budget, ceiling)`, `Sx127x.TxPowerUnderCeiling(output, budget, ceiling)` |
| guard the air | `new RadioDutyCycle(permille)`, `Transmitted(start, link, length)`, `WaitMicros(now)` |
| watch the chip | `Hear(payload, rssiDbm, snrDb)`, `HearCorrupt(rssiDbm, snrDb)`, `Tuning()`, `Sent()` |

<!-- languages end -->

## When it goes wrong

A radio refuses what the chip cannot do before it sends a byte, and says why. The
mistakes that cost an afternoon:

- **Opening a radio throws off Linux.** Only Linux has spidev and the GPIO
  character device. Anywhere else, and in tests, use a simulated chip: its radio is
  the same class, with the same calls.
- **Opening finds no chip.** The reset pulse and the version check come first, so a
  wrong SPI device, a wrong reset line, swapped MISO and MOSI, or an unpowered
  module is caught when the radio opens. An SX126x also needs its BUSY line named.
- **The radio refuses a configuration.** The chip lacks the setting: a bandwidth it
  does not have, a rate an LLCC68 gives up such as SF10 at 125 kHz, an SX1276 below
  SF6, or power settings made for the other amplifier. Take the power from the
  board's own amplifier, as the example does.
- **A transmission is refused.** A radio sends nothing until it is configured, and
  one frame carries at most 255 bytes.
- **Two radios hear nothing from each other.** Both ends must agree on the carrier,
  the spreading factor, the bandwidth, the sync word, and the IQ polarity. A
  LoRaWAN device uses the public sync word and hears downlinks with inverted IQ. In
  Rust `lorawan_device` sets both; elsewhere the configuration names them.
- **The radiated power is over the ceiling.** The ceiling limits what leaves the
  antenna, so an antenna with gain needs less from the amplifier. Pick the setting
  with the family's under-ceiling call, which rounds down.
- **The next frame is held back.** The duty-cycle guard counts from the start of the
  last frame. Ask how long it waits rather than sending again at once.
- **A simulated reception times out at once.** Nothing is timed on a simulated
  chip, so a frame has to be on the air before the radio listens. Put it there with
  `hear` first.

## Where next

<!-- table: next radios -->
- [LoRa airtime and range](lora.md): Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range.
- [LoRaWAN gateways](gateway.md): What a LoRaWAN gateway speaks.
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- Beside it: [Radios and antennas](../radio.md), [Hardware](../hardware.md).
- Also in Radio and reach: [LoRaWAN](lorawan.md), [Mesh frames](mesh.md), [Routing](routing.md).
<!-- end -->

## Reference

<!-- table: reference radios -->
- Rust: [`pamoja-radios`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios)
- TypeScript: [`@pamoja/radios`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios)
- Python: [`pamoja.radios`](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios)
- C#: [`Pamoja.Radios`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios)
- Hardware: [SX1276](https://pamoja.molex.cloud/docs/hardware.html#sx1276), [SX1262](https://pamoja.molex.cloud/docs/hardware.html#sx1262), [LLCC68](https://pamoja.molex.cloud/docs/hardware.html#llcc68), [SX1302](https://pamoja.molex.cloud/docs/hardware.html#sx1302), [SX1303](https://pamoja.molex.cloud/docs/hardware.html#sx1303), [SX1250](https://pamoja.molex.cloud/docs/hardware.html#sx1250), [RAK2287 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak2287), [RAK5146 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak5146), [WM1302 LoRaWAN gateway module](https://pamoja.molex.cloud/docs/hardware.html#wm1302)
<!-- end -->
