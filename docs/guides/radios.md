# LoRa radios

`pamoja-lora` works out what a LoRa link costs and how far it reaches, and a radio
chip has to be told all of it, one SPI command at a time. The Semtech SX1261,
SX1262, and SX1268, and the LLCC68 that shares their command set, sit on boards
such as the Heltec WiFi LoRa 32 V3 and the Wio-SX1262 for XIAO. pamoja builds the
bytes of every command they take and decodes every answer they give, chooses the
amplifier setting a regional EIRP ceiling allows behind a given antenna, and holds
the radio silent for the off time a duty-cycle limit owes. The older SX1276 family,
inside the RFM95W, is driven through registers instead, and for it pamoja gives the
value each register takes and decodes what the chip reads back.

In Rust, `pamoja-radios` also drives either family over any `embedded-hal` SPI bus
and carries mesh frames on it as a pamoja transport. The other languages bind the
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

The third part opens a radio rather than planning for one: the same RFM95W on a
Raspberry Pi's SPI bus, reached through the kernel's spidev and GPIO character
devices. It prints the channel and the power it would use, then opens the chip,
which with nothing wired says so instead of pretending.

The second part plans the same reading on an RFM95W, whose SX1276 is driven
through registers rather than commands: the amplifier settings on its PA_BOOST
output, the carrier and modem registers, the transmit mode, a received packet's
interrupts and signal levels, and whether an LLCC68 could carry the same data rate.

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
- An RFM95W behind the same whip takes the same 14 dBm on its PA_BOOST amplifier,
  which RegPaConfig carries as OutputPower 12 with RegPaDac at its default and the
  100 mA current limit.
- DR3 turns into RegModemConfig1 0x72, RegModemConfig2 0x94 and RegModemConfig3
  0x04, and TX mode on the LoRa register page is RegOpMode 0x8B.
- A packet read back with PacketSnr 0xF6 and PacketRssi 0x30 at 868.1 MHz was
  heard at -109 dBm with an SNR of -2.5 dB, so its own strength was -111.5 dBm.
- An LLCC68 carries DR3, at SF9, but not DR2, at SF10, the first rate it gives up
  at 125 kHz.
- Opening a radio resets the chip and reads its version back, so a swapped MISO and
  MOSI is caught there rather than on the first frame.
- Where no radio is wired, and on every platform that has no spidev, opening one
  says so in a line rather than failing later or pretending it worked.

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

The same reading from an RFM95W:

<!-- snippet: examples/tests/guides/radios.rs#rfm95w -->
From [`examples/tests/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/radios.rs):

```rust
use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::region::Region;
use pamoja_radios::sx126x::config::{llcc68_supports, LoraModulation as Sx126xModulation};
use pamoja_radios::sx127x::config::{frequency_word, LoraModulation, PaOutput, TxPower};
use pamoja_radios::sx127x::irq::IrqFlags;
use pamoja_radios::sx127x::register::{lora_op_mode, Mode};
use pamoja_radios::sx127x::status::{PacketStatus, Port};

// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
// same 16 dBm ceiling leave it the same 14 dBm, set through three registers.
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
println!(
    "rfm95w    {} dBm on PA_BOOST: RegPaConfig {:02x}, RegPaDac {:02x}, RegOcp {:02x}",
    rfm95w.output_dbm, rfm95w.pa_config, rfm95w.pa_dac, rfm95w.ocp
);

// The carrier and the modem go into registers while the chip stands by, and TX mode sends
// the frame the FIFO holds.
let modem = LoraModulation::from_link(&dr3).expect("DR3 fits an SX1276");
println!("carrier   RegFrf {:06x}", frequency_word(channel));
println!(
    "modem     RegModemConfig {:02x} {:02x} {:02x}",
    modem.modem_config_1(),
    modem.modem_config_2(0),
    modem.modem_config_3()
);
println!("tx mode   RegOpMode {:02x}", lora_op_mode(Mode::Tx));

// A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give
// its levels on the high frequency port.
let flags = IrqFlags::from_bits(0x50);
let received = flags.contains(IrqFlags::RX_DONE);
let corrupt = flags.contains(IrqFlags::PAYLOAD_CRC_ERROR);
println!("irq       rx done {received}, crc error {corrupt}");
let packet = PacketStatus::from_bytes([0xF6, 0x30], Port::for_frequency(channel));
let db = |value: Decibels| f64::from(value.hundredths()) / 100.0;
println!(
    "received  RSSI {} dBm, SNR {} dB, signal {} dBm",
    db(packet.rssi_dbm),
    db(packet.snr_db),
    db(packet.signal_rssi_dbm)
);

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
let fits = |data_rate: u8| {
    band.link_settings(data_rate)
        .and_then(|link| Sx126xModulation::from_link(&link))
        .is_some_and(|modulation| {
            llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
        })
};
println!("llcc68    DR3 {}, DR2 {}", fits(3), fits(2));
```
<!-- end -->

The same radio opened on a Linux board:

<!-- snippet: examples/tests/guides/radios.rs#hardware -->
From [`examples/tests/guides/radios.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/radios.rs):

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

