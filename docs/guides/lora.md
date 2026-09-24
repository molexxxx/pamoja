# LoRa airtime and range

A LoRa transmission holds the channel for a length of time the radio settings
fix, and the regulations a band lives under cap how much of the time one node may
hold it. Those two numbers decide how often a long-range node gets to speak, well
before any application logic does. pamoja computes them with exact integer
arithmetic and carries the published regional channel plans, so the figures a
deployment planner works from are available on the node itself, with no radio
involved.

How far a node reaches is the other half of the plan. The power leaving its
antenna, the antenna and cable at each end, and the noise in the receiver decide
how much path loss a link survives. pamoja works that budget out to a hundredth
of a decibel from the documents that define each term: the free-space loss and
Fresnel zones of the ITU-R recommendations, the receiver sensitivity of the
Semtech datasheets, and the antenna rule of 47 CFR 15.247.

## What the example does

It works out what one reading costs a node on the European band: the time a
ten-byte frame holds the channel, the silence the duty cycle then requires, and
how many readings an hour that leaves. It ends by asking about a frequency the
plan does not cover.

Four things are typed into the calls: the region, the data-rate number, the
payload length and the frequencies to look up. SF12 and the 125 kHz bandwidth
come out of the plan's data-rate table, and coding rate 4/5, an eight-symbol
preamble, an explicit header and CRC on are the defaults those settings carry, so
the airtime rests on the published regional parameters rather than on radio
constants a caller picked. The 1% cap and the 16 dBm ceiling are read out of the
sub-band that contains 868.1 MHz, not supplied alongside it.

The second part asks how far that reading reaches. A node drives a 2.15 dBi whip
through half a decibel of pigtail, and a gateway listens through a 6 dBi
collinear antenna behind 1.5 dB of cable, with the 3 dB noise figure Semtech
TN1300.05 works with for a gateway. Those are the parts a maker picks. The
ceiling comes from the sub-band that holds 868.1 MHz, the sensitivity from the
noise floor of a 125 kHz channel and the SNR the demodulator needs at SF12, and
each loss from ITU-R P.525-5 for the distance and frequency given.

It closes with the trade every LoRa deployment makes, running the same reading
through each LoRa data rate from DR0 to DR5 and printing what it costs on air,
how many fit in an hour, and how much path loss the link still survives.

It proves:

- Data rate 0 in EU863-870 selects SF12, the slowest rate the band defines and
  the one that reaches furthest.
- A ten-byte frame at those settings takes 991,232 microseconds on air, the
  published time on air for SF12 at 125 kHz, so a plan carrying the wrong
  bandwidth fails here rather than passing a round-trip against itself.
- 868.1 MHz sits in a sub-band capped at 1% of the time and 16 dBm, both read
  from the plan by frequency.
- One percent of the time buys ninety-nine times the frame's own length in
  silence after it, which leaves thirty-six readings an hour.
- A frequency inside no sub-band the plan describes reports no duty cycle rather
  than an unlimited one, because the limit on it is published elsewhere.
- The 16 dBm ceiling of the plan leaves 14.35 dBm for a radio behind that whip
  and pigtail, which a radio set in whole decibels takes as 14 dBm, radiating
  15.65 dBm.
- The gateway hears SF12 at 125 kHz down to -140.03 dBm: the noise floor of
  -123.03 dBm, raised by the 3 dB noise figure and lowered by the 20 dB the
  demodulator reaches below the noise. The link survives 160.18 dB of path loss.
- Free space takes 97.24 dB at 2 km, 105.20 dB at 5 km, and 114.74 dB at 15 km,
  which leaves 62.94, 54.98, and 45.44 dB to spare.
- Halfway along 5 km the first Fresnel zone is 20.8 m in radius, and ITU-R
  P.526-16 starts the diffraction zone once the clearance falls below 60% of it.
- Under 47 CFR 15.247 a 9 dBi Yagi on a 64-channel hopping system may carry
  27.00 dBm, the 1 W limit less the 3 dB its gain has over 6 dBi.
- From DR0 to DR5 the same reading goes from 991.2 ms on air to 41.2 ms and from
  36 readings an hour to 873, and the link gives up 12.5 dB of path loss on the
  way, 2.5 dB at each step.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example lora" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example lora</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- lora" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- lora</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/lora.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/lora.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lora" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lora</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-lora` is `no_std`, allocation-free, and integer-only. `Region`
names the nine published bands and `plan()` gives each one's `ChannelPlan`, which
answers by data rate and by frequency: `link_settings`, `duty_cycle_permille`,
`max_eirp_dbm`, `max_payload`, and more. `LinkSettings::new` takes a spreading
factor and a bandwidth in hertz and carries the LoRa defaults, which
`with_coding_rate`, `with_preamble`, `implicit_header`, and `without_crc` change;
`airtime_us`, `min_off_time_us`, and `messages_per_hour` do the arithmetic. The
`budget` module holds the range side. `Decibels` counts hundredths of a decibel
exactly, `LinkBudget` holds the six terms of a link with public fields and a
`Default` of 0 dBm between isotropic antennas, and `Fcc15247` is the United States
antenna rule.

