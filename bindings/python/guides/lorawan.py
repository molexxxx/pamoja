"""The LoRaWAN activation guide example; see docs/guides/lorawan.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.lorawan import device, grant

# The root key is provisioned into the device at the factory and known to the network
# server. It is the only secret either side starts with; any 16 bytes stand in here.
app_key = bytes([7]) * 16

# The device asks to join with a nonce it has not used before, which is what stops an old
# accept being replayed at it.
dev_nonce = 1
node = device(bytes(8), bytes(8), app_key)

# The network grants the join. It draws its own nonce, names the network the device is
# joining, and assigns the address the device will answer to from then on.
dev_addr = 0x26012E43
offer = grant(app_nonce=2, net_id=19, dev_addr=dev_addr)
accept = offer.accept(app_key, dev_nonce)
print(f"granted   address 0x{dev_addr:08X} in a {len(accept)}-byte accept")

# The device verifies it against the root key. A join accept carries no device identifier,
# so only that key decides whether it is for this device.
joined = node.accept_join(accept, dev_nonce)
print(f"joined    the device took address 0x{joined.dev_addr:08X}")

# Neither side transmits a session key. Both derive the same pair from the root key and the
# two nonces, so the network reads what the device sends without ever having been told how.
network = offer.session(app_key, dev_nonce)
uplink = joined.session().encode_uplink(1, 1, b"level=high")
received = network.decode(uplink, 1)
print(f"uplink    the network read {received.payload.decode()}")

# A single byte changed in the air fails that check, so no one else can admit the device or
# put words in its mouth.
forged = bytearray(accept)
forged[1] ^= 0xFF
try:
    node.accept_join(bytes(forged), dev_nonce)
    print("a forged accept was taken, which should never happen")
except PamojaError as error:
    print(f"forged    accept refused: {error}")
# ANCHOR_END: example

assert joined.dev_addr == dev_addr
assert received.payload == b"level=high"

# ANCHOR: device
from pamoja.lora import plan_for
from pamoja.lorawan import DeviceError, DeviceSettings, ReceiveWindow, end_device

root_key = bytes([7]) * 16
dev_eui = bytes.fromhex("70b3d57ed0051234")
join_eui = bytes(8)

# The device owns no radio and no clock. It takes the time in microseconds and says what to
# put on the air, so the same code runs over an SX1276, an SX1262, or nothing at all. This
# one's radio puts out 2 to 20 dBm, and its seed would come from the radio's noise.
plan = plan_for("US915")
settings = DeviceSettings(2, 20, seed=1)
node = end_device(plan, dev_eui, join_eui, root_key, settings)

# A US915 device joins in passes over the band, one 125 kHz channel from each group of eight.
# The accept is due on the downlink channel that join channel is answered on.
request = node.join(1, 0)
print(
    f"join      {request.frequency_hz / 1e6:.1f} MHz at DR{request.data_rate}, "
    f"{request.output_dbm} dBm, {request.airtime_us // 1000} ms on air; "
    f"the accept is due {request.rx1.delay_us // 1_000_000} s later on "
    f"{request.rx1.frequency_hz / 1e6:.1f} MHz"
)

# The network answers, and the device takes its address and session from the accept.
network = grant(app_nonce=2, net_id=19, dev_addr=0x26012E43)
heard = node.heard(network.accept(root_key, 1), 7)
if heard.kind == "joined":
    print(f"joined    as 0x{heard.dev_addr:08X}")

# A confirmed reading. While it waits on its windows, the device refuses to send another.
reading = node.send(2, b"21.5", 10_000_000, confirmed=True)
print(
    f"uplink    {reading.frequency_hz / 1e6:.1f} MHz at DR{reading.data_rate}; "
    f"the answer is due {reading.rx1.delay_us // 1_000_000} s later on "
    f"{reading.rx1.frequency_hz / 1e6:.1f} MHz"
)
try:
    node.send(2, b"21.6", 10_000_000)
except DeviceError as error:
    if error.kind == "busy":
        print("busy      the reading before still waits on its windows")

# The network acknowledges it in the first window and sends a setting back on the same
# port. Naming the window holds the frame to the length that window's data rate carries.
answer = network.session(root_key, 1).encode_downlink(0, 2, b"set=19.0", ack=True)
downlink = node.heard(answer, 7, ReceiveWindow.RX1)
if downlink.kind == "data":
    delivery = downlink.delivery
    acknowledged = "true" if delivery.acknowledged else "false"
    print(
        f"downlink  acknowledged: {acknowledged}, port {delivery.port or 0} "
        f"says {delivery.payload.decode()}"
    )

# Before sleeping, the device saves what it settled with the network. After the power cut a
# fresh device resumes it on a clock that starts over, and sends its next reading with no join.
saved = node.save(12_000_000)
woken = end_device(plan, dev_eui, join_eui, root_key, settings)
woken.resume(saved, 0)
following = woken.send(2, b"21.7", 5_000_000)
print(
    f"resumed   {len(saved)} saved bytes; the next reading goes out as uplink "
    f"{woken.fcnt_up - 1} without joining again"
)
# ANCHOR_END: device

assert following.carries_payload
assert woken.dev_addr == 0x26012E43


# ANCHOR: relay
from pamoja.gateway import Network, Rxpk
from pamoja.lorawan import Relay

# One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor in a
# cellar the gateway cannot hear at all.
relay_eui = bytes.fromhex("70b3d57ed0050001")
sensor_eui = bytes.fromhex("70b3d57ed0050002")
band = plan_for("EU868")
radio = DeviceSettings(2, 14, lowest_hz=863_000_000, highest_hz=870_000_000, seed=1)
site = Network(band, 0x00002A, first_dev_addr=0x26010001)
site.register(relay_eui, join_eui, root_key)
site.register(sensor_eui, join_eui, root_key)


def heard_at(transmission, at_us):
    """Hand the network what a gateway heard of one transmission."""
    return Rxpk(
        transmission.frequency_hz,
        transmission.frame,
        link=transmission.link,
        timestamp_us=at_us,
    )


# The relay is an end device that also listens for others, so it joins the ordinary way.
rooftop = Relay.over_the_air(
    band, device(relay_eui, join_eui, root_key), radio, "ppm20", "symbols4"
)
relay_join = rooftop.join(1, 1_000_000)
relay_accept = site.uplink(heard_at(relay_join, 1_000_000))
rooftop.heard_in("rx1", relay_accept.accept.payload, 7)
print(f"relay     joined as 0x{rooftop.dev_addr:08X}")

# The sensor joins too. Its own uplinks never reach the gateway, but its join does, because
# the cellar door is open while it is installed.
cellar = end_device(band, sensor_eui, join_eui, root_key, radio)
sensor_join = cellar.join(2, 20_000_000)
sensor_accept = site.uplink(heard_at(sensor_join, 20_000_000))
cellar.heard(sensor_accept.accept.payload, 7, ReceiveWindow.RX1)
sensor_addr = cellar.dev_addr

# The network hands the relay the key that lets it verify the sensor's wake-up frames, in a
# command riding on the relay's own downlink.
relay_empty = rooftop.send_empty(40_000_000)
relay_carried = site.uplink(heard_at(relay_empty, 40_000_000))
trust = site.trust_command(sensor_addr, 0)
configure = site.command(rooftop.dev_addr, relay_carried.slot, [trust])
rooftop.heard_in("rx1", configure.payload, 7)
print(f"trusted   the relay now forwards for 0x{sensor_addr:08X}")

# It scans once a second on the region's wake-on-radio channel.
rooftop.start("ms1000", 0)
scan = rooftop.next_scan(60_000_000)
print(f"scan      {scan.carrier.frequency_hz / 1e6:.1f} MHz at DR{scan.carrier.data_rate} every second")

# The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble spans
# a whole scan period, because it does not yet know when the relay listens.
cellar.use_relay(True)
relayed_reading = cellar.send(2, b"21.5", 61_000_000)
exchange = relayed_reading.relay
print(
    f"wake      {len(exchange.wake_up.frame)} bytes with a "
    f"{exchange.wake_up.link.preamble_symbols}-symbol preamble, "
    f"{(exchange.uplink_start_us - exchange.wake_up.start_us) // 1000} ms before the uplink"
)

# The relay hears it, knows the device, and answers with when it scanned, so every frame after
# this one carries only the preamble the two clocks could have drifted apart.
woke = rooftop.heard_wor(scan, exchange.wake_up.frame, -90, 4, scan.start_us + 500_000)
said = cellar.heard_wor_ack(woke.acknowledgment.frame)
print(
    f"ack       the relay scans every {int(said.cad_periodicity[2:])} ms and forwards at "
    f"DR{said.relay_data_rate}"
)

# The uplink follows, and the relay wraps it in one of its own on port 226.
due_us = rooftop.heard_uplink(relayed_reading.frame, -88, 6, woke.listen.start_us + 100_000)
forwarded = rooftop.forward(due_us)
relayed = site.uplink(heard_at(forwarded, due_us))
print(
    f"forwarded {relayed.payload.decode()} from 0x{relayed.dev_addr:08X}, "
    f"heard by 0x{relayed.relay.relay:08X} at {relayed.relay.rssi_dbm} dBm"
)

# The answer goes back the same way: the network answers the sensor, the relay unwraps it and
# sends it on, and the sensor hears it in the window it keeps for a relay.
relayed_answer = site.answer(sensor_addr, relayed.slot, 2, b"set=19.0")
passed = rooftop.heard_in("rx1", relayed_answer.payload, 7)
delivered = cellar.heard(passed.downlink.frame, 7, ReceiveWindow.RXR)
print(
    f"downlink  port {delivered.delivery.port} says {delivered.delivery.payload.decode()}, "
    f"{exchange.rxr.delay_us // 1_000_000} s after the uplink"
)
# ANCHOR_END: relay
