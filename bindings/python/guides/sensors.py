"""The sensor-driver guide example; see docs/guides/sensors.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.sensors import ds18b20, ina219

# Stand-ins for the two parts. On a running node the thermometer's nine bytes come off
# the 1-Wire bus and the monitor's registers off I2C; here the library builds what each
# would send, so the program runs with nothing plugged in.
thermometer = ds18b20.build_scratchpad(25.0625, 12, 75, -10)

# The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
# datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
CURRENT_LSB = 1_000
bus = ina219.bus_register(11_980)
current = ina219.current_register(10_000_000, CURRENT_LSB)
power = ina219.power_register(119_800_000, CURRENT_LSB)

# Everything below is the node's own code. The thermometer checksums every read, so a
# reading is verified before it is believed.
reading = ds18b20.parse_scratchpad(thermometer)
print(f"temperature  {reading.celsius:.4f} C")
print(f"resolution   {reading.resolution_bits} bits")
print(f"alarms       {reading.alarm_high} / {reading.alarm_low} C")

# The monitor computes nothing until it has been told what shunt it is across.
print(f"calibration  0x{ina219.calibration(CURRENT_LSB, 2):04X}")
print(f"bus          {ina219.bus_millivolts(bus)} mV")
print(f"current      {ina219.current_microamps(current, CURRENT_LSB) // 1_000} mA")
print(f"power        {ina219.power_microwatts(power, CURRENT_LSB) // 1_000} mW")

# A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
# instead of logging a temperature a couple of degrees off.
corrupted = bytearray(thermometer)
corrupted[0] ^= 1
try:
    ds18b20.parse_scratchpad(bytes(corrupted))
    print("corrupt read accepted, which should never happen")
except PamojaError as error:
    print(f"corrupt read rejected: {error}")
# ANCHOR_END: example

assert reading.raw_temperature == 0x0191
assert reading.micro_celsius == 25_062_500
assert reading.resolution_bits == 12
assert reading.alarm_high == 75
assert reading.alarm_low == -10

# The datasheet's own figures for that design: calibration 0x5000, and registers that
# read back 11.98 V, 10 A, and 119.8 W.
assert ina219.calibration(CURRENT_LSB, 2) == 0x5000
assert ina219.bus_millivolts(bus) == 11_980
assert ina219.current_microamps(current, CURRENT_LSB) == 10_000_000
assert ina219.power_microwatts(power, CURRENT_LSB) == 119_800_000

# The published check value for CRC-8/MAXIM-DOW, the checksum every 1-Wire part appends
# to what it sends.
assert ds18b20.crc8(b"123456789") == 0xA1