<!-- snippet: examples/guides/lora.rs#example -->
From [`examples/guides/lora.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/lora.rs):

```rust
use pamoja_lora::region::Region;

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
// setting that reaches furthest and holds the channel longest.
let plan = Region::Eu868.plan();
let link = plan.link_settings(0).expect("DR0 is a LoRa data rate");
println!(
    "{} DR0 is SF{} at 125 kHz",
    plan.name,
    link.spreading_factor()
);

// The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an
// explicit header and CRC on, carrying a ten-byte reading.
let airtime = link.airtime_us(10);
println!("airtime   {:.2} s for ten bytes", airtime as f64 / 1e6);

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
let channel = 868_100_000;
let permille = plan
    .duty_cycle_permille(channel)
    .expect("868.1 MHz is inside a limited sub-band");
let power = plan.max_eirp_dbm(channel);
println!("channel   {permille} per mille duty cycle, {power} dBm");

let off_time = link.min_off_time_us(10, permille);
println!(
    "silence   {:.1} s owed after each reading",
    off_time as f64 / 1e6
);

// The airtime plus that silence is what one reading really costs, which is the budget
// a deployment plans against.
let per_hour = link.messages_per_hour(10, permille);
println!("budget    {per_hour} readings an hour");

// A frequency in no sub-band the plan describes has no duty cycle to budget against.
// That is a limit published elsewhere, not permission to transmit.
let elsewhere = match plan.duty_cycle_permille(700_000_000) {
    Some(permille) => format!("limited to {permille} per mille"),
    None => "in no sub-band of this plan, so its limit is published elsewhere".to_string(),
};
println!("700 MHz   {elsewhere}");
```
<!-- end -->

The same plan, asked how far:

<!-- snippet: examples/guides/lora.rs#range -->
From [`examples/guides/lora.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/lora.rs):

```rust
use pamoja_lora::budget::{self, Decibels, Fcc15247, LinkBudget, GATEWAY_NOISE_FIGURE_DB};
use pamoja_lora::region::Region;

let eu868 = Region::Eu868.plan();
let dr0 = eu868.link_settings(0).expect("DR0 is a LoRa data rate");
let frequency = 868_100_000;

// A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
// 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
let whip = LinkBudget {
    transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
    transmit_cable_loss_db: Decibels::from_tenths(5),
    ..LinkBudget::default()
};

// The plan caps what leaves the antenna, so the antenna and cable decide how hard the
// radio may drive. A radio takes whole decibels, so the setting rounds down.
let ceiling = Decibels::from_db(eu868.max_eirp_dbm(frequency).into());
let most = whip.max_transmit_power_dbm(ceiling);
let node = LinkBudget {
    transmit_power_dbm: Decibels::from_db(most.floor_db()),
    receive_antenna_gain_dbi: Decibels::from_db(6),
    receive_cable_loss_db: Decibels::from_tenths(15),
    noise_figure_db: GATEWAY_NOISE_FIGURE_DB,
    ..whip
};
println!(
    "radio     {most} dBm allowed, set to {} dBm",
    node.transmit_power_dbm.round_db()
);
println!(
    "eirp      {} dBm under a {} dBm ceiling",
    node.eirp_dbm(),
    ceiling.round_db()
);

// The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
// loss the link survives.
let sensitivity = node.sensitivity_dbm(dr0);
let survives = node.max_path_loss_db(dr0);
println!("gateway   hears down to {sensitivity} dBm, so {survives} dB of path loss");

// Free space at three distances, and what each path leaves to spare.
let mut margins = Vec::new();
for distance_m in [2_000, 5_000, 15_000] {
    let loss = budget::free_space_loss_db(distance_m, frequency);
    let margin = node.margin_db(dr0, loss);
    println!(
        "{:>2} km     {loss} dB lost, {margin} dB to spare",
        distance_m / 1_000
    );
    margins.push(margin);
}

// Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
// diffraction loss, which starts once the clearance falls below 60% of its radius.
let radius = budget::fresnel_radius_mm(2_500, 2_500, frequency);
println!(
    "fresnel   {:.1} m at the middle of 5 km, keep {:.1} m clear",
    f64::from(radius) / 1000.0,
    f64::from(radius * 6 / 10) / 1000.0
);

// In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
// decibel an antenna has over 6 dBi.
let limit = Fcc15247::FrequencyHopping { channels: 64 }
    .max_conducted_dbm(Decibels::from_db(9))
    .expect("64 hopping channels have a limit");
println!("fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit} dBm");

// Each step down in spreading factor halves the time on air and gives up 2.5 dB of
// reach: the trade between how often a node speaks and how far it is heard.
let duty_cycle = eu868
    .duty_cycle_permille(frequency)
    .expect("868.1 MHz is inside a limited sub-band");
let mut trade = Vec::new();
for data_rate in 0..=5 {
    let link = eu868.link_settings(data_rate).expect("DR0 to DR5 are LoRa");
    let sf = link.spreading_factor();
    let ms = link.airtime_us(10) as f64 / 1000.0;
    let readings = link.messages_per_hour(10, duty_cycle);
    let reach = node.max_path_loss_db(link);
    println!(
        "DR{data_rate} SF{sf:<2}  {ms:>5.1} ms, {readings:>3} an hour, {reach} dB of path loss"
    );
    trade.push((readings, reach));
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/lora` gives `planFor(LoraRegion.Eu868)`, a
`LoraChannelPlan` with the same questions as methods: `linkSettings`,
`dutyCyclePermille`, `maxEirpDbm`, `maxPayload`. A link is a plain object,
`{ spreadingFactor, bandwidthHz, codingRateDenominator, preambleSymbols,
explicitHeader, crc }`, built by `link(sf, bandwidthHz)` or taken from a plan, and
the arithmetic is free functions over it: `airtimeUs`, `minOffTimeUs`, which gives
`null` for a zero limit, and `messagesPerHour`. A link budget is a plain object of
decibels too; `linkBudget({ ... })` fills in what is left out, and `eirpDbm`,
`sensitivityDbm`, `maxPathLossDb`, `marginDb`, and `maxTransmitPowerDbm` take it
first. Decibels are numbers resolved to the hundredth the Rust crate holds.

