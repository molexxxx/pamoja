# pamoja-gateway

The Semtech UDP packet forwarder protocol on both sides: the datagrams a gateway and a network server exchange, and the packets, reports and downlinks they carry. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
