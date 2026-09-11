# Pamoja.Radios

The Semtech SX126x LoRa command set and its decoders, the amplifier setting a regional EIRP ceiling allows, and a duty-cycle guard. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