<!-- snippet: bindings/node/guides/lora.ts#example -->
From [`bindings/node/guides/lora.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lora.ts):

```typescript
import { LoraRegion, airtimeUs, messagesPerHour, minOffTimeUs, planFor } from '@pamoja/lora'

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
// that reaches furthest and holds the channel longest.
const plan = planFor(LoraRegion.Eu868)
const link = plan.linkSettings(0)!
console.log(`${plan.name} DR0 is SF${link.spreadingFactor} at 125 kHz`)

// The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an explicit
// header and CRC on, carrying a ten-byte reading.
const airtime = airtimeUs(link, 10)
console.log(`airtime   ${(airtime / 1e6).toFixed(2)} s for ten bytes`)

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every transmission
// buys ninety-nine times its own length in silence.
const channel = 868_100_000
const permille = plan.dutyCyclePermille(channel)!
console.log(`channel   ${permille} per mille duty cycle, ${plan.maxEirpDbm(channel)} dBm`)

const offTime = minOffTimeUs(link, 10, permille)!
console.log(`silence   ${(offTime / 1e6).toFixed(1)} s owed after each reading`)

// The airtime plus that silence is what one reading really costs, which is the budget a
// deployment plans against.
console.log(`budget    ${messagesPerHour(link, 10, permille)} readings an hour`)

// A frequency in no sub-band the plan describes has no duty cycle to budget against. That
// is a limit published elsewhere, not permission to transmit.
const outside = plan.dutyCyclePermille(700_000_000)
const elsewhere =
  outside === null
    ? 'in no sub-band of this plan, so its limit is published elsewhere'
    : `limited to ${outside} per mille`
console.log(`700 MHz   ${elsewhere}`)
```
<!-- end -->

The same plan, asked how far:

<!-- snippet: bindings/node/guides/lora.ts#range -->
From [`bindings/node/guides/lora.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lora.ts):

```typescript
import {
  GATEWAY_NOISE_FIGURE_DB,
  eirpDbm,
  fccMaxConductedDbm,
  freeSpaceLossDb,
  fresnelRadiusMm,
  linkBudget,
  marginDb,
  maxPathLossDb,
  maxTransmitPowerDbm,
  sensitivityDbm,
} from '@pamoja/lora'

const eu868 = planFor(LoraRegion.Eu868)
const dr0 = eu868.linkSettings(0)!
const frequency = 868_100_000

// A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
// 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
const whip = { transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 }
const gateway = {
  receiveAntennaGainDbi: 6,
  receiveCableLossDb: 1.5,
  noiseFigureDb: GATEWAY_NOISE_FIGURE_DB,
}

// The plan caps what leaves the antenna, so the antenna and cable decide how hard the
// radio may drive. A radio takes whole decibels, so the setting rounds down.
const ceiling = eu868.maxEirpDbm(frequency)
const most = maxTransmitPowerDbm(linkBudget(whip), ceiling)
const node = linkBudget({ ...whip, ...gateway, transmitPowerDbm: Math.floor(most) })
console.log(`radio     ${most.toFixed(2)} dBm allowed, set to ${node.transmitPowerDbm} dBm`)
console.log(`eirp      ${eirpDbm(node).toFixed(2)} dBm under a ${ceiling} dBm ceiling`)

// The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
// loss the link survives.
const sensitivity = sensitivityDbm(node, dr0)
const survives = maxPathLossDb(node, dr0)
console.log(
  `gateway   hears down to ${sensitivity.toFixed(2)} dBm, so ${survives.toFixed(2)} dB of path loss`,
)

// Free space at three distances, and what each path leaves to spare.
const margins = [2_000, 5_000, 15_000].map((distanceM) => {
  const loss = freeSpaceLossDb(distanceM, frequency)
  const margin = marginDb(node, dr0, loss)
  const km = String(distanceM / 1_000).padStart(2)
  console.log(`${km} km     ${loss.toFixed(2)} dB lost, ${margin.toFixed(2)} dB to spare`)
  return margin
})

// Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
// diffraction loss, which starts once the clearance falls below 60% of its radius.
const radius = fresnelRadiusMm(2_500, 2_500, frequency)
const clear = Math.floor((radius * 6) / 10)
console.log(
  `fresnel   ${(radius / 1000).toFixed(1)} m at the middle of 5 km, keep ${(clear / 1000).toFixed(1)} m clear`,
)

// In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
// decibel an antenna has over 6 dBi.
const limit = fccMaxConductedDbm(9, 64)!
console.log(`fcc       a 9 dBi Yagi on 64 hopping channels may carry ${limit.toFixed(2)} dBm`)

// Each step down in spreading factor halves the time on air and gives up 2.5 dB of reach:
// the trade between how often a node speaks and how far it is heard.
const dutyCycle = eu868.dutyCyclePermille(frequency)!
const trade = [0, 1, 2, 3, 4, 5].map((dataRate) => {
  const rate = eu868.linkSettings(dataRate)!
  const sf = String(rate.spreadingFactor).padEnd(2)
  const ms = (airtimeUs(rate, 10) / 1000).toFixed(1).padStart(5)
  const readings = messagesPerHour(rate, 10, dutyCycle)
  const reach = maxPathLossDb(node, rate)
  const hour = String(readings).padStart(3)
  console.log(`DR${dataRate} SF${sf}  ${ms} ms, ${hour} an hour, ${reach.toFixed(2)} dB of path loss`)
  return { readings, reach }
})
```
<!-- end -->

