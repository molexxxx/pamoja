"""The Modbus RTU guide example: a gateway at a village water pump polls an energy meter and a
relay module on one RS485 line; see docs/guides/modbus.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert registers == [2301, 418, 0]
assert (out, back) == (8, 11)
assert low
assert states == [True, False, False, False]
assert relays.coil(0) is False
assert refused is not None and refused.kind == "exception"
assert silent is not None and (silent.kind, silent.unit) == ("timeout", 19)
