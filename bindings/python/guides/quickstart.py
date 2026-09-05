"""The first example on the README and the site: one field node's reading taken off a
wire, smoothed, signed, and packed for a link that charges by the byte, start to finish
with nothing plugged in."""

# ANCHOR: example
from pamoja import sensors
from pamoja.codec import pack_samples, unpack_samples
from pamoja.kit import Smoother
from pamoja.security import DeviceIdentity, verify

# A stand-in for the thermometer. On a running node these nine bytes arrive from the
# 1-Wire bus; here the library builds what a part sitting at 25.0625 C would send, so the
# program runs with nothing plugged in.
off_the_bus = sensors.ds18b20.build_scratchpad(25.0625, 12, 75, -10)

# Everything below is the node's own code, and none of it cares where the bytes came from.
# The part checksums every read, so a value mangled on a long run comes back as an error
# instead of a plausible temperature a couple of degrees off.
scratchpad = sensors.ds18b20.parse_scratchpad(off_the_bus)
celsius = scratchpad.micro_celsius / 1e6
print(f"read      {celsius:.4f} C")  # read      25.0625 C

# Readings jitter. A smoother follows the trend without keeping a history to do it, which
# matters on a part with kilobytes of RAM.
smoother = Smoother(0.5)
smoother.update(celsius)
smoothed = smoother.update(celsius + 1.0)
print(f"smoothed  {smoothed:.4f} C")  # smoothed  25.5625 C

# Sign it, so the gateway can tell this device's readings from anyone else's.
device = DeviceIdentity.from_seed(bytes([7]) * 32)
reading = f"{smoothed:.2f}"
signature = device.sign(reading)
if not verify(device.public_key, reading, signature):
    raise SystemExit("the gateway would reject this reading")
print(f"signed    {reading} C, and the signature checks out")

# Send a batch rather than a reading at a time. Successive samples differ by very little,
# so writing down the differences costs a fraction of eight bytes each.
batch = [2506, 2507, 2509, 2508, 2510]
packed = pack_samples(batch)
print(f"packed    {len(batch)} readings into {len(packed)} bytes")
# ANCHOR_END: example

assert celsius == 25.0625
assert celsius < smoothed < celsius + 1.0
assert verify(device.public_key, reading, signature)
assert len(packed) < len(batch) * 8
assert unpack_samples(packed) == batch