## Python

In Python, `pamoja.lora` gives `plan_for("EU868")`, a `ChannelPlan` with
`link_settings`, `duty_cycle_permille`, `max_eirp_dbm`, and `max_payload`, and
`REGIONS` lists the names it takes. A `LoraLink` carries the arithmetic as
methods, `airtime_us`, `min_off_time_us`, which gives `None` for a zero limit, and
`messages_per_hour`, and `link(sf, bandwidth_hz)` builds one with the LoRa
defaults. `LinkBudget` takes its terms as keyword arguments in decibels, each 0
unless given and the noise figure 6 dB, and answers `eirp_dbm()`,
`sensitivity_dbm(link)`, `max_path_loss_db(link)`, `margin_db(link, loss)`, and
`max_transmit_power_dbm(ceiling)`.

<!-- snippet: bindings/python/guides/lora.py#example -->
From [`bindings/python/guides/lora.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lora.py):

```python
from pamoja.lora import messages_per_hour, plan_for

# EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
# that reaches furthest and holds the channel longest.
plan = plan_for("EU868")
link = plan.link_settings(0)
print(f"{plan.name} DR0 is SF{link.spreading_factor} at 125 kHz")

# The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an explicit
# header and CRC on, carrying a ten-byte reading.
airtime = link.airtime_us(10)
print(f"airtime   {airtime / 1e6:.2f} s for ten bytes")

# 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every transmission
# buys ninety-nine times its own length in silence.
channel = 868_100_000
permille = plan.duty_cycle_permille(channel)
print(f"channel   {permille} per mille duty cycle, {plan.max_eirp_dbm(channel)} dBm")

off_time = link.min_off_time_us(10, permille)
print(f"silence   {off_time / 1e6:.1f} s owed after each reading")

# The airtime plus that silence is what one reading really costs, which is the budget a
# deployment plans against.
print(f"budget    {messages_per_hour(link, 10, permille)} readings an hour")

# A frequency in no sub-band the plan describes has no duty cycle to budget against. That
# is a limit published elsewhere, not permission to transmit.
outside = plan.duty_cycle_permille(700_000_000)
elsewhere = (
    "in no sub-band of this plan, so its limit is published elsewhere"
    if outside is None
    else f"limited to {outside} per mille"
)
print(f"700 MHz   {elsewhere}")
```
<!-- end -->

The same plan, asked how far:

<!-- snippet: bindings/python/guides/lora.py#range -->
From [`bindings/python/guides/lora.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lora.py):

```python
import math

from pamoja.lora import (
    GATEWAY_NOISE_FIGURE_DB,
    LinkBudget,
    fcc_max_conducted_dbm,
    free_space_loss_db,
    fresnel_radius_mm,
    plan_for,
)

eu868 = plan_for("EU868")
dr0 = eu868.link_settings(0)
frequency = 868_100_000

# A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
# 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
whip = {"transmit_antenna_gain_dbi": 2.15, "transmit_cable_loss_db": 0.5}
gateway = {
    "receive_antenna_gain_dbi": 6,
    "receive_cable_loss_db": 1.5,
    "noise_figure_db": GATEWAY_NOISE_FIGURE_DB,
}

# The plan caps what leaves the antenna, so the antenna and cable decide how hard the
# radio may drive. A radio takes whole decibels, so the setting rounds down.
ceiling = eu868.max_eirp_dbm(frequency)
most = LinkBudget(**whip).max_transmit_power_dbm(ceiling)
node = LinkBudget(transmit_power_dbm=math.floor(most), **whip, **gateway)
print(f"radio     {most:.2f} dBm allowed, set to {node.transmit_power_dbm:.0f} dBm")
print(f"eirp      {node.eirp_dbm():.2f} dBm under a {ceiling} dBm ceiling")

# The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
# loss the link survives.
sensitivity = node.sensitivity_dbm(dr0)
survives = node.max_path_loss_db(dr0)
print(f"gateway   hears down to {sensitivity:.2f} dBm, so {survives:.2f} dB of path loss")

# Free space at three distances, and what each path leaves to spare.
margins = []
for distance_m in (2_000, 5_000, 15_000):
    loss = free_space_loss_db(distance_m, frequency)
    margin = node.margin_db(dr0, loss)
    print(f"{distance_m // 1_000:>2} km     {loss:.2f} dB lost, {margin:.2f} dB to spare")
    margins.append(margin)

# Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
# diffraction loss, which starts once the clearance falls below 60% of its radius.
radius = fresnel_radius_mm(2_500, 2_500, frequency)
clear = radius * 6 // 10
print(f"fresnel   {radius / 1000:.1f} m at the middle of 5 km, keep {clear / 1000:.1f} m clear")

# In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
# decibel an antenna has over 6 dBi.
limit = fcc_max_conducted_dbm(9, hopping_channels=64)
print(f"fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit:.2f} dBm")

# Each step down in spreading factor halves the time on air and gives up 2.5 dB of reach:
# the trade between how often a node speaks and how far it is heard.
duty_cycle = eu868.duty_cycle_permille(frequency)
trade = []
for data_rate in range(6):
    rate = eu868.link_settings(data_rate)
    ms = rate.airtime_us(10) / 1000
    readings = rate.messages_per_hour(10, duty_cycle)
    reach = node.max_path_loss_db(rate)
    print(
        f"DR{data_rate} SF{rate.spreading_factor:<2}  {ms:>5.1f} ms, {readings:>3} an hour, "
        f"{reach:.2f} dB of path loss"
    )
    trade.append((readings, reach))
```
<!-- end -->

