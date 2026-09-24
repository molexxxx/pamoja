"""The LoRaWAN gateway guide example; see docs/guides/gateway.md."""

# ANCHOR: example
from pamoja.gateway import (
    Packet,
    PacketKind,
    Rxpk,
    Stat,
    Txpk,
    TxStatus,
    acknowledgment,
    encode,
    parse,
)
from pamoja.lora import link
from pamoja.lorawan import session

# A gateway on a Raspberry Pi, whose identifier is written from its network interface.
gateway = "b827ebfffe010203"

# Every few seconds it sends a PULL_DATA, which holds a path open through whatever translates
# its address, so the server has somewhere to send a downlink. The server answers each one,
# and a gateway that stops hearing answers knows the path is gone.
pull = encode(Packet(PacketKind.PULL_DATA, 0x7A01, gateway=gateway))
held = acknowledgment(parse(pull))
print(f"pull      {len(pull)} bytes out and {len(encode(held))} back hold the downlink path open")

# A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge of its
# range. It forwards the frame as it arrived, with the levels, the concentrator's own
# timestamp, and its counts since the last report. It holds no key and reads none of it.
node = session(0x26010001, bytes([0x44]) * 16, bytes([0x55]) * 16)
frame = node.encode_uplink(7, 2, b"21.5")
heard = Rxpk(
    868_100_000,
    frame,
    link=link(9, 125_000),
    rssi_dbm=-97,
    snr_db=-3.2,
    timestamp_us=3_512_348_611,
)
counts = Stat(received=2, received_ok=1, forwarded=1, acknowledged_percent=100.0)
datagram = encode(Packet(PacketKind.PUSH_DATA, 0x1234, gateway=gateway, packets=[heard], status=counts))
print(f"push      a reading and the gateway's counts, {len(datagram)} bytes, token {0x1234:04x}")

# The server reads it. The frequency is in hertz, the datarate identifier is the link
# settings, and the payload is bytes, so nothing is decoded by hand.
forwarded = parse(datagram)
received = forwarded.packets[0]
print(
    f"heard     {received.frequency_hz} Hz at SF{received.link.spreading_factor}, "
    f"{received.link.bandwidth_hz // 1000} kHz, {received.rssi_dbm:.0f} dBm, "
    f"SNR {received.snr_db:.1f} dB, CRC {received.crc.lower()}, {len(received.payload)} bytes"
)
report = forwarded.status
print(
    f"counts    {report.received} received, {report.received_ok} with a good CRC, "
    f"{report.forwarded} forwarded, {report.acknowledged_percent:.1f}% acknowledged"
)

# It is acknowledged at once, by token, before anything in it is read.
ack = acknowledgment(parse(datagram))
print(f"ack       token {ack.token:04x} acknowledged in {len(encode(ack))} bytes")

# An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for the
# device's first receive window, a second after the uplink ended, with the inverted polarity
# a LoRaWAN device listens for.
answer = node.encode_downlink(0, 2, b"ok")
window = Txpk(
    868_100_000,
    answer,
    link=link(9, 125_000),
    timestamp_us=3_513_348_611,
    power_dbm=14,
    invert_polarity=True,
)
transmit = parse(encode(Packet(PacketKind.PULL_RESP, 0x00AB, transmit=window))).transmit
iq = "IQ inverted" if transmit.invert_polarity else "IQ upright"
print(
    f"downlink  at {transmit.timestamp_us} us on {transmit.frequency_hz} Hz, "
    f"{transmit.power_dbm} dBm, {iq}"
)

# The gateway answers each PULL_RESP with a TX_ACK saying what became of it: scheduled, or
# refused with a reason, such as a window that had already passed.
for said in (TxStatus.NONE, TxStatus.TOO_LATE):
    reported = encode(Packet(PacketKind.TX_ACK, 0x00AB, gateway=gateway, tx_status=said))
    status = parse(reported).tx_status
    meaning = "it goes out in the device's window" if status == TxStatus.NONE else "it was not sent"
    print(f"txack     {status}: {meaning}")
# ANCHOR_END: example

assert len(encode(held)) == 4
assert received.payload == frame
assert ack.token == 0x1234
assert transmit.timestamp_us == 3_513_348_611

# ANCHOR: network
from pamoja.core import PamojaError
from pamoja.gateway import Network
from pamoja.lora import plan_for
from pamoja.lorawan import device

# One site, on the band it operates in, admitting one device it was told about: its EUI from
# its label, the application it joins, and the root key it was provisioned with.
dev_eui = bytes.fromhex("70b3d57ed0001234")
join_eui = bytes.fromhex("70b3d57ed0000000")
app_key = bytes([0x2B]) * 16
site = Network(plan_for("EU868"), 0x000013, first_dev_addr=0x26010001)
site.register(dev_eui, join_eui, app_key)

