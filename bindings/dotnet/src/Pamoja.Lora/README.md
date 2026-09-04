# Pamoja.Lora

Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Lora
```

```csharp
using Pamoja.Lora;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lora`](https://crates.io/crates/pamoja-lora) | [docs.rs](https://docs.rs/pamoja-lora), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html) |
| TypeScript | [`@pamoja/lora`](https://www.npmjs.com/package/@pamoja/lora) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html) |
| Python | [`pamoja-lora`](https://pypi.org/project/pamoja-lora/) | [`pamoja.lora`](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html) |
| C# | [`Pamoja.Lora`](https://www.nuget.org/packages/Pamoja.Lora) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.LoraLink.html) |

## Documentation

- [The LoRa airtime guide](https://pamoja.molex.cloud/docs/guides/lora.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
