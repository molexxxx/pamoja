"""The first example on the README and the site: a reading off a wire, smoothed,
signed, and packed for a metered link, with nothing plugged in."""

# ANCHOR: example
from pamoja import sensors
from pamoja.codec import pack_samples, unpack_samples
from pamoja.kit import Smoother
from pamoja.security import DeviceIdentity, verify

# The nine bytes a DS18B20 sends, CRC last; a bad CRC is a rejected read.
scratchpad = bytearray([0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00])
scratchpad[8] = sensors.ds18b20.crc8(bytes(scratchpad[:8]))
celsius = sensors.ds18b20.parse_scratchpad(bytes(scratchpad)).micro_celsius / 1e6
assert celsius == 25.0625

# Smooth the noise out of successive readings.
smoother = Smoother(0.5)
smoother.update(celsius)
smoothed = smoother.update(celsius + 1.0)
assert celsius < smoothed < celsius + 1.0

# Sign the reading so a gateway can prove which device sent it.
device = DeviceIdentity.from_seed(bytes([7]) * 32)
payload = f"{smoothed:.2f}"
signature = device.sign(payload)
assert verify(device.public_key, payload, signature)

# Pack a batch of readings for a link where every byte costs money.
samples = [2506, 2507, 2509, 2508, 2510]
packed = pack_samples(samples)
assert len(packed) < len(samples) * 8
assert unpack_samples(packed) == samples
# ANCHOR_END: example
