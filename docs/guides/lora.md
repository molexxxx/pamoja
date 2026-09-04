# LoRa airtime

A LoRa transmission holds the channel for a length of time the radio settings
fix, and the regulations a band lives under cap how much of the time one node may
hold it. Those two numbers decide how often a long-range node gets to speak, well
before any application logic does. pamoja computes them with exact integer
arithmetic and carries the published regional channel plans, so the figures a
deployment planner works from are available on the node itself, with no radio
involved.

## What the example does

It reads the European channel plan, takes the radio settings its slowest data
rate selects, and checks the time on air of a ten-byte frame against the value
the LoRa formula fixes for those settings. It then looks up what the 868.1 MHz
sub-band allows and turns that limit into the silence each frame costs and the
number of readings an hour it leaves.

It proves:

- DR0 in EU863-870 is SF12 at 125 kHz, the slowest rate the band defines and the
  one that reaches furthest.
- A ten-byte frame at that rate takes 991,232 microseconds on air, the published
  figure for these settings, so an implementation that is wrong but
  self-consistent still fails.
- The 868.1 MHz sub-band is capped at 1% of the time and 16 dBm, so every frame
  buys ninety-nine times its own length in silence.
- What is left is thirty-six readings an hour at the longest-reaching rate.
- A frequency inside no sub-band the plan describes reports no duty cycle rather
  than an unlimited one, because the limit on it is published elsewhere.

## Rust

<!-- snippet: examples/tests/guides/lora.rs#example -->
From [`examples/tests/guides/lora.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/lora.rs):

```rust
use pamoja_lora::region::Region;

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
// setting that reaches furthest and holds the channel longest.
let plan = Region::Eu868.plan();
assert_eq!(plan.name, "EU863-870");
let link = plan.link_settings(0).expect("DR0 is a LoRa data rate");
assert_eq!(link.spreading_factor(), 12);

// The published time on air for SF12 at 125 kHz, coding rate 4/5, an eight-symbol
// preamble, an explicit header and CRC on, carrying ten bytes.
let airtime = link.airtime_us(10);
assert_eq!(airtime, 991_232);

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
let permille = plan
    .duty_cycle_permille(868_100_000)
    .expect("868.1 MHz is inside a limited sub-band");
assert_eq!(permille, 10);
assert_eq!(plan.max_eirp_dbm(868_100_000), 16);
let off_time = link.min_off_time_us(10, permille);
assert_eq!(off_time, airtime * 99);

// The airtime plus that silence is what one reading really costs, which is the
// budget a deployment plans against: at SF12, thirty-six readings an hour.
assert_eq!(3_600_000_000 / (airtime + off_time), 36);

// A frequency in no sub-band the plan describes has no duty cycle to budget
// against. That is a limit published elsewhere, not permission to transmit.
assert_eq!(plan.duty_cycle_permille(700_000_000), None);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/lora.ts#example -->
From [`bindings/node/guides/lora.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/lora.ts):

```typescript
import assert from 'node:assert/strict'

import { LoraRegion, airtimeUs, messagesPerHour, minOffTimeUs, planFor } from '@pamoja/lora'

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
// that reaches furthest and holds the channel longest.
const plan = planFor(LoraRegion.Eu868)
assert.equal(plan.name, 'EU863-870')
const link = plan.linkSettings(0)!
assert.equal(link.spreadingFactor, 12)

// The published time on air for SF12 at 125 kHz, coding rate 4/5, an eight-symbol
// preamble, an explicit header and CRC on, carrying ten bytes.
const airtime = airtimeUs(link, 10)
assert.equal(airtime, 991_232)

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
const permille = plan.dutyCyclePermille(868_100_000)!
assert.equal(permille, 10)
assert.equal(plan.maxEirpDbm(868_100_000), 16)
assert.equal(minOffTimeUs(link, 10, permille), airtime * 99)

// The airtime plus that silence is what one reading really costs, which is the budget a
// deployment plans against: at SF12, thirty-six readings an hour.
assert.equal(messagesPerHour(link, 10, permille), 36)

// A frequency in no sub-band the plan describes has no duty cycle to budget against. That
// is a limit published elsewhere, not permission to transmit.
assert.equal(plan.dutyCyclePermille(700_000_000), null)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/lora.py#example -->
From [`bindings/python/guides/lora.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/lora.py):

```python
from pamoja.lora import messages_per_hour, plan_for

# EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
# setting that reaches furthest and holds the channel longest.
plan = plan_for("EU868")
assert plan.name == "EU863-870"
link = plan.link_settings(0)
assert link.spreading_factor == 12

# The published time on air for SF12 at 125 kHz, coding rate 4/5, an eight-symbol
# preamble, an explicit header and CRC on, carrying ten bytes.
airtime = link.airtime_us(10)
assert airtime == 991_232

# 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
# transmission buys ninety-nine times its own length in silence.
permille = plan.duty_cycle_permille(868_100_000)
assert permille == 10
assert plan.max_eirp_dbm(868_100_000) == 16
assert link.min_off_time_us(10, permille) == airtime * 99

# The airtime plus that silence is what one reading really costs, which is the
# budget a deployment plans against: at SF12, thirty-six readings an hour.
assert messages_per_hour(link, 10, permille) == 36

# A frequency in no sub-band the plan describes has no duty cycle to budget
# against. That is a limit published elsewhere, not permission to transmit.
assert plan.duty_cycle_permille(700_000_000) is None
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LoraGuide.cs):

```csharp
// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz,
// the setting that reaches furthest and holds the channel longest.
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
Expect(plan.Name == "EU863-870", "the plan names its band");
LoraLink link = plan.LinkSettings(0)!;
Expect(link.SpreadingFactor == 12, "DR0 is the slowest rate the band defines");

// The published time on air for SF12 at 125 kHz, coding rate 4/5, an
// eight-symbol preamble, an explicit header and CRC on, carrying ten bytes.
ulong airtime = link.AirtimeMicros(10);
Expect(airtime == 991_232, "the published time on air of a ten-byte frame");

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
uint permille = plan.DutyCyclePermille(868_100_000)!.Value;
Expect(permille == 10, "the 868.1 MHz sub-band is limited to 1%");
Expect(plan.MaxEirpDbm(868_100_000) == 16, "and to 16 dBm");
Expect(
    link.MinOffTimeMicros(10, permille) == airtime * 99,
    "the silence a 1% duty cycle forces after one frame");

// The airtime plus that silence is what one reading really costs, which is the
// budget a deployment plans against: at SF12, thirty-six readings an hour.
Expect(link.MessagesPerHour(10, permille) == 36, "the message budget at DR0");

// A frequency in no sub-band the plan describes has no duty cycle to budget
// against. That is a limit published elsewhere, not permission to transmit.
Expect(plan.DutyCyclePermille(700_000_000) is null, "700 MHz is outside the band");
```
<!-- end -->

## Reference

<!-- table: reference lora -->
- Rust: [`pamoja-lora`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html)
- TypeScript: [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html)
- Python: [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html)
- C#: [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html)
<!-- end -->
