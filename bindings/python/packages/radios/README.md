# pamoja-radios

The Semtech SX126x and SX127x LoRa radios and the SX1302 and SX1303 gateway concentrators: their commands, registers, and decoders, the amplifier setting a regional EIRP ceiling allows, a duty-cycle guard, and simulated chips that stand in for a module. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/radios.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html)

## Install

```sh
pip install pamoja-radios
```

```python
from pamoja import radios
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-radios`](https://crates.io/crates/pamoja-radios) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html), [docs.rs](https://docs.rs/pamoja-radios), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-radios) |
| TypeScript | [`@pamoja/radios`](https://www.npmjs.com/package/@pamoja/radios) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_radios.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-radios) |
| Python | [`pamoja-radios`](https://pypi.org/project/pamoja-radios/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-radios) |
| C# | [`Pamoja.Radios`](https://www.nuget.org/packages/Pamoja.Radios) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Radios.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-radios) |

## Documentation

- [`pamoja.radios` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/radios.html), every class and function in this module.
- [The LoRa radios guide](https://pamoja.molex.cloud/docs/guides/radios.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
