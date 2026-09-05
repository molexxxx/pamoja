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
println!("budget    {per_hour} readings an hour at this data rate");

// A frequency in no sub-band the plan describes has no duty cycle to budget against.
// That is a limit published elsewhere, not permission to transmit.
match plan.duty_cycle_permille(700_000_000) {
    Some(limit) => println!("700 MHz reported a {limit} per mille limit, which it has none of"),
    None => println!("700 MHz  is outside this plan, so it budgets nothing"),
}
```
<!-- end -->

## TypeScript

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

## Python

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

## C#

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

## Reference

<!-- table: reference lora -->
- Rust: [`pamoja-lora`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html)
- TypeScript: [`@pamoja/lora`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html)
- Python: [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html)
- C#: [`Pamoja.Lora`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html)
<!-- end -->
