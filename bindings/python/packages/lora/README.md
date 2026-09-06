# pamoja-lora

Time-on-air, duty-cycle off-time, and the regional channel plans a LoRa node must keep to. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/lora.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-lora
```

```python
from pamoja import lora
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-lora`](https://crates.io/crates/pamoja-lora) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html), [docs.rs](https://docs.rs/pamoja-lora), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-lora) |
| TypeScript | [`@pamoja/lora`](https://www.npmjs.com/package/@pamoja/lora) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_lora.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-lora) |
| Python | [`pamoja-lora`](https://pypi.org/project/pamoja-lora/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-lora) |
| C# | [`Pamoja.Lora`](https://www.nuget.org/packages/Pamoja.Lora) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Lora.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-lora) |

## Documentation

- [`pamoja.lora` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/lora.html), every class and function in this module.
- [The LoRa airtime guide](https://pamoja.molex.cloud/docs/guides/lora.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