# The gateway forwards a join request it heard. Nothing about the device is known here beyond
# the key, which is what verifies the request, and the accept is timed for the join window,
# five seconds after the request.
eu868 = plan_for("EU868")
dr5 = eu868.link_settings(5)
joiner = device(dev_eui, join_eui, app_key)
heard_at = 1_000_000
joined = site.uplink(
    Rxpk(868_100_000, joiner.join_request(0x0102), link=dr5, timestamp_us=heard_at)
)
accepted_at = joined.accept.timestamp_us
print(
    f"joined    {joined.dev_addr:#010x}, accepted at {accepted_at} us, "
    f"{(accepted_at - heard_at) // 1_000_000} s after the request"
)

# The device reads the accept and sends a reading. The site decrypts it and says where an
# answer goes: the uplink's own channel, a second after it ended.
granted = joiner.accept_join(joined.accept.payload, 0x0102).session()
carried = Rxpk(
    868_100_000,
    granted.encode_uplink(0, 2, b"21.5"),
    link=dr5,
    timestamp_us=9_000_000,
)
reading = site.uplink(carried)
print(
    f"uplink    frame {reading.fcnt} on port {reading.fport} says {reading.payload.decode()}, "
    f"answer at {reading.slot.timestamp_us} us on {reading.slot.frequency_hz} Hz"
)

# The answer goes out in that window, encrypted with the session the join granted.
reply = site.answer(reading.dev_addr, reading.slot, 2, b"ok")
print(f"answer    {len(reply.payload)} bytes at {reply.timestamp_us} us")

# The same frame again, as a replay would send it, is refused: its counter was seen.
try:
    site.uplink(carried)
    raise AssertionError("a counter is taken once")
except PamojaError as refused:
    print(f"replay    {refused}")

# A gateway hears every network in range, and a frame from one this site never granted is
# reported as another network's rather than refused.
elsewhere = session(0x12345678, bytes([0x09]) * 16, bytes([0x08]) * 16)
stranger = site.uplink(
    Rxpk(868_300_000, elsewhere.encode_uplink(0, 1, b"hello"), link=dr5)
)
print(f"foreign   {stranger.dev_addr:#010x} belongs to another network")
# ANCHOR_END: network

assert reading.payload == b"21.5"
assert reading.slot.timestamp_us == 10_000_000
assert stranger.outcome == "foreign"

# ANCHOR: station
from pamoja.core import version
from pamoja.gateway import (
    DISCOVERY_PATH,
    STATION_PROTOCOL_VERSION,
    StationKind,
    StationLevels,
    StationMessage,
    StationWindow,
    station_discovery,
    station_discovery_parse,
    station_encode,
    station_heard,
    station_parse,
    station_router_accepted,
    station_router_parse,
    station_xtime,
    station_xtime_parts,
)

# The station asks its configured address where its network server is, naming itself. The
# server reads who asked, and sends it to the websocket its session runs on.
station = "b827ebfffe010203"
asking = station_discovery(station)
print(f"ask       {DISCOVERY_PATH} {asking}")
asked = station_discovery_parse(asking)
answer = station_router_accepted(asked, "0000000000000001", "ws://lns.example.invalid:3001/router")
print(f"open      {station_router_parse(answer).uri}")

# Once the websocket is open the station speaks first, saying what it is.
hello = station_encode(
    StationMessage(
        StationKind.VERSION,
        station="pamoja",
        firmware=version(),
        package="pamoja-gateway",
        model="linux",
        protocol=STATION_PROTOCOL_VERSION,
        features="gps",
    )
)
said = station_parse(hello)
print(f"version   {said.station} {said.firmware} on {said.model}, protocol {said.protocol}")

# The radio hears a node's reading 3512.348611 seconds into the station's first run. A
# station holds no key, so it splits the frame into the fields the protocol names and lets the
# server judge them, with its own clock for the moment it arrived.
heard_at = station_xtime(0, 1, 3_512_348_611)
updf = station_encode(
    station_heard(
        node.encode_uplink(7, 2, b"21.5"),
        5,
        868_100_000,
        StationLevels(rctx=0, xtime=heard_at, rssi=-97.0, snr=-3.2),
    )
)
uplink = station_parse(updf)
print(
    f"updf      {uplink.dev_addr:#010x} counter {uplink.fcnt} on port {uplink.fport}, "
    f"DR{uplink.data_rate}, {len(uplink.payload)} bytes still encrypted"
)

