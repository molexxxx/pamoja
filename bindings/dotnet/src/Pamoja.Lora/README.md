# Pamoja.Lora

Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/lora.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lora`](https://crates.io/crates/pamoja-lora) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html), [docs.rs](https://docs.rs/pamoja-lora) |
| TypeScript | [`@pamoja/lora`](https://www.npmjs.com/package/@pamoja/lora) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html) |
| Python | [`pamoja-lora`](https://pypi.org/project/pamoja-lora/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html) |
| C# | [`Pamoja.Lora`](https://www.nuget.org/packages/Pamoja.Lora) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html) |

## Documentation

- [`Pamoja.Lora` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html), every type in this namespace.
- [The LoRa airtime guide](https://pamoja.molex.cloud/docs/guides/lora.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