The same reading from an RFM95W:

<!-- snippet: bindings/node/guides/radios.ts#rfm95w -->
From [`bindings/node/guides/radios.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/radios.ts):

```typescript
import { sx127x } from '@pamoja/radios'

// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
// 16 dBm ceiling leave it the same 14 dBm, set through three registers.
const band = planFor(LoraRegion.Eu868)
const channel = 868_100_000
const dr3 = band.linkSettings(3)!
const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
const rfm95w = sx127x.txPowerUnderCeiling(sx127x.PaOutput.PaBoost, antenna, band.maxEirpDbm(channel))
const byte = (value: number): string => value.toString(16).padStart(2, '0')
console.log(
  `rfm95w    ${rfm95w.outputDbm} dBm on PA_BOOST: RegPaConfig ${byte(rfm95w.paConfig)}, ` +
    `RegPaDac ${byte(rfm95w.paDac)}, RegOcp ${byte(rfm95w.ocp)}`,
)

// The carrier and the modem go into registers while the chip stands by, and TX mode sends
// the frame the FIFO holds.
const modem = sx127x.modem(dr3, channel)
console.log(`carrier   RegFrf ${sx127x.frequencyWord(channel).toString(16).padStart(6, '0')}`)
console.log(
  `modem     RegModemConfig ${byte(modem.modemConfig1)} ${byte(modem.modemConfig2)} ` +
    `${byte(modem.modemConfig3)}`,
)
console.log(`tx mode   RegOpMode ${byte(sx127x.loraOpMode(sx127x.Mode.Tx))}`)

// A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give
// its levels on the high frequency port.
const flags = 0x50
const received = (flags & sx127x.Irq.RxDone) !== 0
const corrupt = (flags & sx127x.Irq.PayloadCrcError) !== 0
console.log(`irq       rx done ${received}, crc error ${corrupt}`)
const packet = sx127x.packetStatus(Buffer.from([0xf6, 0x30]), channel)
console.log(
  `received  RSSI ${packet.rssiDbm} dBm, SNR ${packet.snrDb} dB, signal ${packet.signalRssiDbm} dBm`,
)

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
const fits = (dataRate: number): boolean => sx126x.llcc68Supports(band.linkSettings(dataRate)!)
console.log(`llcc68    DR3 ${fits(3)}, DR2 ${fits(2)}`)
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

The same reading from an RFM95W:

<!-- snippet: bindings/python/guides/radios.py#rfm95w -->
From [`bindings/python/guides/radios.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/radios.py):

```python
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import sx126x, sx127x

# An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
# 16 dBm ceiling leave it the same 14 dBm, set through three registers.
band = plan_for("EU868")
channel = 868_100_000
dr3 = band.link_settings(3)
antenna = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
rfm95w = sx127x.tx_power_under_ceiling(
    sx127x.PaOutput.PA_BOOST, antenna, band.max_eirp_dbm(channel)
)
print(
    f"rfm95w    {rfm95w.output_dbm} dBm on PA_BOOST: RegPaConfig {rfm95w.pa_config:02x}, "
    f"RegPaDac {rfm95w.pa_dac:02x}, RegOcp {rfm95w.ocp:02x}"
)

# The carrier and the modem go into registers while the chip stands by, and TX mode sends the
# frame the FIFO holds.
modem = sx127x.modem(dr3, channel)
print(f"carrier   RegFrf {sx127x.frequency_word(channel):06x}")
print(
    f"modem     RegModemConfig {modem.modem_config_1:02x} {modem.modem_config_2:02x} "
    f"{modem.modem_config_3:02x}"
)
print(f"tx mode   RegOpMode {sx127x.lora_op_mode(sx127x.Mode.TX):02x}")

# A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give its
# levels on the high frequency port.
flags = sx127x.Irq(0x50)
received = sx127x.Irq.RX_DONE in flags
corrupt = sx127x.Irq.PAYLOAD_CRC_ERROR in flags
print(f"irq       rx done {received}, crc error {corrupt}")
packet = sx127x.packet_status(bytes([0xF6, 0x30]), channel)
print(
    f"received  RSSI {packet.rssi_dbm} dBm, SNR {packet.snr_db} dB, "
    f"signal {packet.signal_rssi_dbm} dBm"
)


# An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
def fits(data_rate: int) -> bool:
    return sx126x.llcc68_supports(band.link_settings(data_rate))


print(f"llcc68    DR3 {fits(3)}, DR2 {fits(2)}")
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

The same reading from an RFM95W:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs#rfm95w -->
From [`bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RadiosGuide.cs):

```csharp
// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and
// the same 16 dBm ceiling leave it the same 14 dBm, set through three registers.
using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
const uint Channel = 868_100_000;
LoraLink dr3 = band.LinkSettings(3)!;
var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
    Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));
Console.WriteLine(
    $"rfm95w    {rfm95w.OutputDbm} dBm on PA_BOOST: RegPaConfig {rfm95w.PaConfig:x2}, " +
    $"RegPaDac {rfm95w.PaDac:x2}, RegOcp {rfm95w.Ocp:x2}");

// The carrier and the modem go into registers while the chip stands by, and TX mode
// sends the frame the FIFO holds.
Sx127xModem modem = Sx127x.Modem(dr3, Channel);
Console.WriteLine($"carrier   RegFrf {Sx127x.FrequencyWord(Channel):x6}");
Console.WriteLine(
    $"modem     RegModemConfig {modem.ModemConfig1:x2} {modem.ModemConfig2:x2} " +
    $"{modem.ModemConfig3:x2}");
Console.WriteLine($"tx mode   RegOpMode {Sx127x.LoraOpMode(Sx127xMode.Tx):x2}");

// A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers
// give its levels on the high frequency port.
var flags = (Sx127xIrq)0x50;
bool received = flags.HasFlag(Sx127xIrq.RxDone);
bool corrupt = flags.HasFlag(Sx127xIrq.PayloadCrcError);
Console.WriteLine($"irq       rx done {received}, crc error {corrupt}");
Sx127xPacketStatus packet = Sx127x.PacketStatus([0xF6, 0x30], Channel);
Console.WriteLine(
    $"received  RSSI {packet.RssiDbm} dBm, SNR {packet.SnrDb} dB, signal {packet.SignalRssiDbm} dBm");

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
bool Fits(byte dataRate) => Sx126x.Llcc68Supports(band.LinkSettings(dataRate)!);
Console.WriteLine($"llcc68    DR3 {Fits(3)}, DR2 {Fits(2)}");
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

## Reference

<!-- table: reference radios -->
- Rust: [`pamoja-radios`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios)
- TypeScript: [`@pamoja/radios`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios)
- Python: [`pamoja.radios`](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios)
- C#: [`Pamoja.Radios`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios)
- Hardware: [SX1276](https://pamoja.molex.cloud/docs/hardware.html#sx1276), [SX1262](https://pamoja.molex.cloud/docs/hardware.html#sx1262), [LLCC68](https://pamoja.molex.cloud/docs/hardware.html#llcc68), [SX1302](https://pamoja.molex.cloud/docs/hardware.html#sx1302), [SX1303](https://pamoja.molex.cloud/docs/hardware.html#sx1303)
<!-- end -->