## C#

In C#, `Pamoja.Lora` holds `LoraChannelPlan`, disposable and opened with
`LoraChannelPlan.ForRegion(LoraRegion.Eu868)`, with `LinkSettings`,
`DutyCyclePermille`, `MaxEirpDbm`, and `MaxPayload`. `LoraLink` is built with
`new LoraLink(sf, bandwidthHz)` and changed with `WithCodingRate`, `WithPreamble`,
`WithImplicitHeader`, and `WithoutCrc`, and it answers `AirtimeMicros`,
`MinOffTimeMicros`, which gives `null` for a zero limit, and `MessagesPerHour`; a
negative payload length throws `ArgumentOutOfRangeException`. `LoraLinkBudget` is
a record with init-only decibel properties, `EirpDbm`, and the budget methods,
and its statics hold the path, the Fresnel zone, and the FCC rule.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs):

```csharp
// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
// setting that reaches furthest and holds the channel longest.
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
LoraLink link = plan.LinkSettings(0)!;
Console.WriteLine($"{plan.Name} DR0 is SF{link.SpreadingFactor} at 125 kHz");

// The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an
// explicit header and CRC on, carrying a ten-byte reading.
ulong airtime = link.AirtimeMicros(10);
Console.WriteLine(Invariant($"airtime   {airtime / 1e6:F2} s for ten bytes"));

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
const uint Channel = 868_100_000;
uint permille = plan.DutyCyclePermille(Channel)!.Value;
Console.WriteLine(
    $"channel   {permille} per mille duty cycle, {plan.MaxEirpDbm(Channel)} dBm");

ulong offTime = link.MinOffTimeMicros(10, permille)!.Value;
Console.WriteLine(Invariant($"silence   {offTime / 1e6:F1} s owed after each reading"));

// The airtime plus that silence is what one reading really costs, which is the
// budget a deployment plans against.
Console.WriteLine($"budget    {link.MessagesPerHour(10, permille)} readings an hour");

// A frequency in no sub-band the plan describes has no duty cycle to budget
// against. That is a limit published elsewhere, not permission to transmit.
uint? outside = plan.DutyCyclePermille(700_000_000);
string elsewhere = outside is null
    ? "in no sub-band of this plan, so its limit is published elsewhere"
    : $"limited to {outside} per mille";
Console.WriteLine($"700 MHz   {elsewhere}");
```
<!-- end -->

