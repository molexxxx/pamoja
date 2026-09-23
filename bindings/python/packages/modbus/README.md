# pamoja-modbus

Modbus RTU for RS485 field devices: a client that polls them over a serial port with the line's timing, simulated devices that answer as real ones do, and the frames with their CRC-16/MODBUS. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html)

## Install

```sh
pip install pamoja-modbus
```

```python
from pamoja import modbus
```

This pulls in `pamoja-native`, the compiled engine, and `pamoja-hal`. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
from pamoja.hal import Parity, SerialSettings
from pamoja.modbus import BROADCAST, ModbusClient, ModbusClientError, ModbusLine, ModbusServer


def word(on: bool) -> str:
    """A state as the output prints it."""
    return "on" if on else "off"


# The line: 19200 baud, even parity, one stop bit, the default the Modbus specification sets.
# Eleven bits a character, and 3.5 of them of silence mark where a frame ends.
settings = SerialSettings(19_200, Parity.EVEN)
gap = ModbusClient.frame_gap_nanos(settings) // 1_000
print(f"line         {settings}, {settings.bits_per_character} bits a character, t3.5 is {gap} us")

# Each device's manual gives its unit address and where its values live. The meter keeps its
# measurements in input registers from 0: volts in tenths, amps in hundredths, then a fault
# word. The relay module has four relays as coils 0 to 3, and the tank's low-level float switch
# as discrete input 0, on while the water is below it.
METER = 17
PUMP = 18
meter = ModbusServer(METER)
meter.set_input_registers(0, [2301, 418, 0])
relays = ModbusServer(PUMP)
relays.set_coils(0, [False] * 4)
relays.set_discrete_inputs(0, [True])
line = ModbusLine().attach(meter).attach(relays)

# The devices sit on a simulated line. On a gateway the port is
# SerialPort.open("/dev/ttyUSB0", settings), and nothing after this statement changes.
port = line.port(settings)
client = ModbusClient(port)

# Poll the meter with function 0x04 for three input registers, and scale each one as its manual
# says.
registers = client.read_input_registers(METER, 0, 3)
print(f"meter        {registers[0] / 10:.1f} V, {registers[1] / 100:.2f} A, faults {registers[2]}")

# What that poll cost the line: the request, the reply, and the silence before the request.
out, back = port.written, port.received
line_time = settings.transfer_micros(out) + settings.transfer_micros(back) + gap
print(f"poll         {out} bytes out, {back} back, {line_time / 1_000:.2f} ms of line time")

# Read the float switch, and start the pump on relay 0 when the tank is low.
[low] = client.read_discrete_inputs(PUMP, 0, 1)
print(f"tank         low-level switch {word(low)}")
if low:
    client.write_single_coil(PUMP, 0, True)
states = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in states)}")

# A broadcast, to unit 0, reaches every device on the line and none answers: here every relay
# off at once. The client waits out the turnaround so each device has carried it out before the
# next request.
client.write_multiple_coils(BROADCAST, 0, [False] * 4)
print(f"broadcast    every relay off, no reply, {round(client.turnaround * 1_000)} ms turnaround")
after = client.read_coils(PUMP, 0, 4)
print(f"relays       {' '.join(word(on) for on in after)}")

# The meter keeps its measurements in input registers. Asking for them as holding registers,
# function 0x03, is the usual mistake with a new device, and the meter refuses it with an
# exception instead of answering.
refused = None
try:
    client.read_holding_registers(METER, 0, 3)
except ModbusClientError as error:
    refused = error
    print(f"refused      {error}")

# A unit that is not on the line never answers. The client gives up after its response timeout,
# one second unless told otherwise, which a simulated line counts instead of sleeping through.
before = port.waited_micros
silent = None
try:
    client.read_input_registers(19, 0, 1)
except ModbusClientError as error:
    silent = error
    waited = (port.waited_micros - before) // 1_000
    print(f"silent       {error}, {waited} ms counted and not slept")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus) |

## Documentation

- [`pamoja.modbus` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), every class and function in this module.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