# The server answers in the receive windows the region gives: the first at the uplink's own
# rate and channel, the second where the plan fixes it. It hands the station's clock back
# untouched, so the station can time the answer from the moment it heard the uplink.
rx1_rate = eu868.rx1_data_rate(uplink.data_rate, 0, False)
rx2_hz, rx2_rate = eu868.rx2()
dnmsg = station_encode(
    StationMessage(
        StationKind.DOWNLINK,
        dev_eui="70b3d57ed0001234",
        class_=0,
        diid=1,
        payload=node.encode_downlink(0, 2, b"ok"),
        rx_delay=1,
        rx1=StationWindow(rx1_rate, uplink.frequency_hz),
        rx2=StationWindow(rx2_rate, rx2_hz),
        priority=0,
        xtime=uplink.levels.xtime,
        rctx=uplink.levels.rctx,
    )
)
told = station_parse(dnmsg)
delay = told.rx_delay or 1
print(
    f"dnmsg     RX1 DR{told.rx1.data_rate} on {told.rx1.frequency_hz} Hz or "
    f"RX2 DR{told.rx2.data_rate} on {told.rx2.frequency_hz} Hz, {delay} s after the uplink"
)

# The station opens the first window a second after the uplink on its own clock, puts the
# answer on the air, and reports it by the identifier the server gave it.
uplink_at = station_xtime_parts(told.xtime)
sent_at = station_xtime(uplink_at.unit, uplink_at.session, uplink_at.micros + delay * 1_000_000)
dntxed = station_encode(
    StationMessage(
        StationKind.TRANSMITTED,
        diid=told.diid,
        dev_eui=told.dev_eui,
        rctx=told.rctx or 0,
        xtime=sent_at,
        txtime=station_xtime_parts(sent_at).micros / 1e6,
    )
)
reported = station_parse(dntxed)
went = station_xtime_parts(reported.xtime)
print(f"dntxed    downlink {reported.diid} went out at {went.micros} us of run {went.session}")
# ANCHOR_END: station

assert asked == station
assert uplink.dev_addr == 0x26010001
assert uplink.fcnt == 7
assert uplink.payload != b"21.5"
assert went.micros == 3_513_348_611


# ANCHOR: hardware
import socket as udp

from pamoja.gateway import DEFAULT_PORT, Crc


def serve_beside_the_gateway():
    """Answers what a gateway daemon on this Raspberry Pi forwards, until it goes quiet."""
    # The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
    # elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol has
    # no authentication of its own.
    site = Network(plan_for("EU868"), 0x000013)
    site.register(
        bytes.fromhex("70b3d57ed0001234"),
        bytes.fromhex("70b3d57ed0000000"),
        bytes([0x2B]) * 16,
    )

    listener = udp.socket(udp.AF_INET, udp.SOCK_DGRAM)
    try:
        listener.bind(("127.0.0.1", DEFAULT_PORT))
    except OSError:
        print(f"absent    another program holds port {DEFAULT_PORT}")
        return

    # A pamoja gateway holds its path open every five seconds, so six seconds of silence
    # means none is running. After that the server keeps answering until a minute passes
    # with nothing heard.
    listener.settimeout(6)
    downlinks = None
    token = 0
    with listener:
        while True:
            try:
                datagram, sender = listener.recvfrom(65_535)
            except TimeoutError:
                break
            listener.settimeout(60)

            try:
                packet = parse(datagram)
            except PamojaError as why:
                print(f"ignored   {why}")
                continue
            ack = acknowledgment(packet)
            if ack is not None:
                listener.sendto(encode(ack), sender)

            if packet.kind == PacketKind.PULL_DATA:
                if downlinks is None:
                    print(f"gateway   {packet.gateway} holds its downlink path open")
                downlinks = sender
            elif packet.kind == PacketKind.PUSH_DATA:
                for arrived in packet.packets:
                    if arrived.crc != Crc.OK:
                        continue
                    try:
                        event = site.uplink(arrived)
                    except PamojaError as why:
                        print(f"refused   {why}")
                        continue
                    if event.outcome == "joined":
                        print(f"joined    {event.dev_addr:#010x}")
                        transmit = event.accept
                    elif event.outcome == "data":
                        print(f"reading   {event.dev_addr:#010x} says {event.payload.decode()}")
                        transmit = site.answer(event.dev_addr, event.slot, 2, b"ok")
                    else:
                        continue
                    if downlinks is not None:
                        token = (token + 1) & 0xFFFF
                        resp = Packet(PacketKind.PULL_RESP, token, transmit=transmit)
                        listener.sendto(encode(resp), downlinks)
            elif packet.kind == PacketKind.TX_ACK:
                print(f"txack     {packet.tx_status}")

    if downlinks is None:
        print("absent    no gateway reported in, so nothing was answered")


serve_beside_the_gateway()
# ANCHOR_END: hardware
