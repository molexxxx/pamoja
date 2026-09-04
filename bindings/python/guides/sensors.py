"""The sensor-driver guide example; see docs/guides/sensors.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.sensors import ds18b20, ina219

# Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published check
# value over the ASCII digits 1 to 9 is 0xA1.
assert ds18b20.crc8(b"123456789") == 0xA1

# A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over the
# other eight, so a reading is verified before it is believed.
scratchpad = bytearray([0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00])
scratchpad[8] = ds18b20.crc8(bytes(scratchpad[:8]))
reading = ds18b20.parse_scratchpad(bytes(scratchpad))

# Register 0x0191 is the +25.0625 degree row of the datasheet's temperature table, each
# count a sixteenth of a degree, so micro-degrees stay exact in integer arithmetic.
assert reading.raw_temperature == 0x0191
assert reading.micro_celsius == 25_062_500
assert reading.resolution_bits == 12
assert reading.alarm_high == 75

# A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a plausible
# temperature a few degrees off.
corrupt = bytearray(scratchpad)
corrupt[0] ^= 0x01
try:
    ds18b20.parse_scratchpad(bytes(corrupt))
except PamojaError:
    pass
else:
    raise AssertionError("a scratchpad corrupted on the bus should be rejected")

# The INA219 datasheet's worked design example: 1 mA per count across a 2 milliohm
# shunt calibrates to 0x5000, and its registers then read 11.98 V, 10 A, and 119.8 W.
current_lsb = 1_000
assert ina219.calibration(current_lsb, 2) == 0x5000
assert ina219.bus_millivolts(0x5D98) == 11_980
assert ina219.current_microamps(0x2710, current_lsb) == 10_000_000
assert ina219.power_microwatts(0x1766, current_lsb) == 119_800_000
# ANCHOR_END: example
