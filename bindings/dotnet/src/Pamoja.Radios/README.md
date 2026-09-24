# Pamoja.Radios

The Semtech SX126x and SX127x LoRa radios and the SX1302 and SX1303 gateway concentrators: their commands, registers, and decoders, the amplifier setting a regional EIRP ceiling allows, a duty-cycle guard, and simulated chips that stand in for a module. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/radios.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html)

## Install

```sh
dotnet add package Pamoja.Radios
```

```csharp
using Pamoja.Radios;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Lora`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-radios`](https://crates.io/crates/pamoja-radios) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [docs.rs](https://docs.rs/pamoja-radios), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios) |
| TypeScript | [`@pamoja/radios`](https://www.npmjs.com/package/@pamoja/radios) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios) |
| Python | [`pamoja-radios`](https://pypi.org/project/pamoja-radios/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios) |
| C# | [`Pamoja.Radios`](https://www.nuget.org/packages/Pamoja.Radios) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios) |

## Documentation

- [`Pamoja.Radios` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), every type in this namespace.
- [The LoRa radios guide](https://pamoja.molex.cloud/docs/guides/radios.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
