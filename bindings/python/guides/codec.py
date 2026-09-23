"""The codecs guide example; see docs/guides/codec.md."""

# ANCHOR: example
import json
import math

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples
from pamoja.core import PamojaError
from pamoja.lora import plan_for

# The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because it
# reaches farthest. An uplink there carries only a few bytes of payload.
budget = plan_for("US915").max_payload(0).application


def fits(size: int) -> str:
    return "fits one uplink" if size <= budget else "too big for one uplink"


print(f"uplink    carries {budget} bytes at the slowest US915 data rate")

# One reading as the JSON a web service would take. CBOR carries the same document in
# fewer bytes, but every key name still rides along with every reading.
reading = {"depth_cm": 142.5, "air_c": -6.5, "battery_mv": 3712}
as_json = json.dumps(reading, separators=(",", ":")).encode()
cbor = to_cbor(reading)
print(f"json      {len(as_json)} bytes, {fits(len(as_json))}")
print(f"cbor      {len(cbor)} bytes, {fits(len(cbor))}")
print(f"cbor      reads back as {json.dumps(from_cbor(cbor), separators=(',', ':'))}")

# A batch the gauge and the server agree on needs no key names. Six hourly depths, kept to
# the millimeter, pack to a count, the first depth, and five small steps.
quantizer = Quantizer(10)
depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5]
depth_batch = quantizer.encode(depths)
depth_bytes = len(depth_batch)
print(f"depths    {len(depths)} readings in {depth_bytes} bytes, {fits(depth_bytes)}")
depths_back = [f"{depth:.1f}" for depth in quantizer.decode(depth_batch)]
print(f"depths    read back as {', '.join(depths_back)}")

# Battery millivolts are whole numbers already, so they pack with no scale, and a falling
# voltage packs as small as a rising one.
battery = [3712, 3709, 3705, 3702, 3698, 3695]
battery_batch = pack_samples(battery)
battery_bytes = len(battery_batch)
print(f"battery   {len(battery)} readings in {battery_bytes} bytes, {fits(battery_bytes)}")
print(f"battery   reads back as {', '.join(map(str, unpack_samples(battery_batch)))}")

# Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The quantizer
# refuses the batch rather than send the gap as a depth.
try:
    quantizer.encode([145.5, math.nan])
except PamojaError as error:
    print(f"depths    refused a batch with a missing depth: {error}")
# ANCHOR_END: example

assert len(cbor) < len(as_json)
assert len(cbor) > budget
assert depth_bytes <= budget and battery_bytes <= budget
assert list(from_cbor(cbor)) == ["air_c", "battery_mv", "depth_cm"], "keys come back sorted"
assert all(abs(got - sent) <= 0.05 for got, sent in zip(quantizer.decode(depth_batch), depths))
assert unpack_samples(battery_batch) == battery