The same plan, asked how far:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs#range -->
From [`bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs):

```csharp
using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
LoraLink dr0 = eu868.LinkSettings(0)!;
const uint Frequency = 868_100_000;

// A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with
// a 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };

// The plan caps what leaves the antenna, so the antenna and cable decide how hard
// the radio may drive. A radio takes whole decibels, so the setting rounds down.
sbyte ceiling = eu868.MaxEirpDbm(Frequency);
double most = whip.MaxTransmitPowerDbm(ceiling);
LoraLinkBudget node = whip with
{
    TransmitPowerDbm = Math.Floor(most),
    ReceiveAntennaGainDbi = 6,
    ReceiveCableLossDb = 1.5,
    NoiseFigureDb = LoraLinkBudget.GatewayNoiseFigureDb,
};
Console.WriteLine(Invariant($"radio     {most:F2} dBm allowed, set to {node.TransmitPowerDbm} dBm"));
Console.WriteLine(Invariant($"eirp      {node.EirpDbm:F2} dBm under a {ceiling} dBm ceiling"));

// The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most
// path loss the link survives.
double sensitivity = node.SensitivityDbm(dr0);
double survives = node.MaxPathLossDb(dr0);
Console.WriteLine(Invariant(
    $"gateway   hears down to {sensitivity:F2} dBm, so {survives:F2} dB of path loss"));

// Free space at three distances, and what each path leaves to spare.
var margins = new List<double>();
foreach (uint distanceMeters in new uint[] { 2_000, 5_000, 15_000 })
{
    double loss = LoraLinkBudget.FreeSpaceLossDb(distanceMeters, Frequency);
    double margin = node.MarginDb(dr0, loss);
    Console.WriteLine(Invariant(
        $"{distanceMeters / 1_000,2} km     {loss:F2} dB lost, {margin:F2} dB to spare"));
    margins.Add(margin);
}

// Free space assumes nothing is in the way. Terrain inside the first Fresnel zone
// adds diffraction loss, which starts once the clearance falls below 60% of its
// radius.
uint radius = LoraLinkBudget.FresnelRadiusMillimeters(2_500, 2_500, Frequency);
uint clear = radius * 6 / 10;
Console.WriteLine(Invariant(
    $"fresnel   {radius / 1000.0:F1} m at the middle of 5 km, keep {clear / 1000.0:F1} m clear"));

// In the United States, 47 CFR 15.247 caps conducted power instead, and takes off
// every decibel an antenna has over 6 dBi.
double limit = LoraLinkBudget.FccMaxConductedDbm(9, hoppingChannels: 64)!.Value;
Console.WriteLine(Invariant($"fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit:F2} dBm"));

// Each step down in spreading factor halves the time on air and gives up 2.5 dB of
// reach: the trade between how often a node speaks and how far it is heard.
uint dutyCycle = eu868.DutyCyclePermille(Frequency)!.Value;
var trade = new List<(ulong Readings, double Reach)>();
for (byte dataRate = 0; dataRate <= 5; dataRate++)
{
    LoraLink rate = eu868.LinkSettings(dataRate)!;
    double ms = rate.AirtimeMicros(10) / 1000.0;
    ulong readings = rate.MessagesPerHour(10, dutyCycle);
    double reach = node.MaxPathLossDb(rate);
    Console.WriteLine(Invariant(
        $"DR{dataRate} SF{rate.SpreadingFactor,-2}  {ms,5:F1} ms, {readings,3} an hour, {reach:F2} dB of path loss"));
    trade.Add((readings, reach));
}
```
<!-- end -->

## Values at a glance

**The regions,** each with the ceiling it assumes where no sub-band says otherwise
and the sub-bands it limits. The ceiling is radiated power except in US902-928,
which counts conducted power:

| Region | Plan | Ceiling | Limited sub-bands |
| --- | --- | --- | --- |
| EU868 | EU863-870 | 16 dBm | 868.0 to 868.6 MHz at 1% and 16 dBm; 869.4 to 869.65 MHz at 10% and 27 dBm |
| US915 | US902-928 | 30 dBm | none |
| EU433 | EU433 | 12 dBm | 433.05 to 434.79 MHz at 10% and 12 dBm |
| AU915 | AU915-928 | 30 dBm | none |
| CN470 | CN470-510 | 19 dBm | none |
| AS923 | AS923 | 16 dBm | 923.0 to 923.5 MHz at 1% and 16 dBm |
| KR920 | KR920-923 | 14 dBm | 920.9 to 921.9 MHz at 10 dBm and 922.1 to 923.3 MHz at 14 dBm, no duty limit |
| IN865 | IN865 | 29 dBm | none |
| RU864 | RU864-870 | 16 dBm | 868.7 to 869.2 MHz at 1% and 16 dBm |

**EU863-870's data rates,** with a ten-byte reading at 1% of the time:

| Data rate | Modulation | On air | An hour | Most application payload |
| --- | --- | --- | --- | --- |
| DR0 | SF12, 125 kHz | 991.2 ms | 36 | 51 bytes |
| DR1 | SF11, 125 kHz | 577.5 ms | 62 | 51 bytes |
| DR2 | SF10, 125 kHz | 288.8 ms | 124 | 51 bytes |
| DR3 | SF9, 125 kHz | 144.4 ms | 249 | 115 bytes |
| DR4 | SF8, 125 kHz | 72.2 ms | 498 | 242 bytes |
| DR5 | SF7, 125 kHz | 41.2 ms | 873 | 242 bytes |
| DR6 | SF7, 250 kHz | 20.6 ms | 1746 | 242 bytes |
| DR7 | FSK at 50 kbit/s | no LoRa settings | | 242 bytes |
| DR8 to DR11 | LR-FHSS | no LoRa settings | | 50 or 115 bytes |
| DR12 | SF6, 125 kHz | 23.2 ms | 1553 | 242 bytes |
| DR13 | SF5, 125 kHz | 12.9 ms | 2798 | 242 bytes |

**A link's settings:**

| Setting | Default | Range |
| --- | --- | --- |
| spreading factor | from the data rate | 5 to 12, clamped |
| bandwidth | from the data rate | hertz; 0 counts as one hertz |
| coding rate | 4/5 | 4/5 to 4/8, clamped |
| preamble | 8 symbols | any length |
| header | explicit | explicit or implicit |
| CRC | on | on or off |
| low data rate optimization | on above 16 ms a symbol | worked out from the rest, not set |

**A link budget's terms,** each in decibels:

| Term | Default | What it is |
| --- | --- | --- |
| transmit power | 0 dBm | what the radio delivers at its antenna port |
| transmit antenna gain | 0 dBi | the node's antenna over an isotropic one |
| transmit cable loss | 0 dB | the pigtail and connectors at the node |
| receive antenna gain | 0 dBi | the gateway's antenna |
| receive cable loss | 0 dB | the cable and connectors at the gateway |
| noise figure | 6 dB | the receiver's own noise; 3 dB for a gateway |

Sensitivity is the noise floor, -174 dBm/Hz plus `10 log10` of the bandwidth,
raised by the noise figure and lowered by the SNR the demodulator reaches below
the noise: -2.5 dB at SF5, 2.5 dB lower at each step, and -20 dB at SF12.

**47 CFR 15.247 at 902 to 928 MHz,** each limit less every decibel the antenna
has over 6 dBi:

| System | Conducted limit |
| --- | --- |
| digital modulation | 30 dBm, 1 W |
| hopping on 50 channels or more | 30 dBm, 1 W |
| hopping on 25 to 49 channels | 23.98 dBm, 0.25 W |
| hopping on fewer than 25 | none named, so no answer |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| open a region's plan | `Region::Eu868.plan()` |
| take a data rate's settings | `plan.link_settings(dr)`, `plan.max_payload(dr, behind_repeater)` |
| build settings | `LinkSettings::new(sf, bandwidth_hz)`, `with_coding_rate`, `with_preamble`, `implicit_header`, `without_crc` |
| time a frame | `airtime_us(len)`, `symbol_time_us()`, `low_data_rate_optimization()` |
| read a frequency's limits | `plan.duty_cycle_permille(hz)`, `plan.max_eirp_dbm(hz)` |
| budget the time | `min_off_time_us(len, permille)`, `messages_per_hour(len, permille)` |
| budget the link | `LinkBudget { .. }`, `eirp_dbm()`, `sensitivity_dbm(link)`, `max_path_loss_db(link)`, `margin_db(link, loss)`, `max_transmit_power_dbm(ceiling)` |
| size the path | `budget::free_space_loss_db(m, hz)`, `budget::fresnel_radius_mm(near, far, hz)` |
| apply the FCC rule | `Fcc15247::FrequencyHopping { channels }.max_conducted_dbm(gain)` |

### TypeScript

| To | Call |
| --- | --- |
| open a region's plan | `planFor(LoraRegion.Eu868)` |
| take a data rate's settings | `plan.linkSettings(dr)`, `plan.maxPayload(dr)` |
| build settings | `link(sf, bandwidthHz)`, then change the object's fields |
| time a frame | `airtimeUs(link, len)`, `symbolTimeUs(link)`, `lowDataRateOptimization(link)` |
| read a frequency's limits | `plan.dutyCyclePermille(hz)`, `plan.maxEirpDbm(hz)` |
| budget the time | `minOffTimeUs(link, len, permille)`, `messagesPerHour(link, len, permille)`, `messagesPerHourAt(plan, dr, len, hz)` |
| budget the link | `linkBudget({ ... })`, `eirpDbm(b)`, `sensitivityDbm(b, link)`, `maxPathLossDb(b, link)`, `marginDb(b, link, loss)`, `maxTransmitPowerDbm(b, ceiling)` |
| size the path | `freeSpaceLossDb(m, hz)`, `fresnelRadiusMm(near, far, hz)` |
| apply the FCC rule | `fccMaxConductedDbm(gain, hoppingChannels?)` |

### Python

| To | Call |
| --- | --- |
| open a region's plan | `plan_for("EU868")` |
| take a data rate's settings | `plan.link_settings(dr)`, `plan.max_payload(dr)` |
| build settings | `link(sf, bandwidth_hz)`, or `LoraLink(sf, bandwidth_hz, coding_rate_denominator=5, preamble_symbols=8, explicit_header=True, crc=True)` |
| time a frame | `link.airtime_us(len)`, `link.symbol_time_us()`, `link.low_data_rate_optimization()` |
| read a frequency's limits | `plan.duty_cycle_permille(hz)`, `plan.max_eirp_dbm(hz)` |
| budget the time | `link.min_off_time_us(len, permille)`, `link.messages_per_hour(len, permille)`, `messages_per_hour_at(plan, dr, len, hz)` |
| budget the link | `LinkBudget(transmit_power_dbm=...)`, `eirp_dbm()`, `sensitivity_dbm(link)`, `max_path_loss_db(link)`, `margin_db(link, loss)`, `max_transmit_power_dbm(ceiling)` |
| size the path | `free_space_loss_db(m, hz)`, `fresnel_radius_mm(near, far, hz)` |
| apply the FCC rule | `fcc_max_conducted_dbm(gain, hopping_channels=None)` |

### C#

| To | Call |
| --- | --- |
| open a region's plan | `LoraChannelPlan.ForRegion(LoraRegion.Eu868)` |
| take a data rate's settings | `plan.LinkSettings(dr)`, `plan.MaxPayload(dr)` |
| build settings | `new LoraLink(sf, bandwidthHz)`, `WithCodingRate`, `WithPreamble`, `WithImplicitHeader`, `WithoutCrc` |
| time a frame | `AirtimeMicros(len)`, `SymbolTimeMicros`, `LowDataRateOptimization` |
| read a frequency's limits | `plan.DutyCyclePermille(hz)`, `plan.MaxEirpDbm(hz)` |
| budget the time | `MinOffTimeMicros(len, permille)`, `MessagesPerHour(len, permille)` |
| budget the link | `new LoraLinkBudget { ... }`, `EirpDbm`, `SensitivityDbm(link)`, `MaxPathLossDb(link)`, `MarginDb(link, loss)`, `MaxTransmitPowerDbm(ceiling)` |
| size the path | `LoraLinkBudget.FreeSpaceLossDb(m, hz)`, `LoraLinkBudget.FresnelRadiusMillimeters(near, far, hz)` |
| apply the FCC rule | `LoraLinkBudget.FccMaxConductedDbm(gain, hoppingChannels)` |

<!-- languages end -->

## When it goes wrong

The arithmetic refuses nothing. A plan answers every question about its band,
and a question with no answer gets none rather than a guess. The mistakes that
cost an afternoon:

- **A data rate gives no link settings.** Not every number is LoRa. In
  EU863-870, DR7 is FSK, DR8 to DR11 are LR-FHSS, and DR14 and DR15 are
  reserved, so the plan has no LoRa settings to give for them.
- **Every airtime is a thousand times too long.** The bandwidth is in hertz, so
  `125` is 125 Hz, not 125 kHz. Write `125_000`. A bandwidth of 0 counts as one
  hertz rather than dividing by zero.
- **A frequency reports no duty cycle.** It lies in no sub-band the plan
  describes. That is not permission to transmit: the limit on it is published
  elsewhere, and the operator is the one who knows which.
- **A limit of zero leaves nothing to send.** It forbids transmitting, so the off
  time is unbounded, `u64::MAX` in Rust and C and `null` or `None` in the
  bindings, and the hour holds no readings.
- **The frame is refused though the budget allowed it.** A data rate carries a
  limited payload, 51 bytes of application data at DR0 in EU863-870. Check the
  plan's most payload before sending; the airtime keeps counting past what one
  frame carries.
- **The radiated power is over the ceiling.** The ceiling limits what leaves the
  antenna, so an antenna with gain needs less from the radio. Take the most the
  radio may drive from `max_transmit_power_dbm`, and round it down, never to the
  nearest decibel. In US902-928 the ceiling is conducted power instead.
- **A radio set from these settings hears nothing at SF11 or SF12.** Low data
  rate optimization is on above 16 ms a symbol, and both ends must agree on it.
  The link says whether it applies, `low_data_rate_optimization` in Rust and Python,
  `lowDataRateOptimization` in TypeScript and `LowDataRateOptimization` in C#; set
  the radio the same way.
- **A path the budget says reaches fails in the field.** Free space is the best
  case. Terrain inside the Fresnel zone, trees, walls and fading all take more,
  so keep part of the margin rather than spending all of it on distance.

## Where next

<!-- table: next lora -->
- [LoRa radios](radios.md): The Semtech SX126x and SX127x LoRa radios and the SX1302 and SX1303 gateway concentrators.
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
- [Power](power.md): Duty cycling and an energy-aware governor that stretches work as the battery drains and holds its mode against a wandering charge.
- Beside it: [Radios and antennas](../radio.md).
- Also in Radio and reach: [LoRaWAN gateways](gateway.md), [Mesh frames](mesh.md), [Routing](routing.md).
<!-- end -->

## Reference

<!-- table: reference lora -->
- Rust: [`pamoja-lora`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lora)
- TypeScript: [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lora)
- Python: [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lora)
- C#: [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lora)
- Hardware: [SX1276](https://pamoja.molex.cloud/docs/hardware.html#sx1276), [SX1262](https://pamoja.molex.cloud/docs/hardware.html#sx1262), [LLCC68](https://pamoja.molex.cloud/docs/hardware.html#llcc68), [SX1302](https://pamoja.molex.cloud/docs/hardware.html#sx1302), [SX1303](https://pamoja.molex.cloud/docs/hardware.html#sx1303), [SX1250](https://pamoja.molex.cloud/docs/hardware.html#sx1250), [RAK2287 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak2287), [RAK5146 WisLink concentrator](https://pamoja.molex.cloud/docs/hardware.html#rak5146), [WM1302 LoRaWAN gateway module](https://pamoja.molex.cloud/docs/hardware.html#wm1302), [5.8 dBi fiberglass antenna, 863 to 870 MHz](https://pamoja.molex.cloud/docs/hardware.html#rakarg13), [5.8 dBi fiberglass antenna, 902 to 928 MHz](https://pamoja.molex.cloud/docs/hardware.html#rakarg14), [MHF to SMA or RP-SMA pigtail](https://pamoja.molex.cloud/docs/hardware.html#rak-mhf-sma-pigtail), [SMA to u.FL adapter cable](https://pamoja.molex.cloud/docs/hardware.html#adafruit-sma-ufl-cable), [N-type lightning arrestor](https://pamoja.molex.cloud/docs/hardware.html#rak-lightning-arrestor), [N-type lightning surge protector, DC to 6 GHz](https://pamoja.molex.cloud/docs/hardware.html#l-com-lightning-protector)
<!-- end -->
