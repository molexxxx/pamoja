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
payload length and the frequencies to look up. The Rust version types in one
more, the microseconds in an hour, because it divides the budget out by hand
where the bindings call their messages-per-hour helper. SF12 and the 125 kHz
bandwidth come out of the plan's data-rate table, and coding rate 4/5, an
eight-symbol preamble, an explicit header and CRC on are the defaults those
settings carry, so the airtime rests on the published regional parameters
rather than on radio constants a caller picked. The 1% cap and the 16 dBm
ceiling are read out of the sub-band that contains 868.1 MHz, not supplied
alongside it.

The second part asks how far that reading reaches. A node drives a 2.15 dBi whip
through half a decibel of pigtail, and a gateway listens through a 6 dBi
collinear antenna behind 1.5 dB of cable, with the 3 dB noise figure Semtech
TN1300.05 works with for a gateway. Those are the parts a maker picks. The
ceiling comes from the sub-band that holds 868.1 MHz, the sensitivity from the
noise floor of a 125 kHz channel and the SNR the demodulator needs at SF12, and
each loss from ITU-R P.525-5 for the distance and frequency given.

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

## Run it

The example below is a test that runs in CI, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo test -p pamoja-examples --test guides lora -- --nocapture" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo test -p pamoja-examples --test guides lora -- --nocapture</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run test:guides -- lora" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run test:guides -- lora</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/lora.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/lora.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lora" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- lora</code></div>
</div>
<!-- end -->

## Rust

What one reading costs:

<!-- snippet: examples/tests/guides/lora.rs#example -->
From [`examples/tests/guides/lora.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lora.rs):

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
let per_hour = 3_600_000_000 / (airtime + off_time);
println!("budget    {per_hour} readings an hour");

// A frequency in no sub-band the plan describes has no duty cycle to budget against.
// That is a limit published elsewhere, not permission to transmit.
match plan.duty_cycle_permille(700_000_000) {
    Some(limit) => println!("700 MHz reported a {limit} per mille limit, which it has none of"),
    None => println!("700 MHz  is outside this plan, so it budgets nothing: true"),
}
```
<!-- end -->

How far it reaches:

<!-- snippet: examples/tests/guides/lora.rs#range -->
From [`examples/tests/guides/lora.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lora.rs):

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
```
<!-- end -->

## TypeScript

What one reading costs:

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
console.log(`700 MHz  is outside this plan, so it budgets nothing: ${outside === null}`)
```
<!-- end -->

How far it reaches:

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
```
<!-- end -->

## Python

What one reading costs:

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
print(f"700 MHz  is outside this plan, so it budgets nothing: {outside is None}")
```
<!-- end -->

How far it reaches:

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
```
<!-- end -->

## C#

What one reading costs:

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
Console.WriteLine($"airtime   {airtime / 1e6:F2} s for ten bytes");

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
const uint Channel = 868_100_000;
uint permille = plan.DutyCyclePermille(Channel)!.Value;
Console.WriteLine(
    $"channel   {permille} per mille duty cycle, {plan.MaxEirpDbm(Channel)} dBm");

ulong offTime = link.MinOffTimeMicros(10, permille)!.Value;
Console.WriteLine($"silence   {offTime / 1e6:F1} s owed after each reading");

// The airtime plus that silence is what one reading really costs, which is the
// budget a deployment plans against.
Console.WriteLine($"budget    {link.MessagesPerHour(10, permille)} readings an hour");

// A frequency in no sub-band the plan describes has no duty cycle to budget
// against. That is a limit published elsewhere, not permission to transmit.
uint? outside = plan.DutyCyclePermille(700_000_000);
Console.WriteLine($"700 MHz  is outside this plan, so it budgets nothing: {outside is null}");
```
<!-- end -->

How far it reaches:

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
Console.WriteLine($"radio     {most:F2} dBm allowed, set to {node.TransmitPowerDbm} dBm");
Console.WriteLine($"eirp      {node.EirpDbm:F2} dBm under a {ceiling} dBm ceiling");

// The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most
// path loss the link survives.
double sensitivity = node.SensitivityDbm(dr0);
double survives = node.MaxPathLossDb(dr0);
Console.WriteLine(
    $"gateway   hears down to {sensitivity:F2} dBm, so {survives:F2} dB of path loss");

// Free space at three distances, and what each path leaves to spare.
var margins = new List<double>();
foreach (uint distanceMeters in new uint[] { 2_000, 5_000, 15_000 })
{
    double loss = LoraLinkBudget.FreeSpaceLossDb(distanceMeters, Frequency);
    double margin = node.MarginDb(dr0, loss);
    Console.WriteLine(
        $"{distanceMeters / 1_000,2} km     {loss:F2} dB lost, {margin:F2} dB to spare");
    margins.Add(margin);
}

// Free space assumes nothing is in the way. Terrain inside the first Fresnel zone
// adds diffraction loss, which starts once the clearance falls below 60% of its
// radius.
uint radius = LoraLinkBudget.FresnelRadiusMillimeters(2_500, 2_500, Frequency);
uint clear = radius * 6 / 10;
Console.WriteLine(
    $"fresnel   {radius / 1000.0:F1} m at the middle of 5 km, keep {clear / 1000.0:F1} m clear");

// In the United States, 47 CFR 15.247 caps conducted power instead, and takes off
// every decibel an antenna has over 6 dBi.
double limit = LoraLinkBudget.FccMaxConductedDbm(9, hoppingChannels: 64)!.Value;
Console.WriteLine($"fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit:F2} dBm");
```
<!-- end -->

## Reference

<!-- table: reference lora -->
- Rust: [`pamoja-lora`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lora)
- TypeScript: [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lora)
- Python: [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lora)
- C#: [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lora)
- Hardware: [SX1276](https://pamoja.molex.cloud/docs/hardware.html#sx1276), [SX1262](https://pamoja.molex.cloud/docs/hardware.html#sx1262), [LoRaWAN Regional Parameters](https://pamoja.molex.cloud/docs/hardware.html#lorawan-rp002)
<!-- end -->
