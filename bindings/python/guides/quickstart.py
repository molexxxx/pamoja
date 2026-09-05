"""The first example on the README and the site: a reading taken off a wire on a field
node, sent over a link, and checked on the gateway that receives it, with nothing plugged
in and nothing running."""

# ANCHOR: example
import asyncio

from pamoja import sensors
from pamoja.codec import pack_samples, unpack_samples
from pamoja.kit import Smoother
from pamoja.loopback import LoopbackBroker
from pamoja.security import DeviceIdentity, fingerprint, verify_message

# The device's identity is provisioned once and never leaves it. The gateway is told only
# the public half, which is how it recognises this device later.
SEED = bytes([7]) * 32
TOPIC = "sensors/1/temperature"


async def main() -> bytes:
    # The link. A loopback broker stands in for MQTT or CoAP, so this runs with no network
    # and nothing listening. Point the node at a real transport and nothing below changes.
    broker = LoopbackBroker()
    node = broker.link()
    gateway = broker.link()
    await node.connect()
    await gateway.connect()
    await gateway.subscribe(TOPIC)

    device = DeviceIdentity.from_seed(SEED)
    known = device.public_key
    print(f"gateway trusts device {fingerprint(known)}")

    # A stand-in for the thermometer. On a running node these nine bytes arrive from the
    # 1-Wire bus; here the library builds what a part at 25.0625 C would send.
    off_the_bus = sensors.ds18b20.build_scratchpad(25.0625, 12, 75, -10)

    # On the node. The part checksums every read, so a value mangled on a long run is an
    # error rather than a plausible temperature a couple of degrees off.
    celsius = sensors.ds18b20.parse_scratchpad(off_the_bus).micro_celsius / 1e6
    print(f"read      {celsius:.4f} C")

    # Readings jitter, so smooth them, and send a batch rather than one at a time.
    # Successive readings differ by very little, so the differences cost a fraction of
    # what the readings would on a link that charges by the byte.
    smoother = Smoother(0.5)
    batch = [
        round(smoother.update(sample) * 100)
        for sample in (celsius, celsius + 0.5, celsius + 0.4)
    ]
    packed = pack_samples(batch)
    print(f"packed    {len(batch)} readings into {len(packed)} bytes")

    # Sign the batch and send it. The signature travels with the payload as one message,
    # so there is nothing to keep together and split correctly at the far end.
    await node.send(TOPIC, device.sign_message(packed))

    # On the gateway. Verifying returns the payload, so a reading that was altered on the
    # way, or signed by some other device, never reaches the code that unpacks it.
    received = await gateway.recv()
    payload = verify_message(known, received.payload)
    if payload is None:
        print("gateway   rejected the reading")
    else:
        print(f"gateway   accepted {unpack_samples(payload)} in hundredths of a degree")

    return received.payload


message = asyncio.run(main())
# ANCHOR_END: example

known_key = DeviceIdentity.from_seed(SEED).public_key
assert unpack_samples(verify_message(known_key, message)) == [2506, 2531, 2539]
assert len(message) < 64 + 3 * 8

# A message edited in transit does not verify, so the gateway never unpacks it.
edited = bytearray(message)
edited[-1] ^= 0xFF
assert verify_message(known_key, bytes(edited)) is None
