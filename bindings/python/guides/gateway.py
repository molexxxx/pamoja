"""The LoRaWAN gateway guide example; see docs/guides/gateway.md."""

# ANCHOR: example
from pamoja.gateway import Packet, PacketKind, Rxpk, Txpk, TxStatus, acknowledgment, encode, parse
from pamoja.lora import link

# A gateway on a Raspberry Pi, whose identifier is written from its network interface.
gateway = "b827ebfffe010203"
dr5 = link(7, 125_000)

# It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and the
# concentrator's own timestamp of the reception.
heard = Rxpk(
    868_100_000,
    b"TEST_PACKET_1234",
    link=dr5,
    rssi_dbm=-35,
    snr_db=5.1,
    timestamp_us=3_512_348_611,
)
datagram = encode(Packet(PacketKind.PUSH_DATA, 0x1234, gateway=gateway, packets=[heard]))
print(f"push      {len(datagram)} bytes, token {0x1234:04x}")

# The server reads it. Nothing about the packet has to be decoded by hand: the frequency is in
# hertz, the datarate identifier is the link settings, and the payload is bytes.
received = parse(datagram).packets[0]
print(
    f"heard     {received.frequency_hz} Hz at SF{received.link.spreading_factor}, "
    f"{received.link.bandwidth_hz // 1000} kHz, {received.rssi_dbm} dBm, "
    f"SNR {received.snr_db} dB, {len(received.payload)} bytes"
)

# Every uplink is acknowledged at once, by token, before anything is processed.
ack = acknowledgment(parse(datagram))
print(f"ack       {len(encode(ack))} bytes")

# Later the server sends one back, at the concentrator timestamp that hits the device's receive
# window, with the inverted polarity a LoRaWAN device listens for.
downlink = encode(
    Packet(
        PacketKind.PULL_RESP,
        0x00AB,
        transmit=Txpk(
            869_525_000,
            b"downlink",
            link=dr5,
            timestamp_us=3_513_348_611,
            power_dbm=27,
            invert_polarity=True,
            without_crc=True,
        ),
    )
)
transmit = parse(downlink).transmit
print(
    f"downlink  {transmit.frequency_hz} Hz at {transmit.power_dbm} dBm, "
    f"inverted IQ {transmit.invert_polarity}"
)

# The gateway answers with what became of it. A packet already scheduled in that window is
# refused rather than dropped silently.
refused = encode(
    Packet(PacketKind.TX_ACK, 0x00AB, gateway=gateway, tx_status=TxStatus.COLLISION_PACKET)
)
status = parse(refused).tx_status
print(f"txack     {status}, scheduled {status == TxStatus.NONE}")
# ANCHOR_END: example

assert received.payload == b"TEST_PACKET_1234"
assert list(encode(ack)) == [2, 0x12, 0x34, 0x01]
assert status == "COLLISION_PACKET"
# ANCHOR: network
from pamoja.gateway import Network
from pamoja.lora import plan_for
from pamoja.lorawan import device, session

# One site, on the band it operates in, admitting one device it was told about.
dev_eui = bytes([0x11]) * 8
app_eui = bytes([0x22]) * 8
app_key = bytes([0x33]) * 16
site = Network(plan_for("EU868"), 0x00002A, first_dev_addr=0x26010001)
site.register(dev_eui, app_eui, app_key)

# The gateway forwards a join request it heard. Nothing about the device is known here beyond
# the key it was provisioned with, which is what verifies the request.
joiner = device(dev_eui, app_eui, app_key)
joined = site.uplink(
    Rxpk(868_100_000, joiner.join_request(0x0102), link=dr5, timestamp_us=1_000_000)
)
print(
    f"joined    {joined.dev_addr:#010x} at {joined.accept.timestamp_us} us, "
    f"inverted IQ {joined.accept.invert_polarity}"
)

# The device reads the accept and sends a reading. The site decrypts it and says where an
# answer goes, which is the uplink window plus the delay the region recommends.
granted = joiner.accept_join(joined.accept.payload, 0x0102)
carried = site.uplink(
    Rxpk(
        868_100_000,
        granted.session().encode_uplink(0, 2, b"21.5"),
        link=dr5,
        timestamp_us=9_000_000,
    )
)
print(
    f"uplink    frame {carried.fcnt}, {len(carried.payload)} bytes, "
    f"answer at {carried.slot.timestamp_us} us on {carried.slot.frequency_hz} Hz"
)

# The answer goes out in that window, encrypted with the session the join granted.
answer = site.answer(carried.dev_addr, carried.slot, 2, b"ok")
print(f"downlink  {len(answer.payload)} bytes at {answer.timestamp_us} us")

# A gateway hears every network in range, and a frame from one this site never granted is
# reported rather than refused.
stranger = site.uplink(
    Rxpk(
        868_100_000,
        session(0x12345678, bytes([0x09]) * 16, bytes([0x08]) * 16).encode_uplink(0, 1, b"hello"),
        link=dr5,
    )
)
print(f"foreign   {stranger.dev_addr:#010x} belongs to another network")
# ANCHOR_END: network

assert carried.payload == b"21.5"
assert carried.slot.timestamp_us == 10_000_000
assert stranger.outcome == "foreign"

# ANCHOR: station
from pamoja.gateway import (
    DISCOVERY_PATH,
    STATION_PROTOCOL_VERSION,
    StationKind,
    StationLevels,
    station_discovery,
    station_heard,
    station_parse,
    station_router_parse,
)
from pamoja.lorawan import session

# The same gateway, now speaking the other protocol. It is configured with an address, and
# asks on that path for the websocket its session runs on.
station_eui = "b827ebfffe010203"
ask = station_discovery(station_eui)
print(f"ask       {DISCOVERY_PATH} {ask}")

# The server answers with where to connect, or with why it will not have this station.
answer = station_router_parse(
    '{"router":"b827:ebff:fe01:203","muxs":"::0","uri":"ws://lns.example.invalid:3001/router"}'
)
print(f"open      {answer.uri}")

# A station opens with what it is, which is how the server knows what it can do.
hello = station_parse(
    '{"msgtype":"version","station":"pamoja","firmware":"0.1.18",'
    '"package":"pamoja-gateway","model":"linux",'
    f'"protocol":{STATION_PROTOCOL_VERSION},"features":"gps"}}'
)
print(f"version   {hello.msgtype} {hello.station} {hello.firmware}")

# Now a device sends a reading, and the radio hears the frame. A station holds no key, so it
# does not read the payload: it splits the frame into the fields the protocol names and lets
# the server judge them.
active = session(0x26010001, bytes([0x44] * 16), bytes([0x55] * 16))
frame = active.encode_uplink(7, 2, b"21.5")
heard = station_heard(
    frame,
    5,
    868_100_000,
    StationLevels(rctx=0, xtime=1_000_000, rssi=-35.0, snr=5.1),
)
print(f"updf      {heard.msgtype} on {heard.frequency_hz} Hz at DR{heard.data_rate}")
print(f"heard     {heard.dev_addr:#010x} counter {heard.fcnt} on port {heard.fport}")
print(f"payload   {len(heard.payload)} bytes, still encrypted")
# ANCHOR_END: station

assert ask == '{"router":"b827:ebff:fe01:203"}'
assert answer.uri == "ws://lns.example.invalid:3001/router"
assert hello.msgtype == StationKind.VERSION
assert heard.dev_addr == 0x26010001
assert heard.fcnt == 7
assert heard.fport == 2
assert heard.payload != b"21.5"
