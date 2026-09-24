# pamoja-gateway

What a LoRaWAN gateway speaks: the Semtech packet forwarder protocol and the Basics Station protocol on both sides, the network side of a single site, and a bridge from the radio to the link that leaves it. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/gateway.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html)

## Install

```sh
pip install pamoja-gateway
```

```python
from pamoja import gateway
```

This pulls in `pamoja-native`, the compiled engine, and `pamoja-lora`. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/gateway.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gateway.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-gateway`](https://crates.io/crates/pamoja-gateway) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gateway/index.html), [docs.rs](https://docs.rs/pamoja-gateway), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gateway) |
| TypeScript | [`@pamoja/gateway`](https://www.npmjs.com/package/@pamoja/gateway) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gateway.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gateway) |
| Python | [`pamoja-gateway`](https://pypi.org/project/pamoja-gateway/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gateway) |
| C# | [`Pamoja.Gateway`](https://www.nuget.org/packages/Pamoja.Gateway) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gateway.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gateway) |

## Documentation

- [`pamoja.gateway` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/gateway.html), every class and function in this module.
- [The LoRaWAN gateways guide](https://pamoja.molex.cloud/docs/guides/gateway.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
