"""Smoke test: confirms the facade loads, the native core is reachable, and the
MQTT transport surfaces errors as exceptions (no broker required)."""

import asyncio

import pytest

import pamoja
from pamoja import MqttClient, PamojaError, Qos, version


def test_version_returns_string():
    assert isinstance(version(), str)
    assert version() == pamoja.version()


def test_qos_exposes_protocol_levels():
    assert Qos.AT_LEAST_ONCE.value == "AtLeastOnce"
    assert Qos.AT_MOST_ONCE.value == "AtMostOnce"
    assert Qos.EXACTLY_ONCE.value == "ExactlyOnce"


def test_raw_escape_hatch_exposes_the_native_contract():
    from pamoja import raw

    assert hasattr(raw, "MqttClient")
    assert raw.version() == version()


def test_connect_failure_raises_and_leaves_client_disconnected():
    async def run():
        client = MqttClient(
            client_id="smoke",
            host="127.0.0.1",
            port=47811,
            keep_alive_secs=1,
        )

        assert await client.is_connected() is False

        try:
            await client.connect()
        except PamojaError as err:
            assert "transport error" in str(err)
        else:
            raise AssertionError("connecting to a closed port should raise")

        assert await client.is_connected() is False

    asyncio.run(run())


def test_identity_signs_and_verifies_a_reading():
    from pamoja import DeviceIdentity, fingerprint, verify

    device = DeviceIdentity.from_seed(bytes([7]) * 32)
    assert len(device.public_key) == 32

    signature = device.sign("21.5")
    assert len(signature) == 64
    assert verify(device.public_key, "21.5", signature) is True
    assert verify(device.public_key, "21.6", signature) is False

    assert fingerprint(device.public_key) == device.fingerprint
    assert len(device.fingerprint) == 16
    assert all(character in "0123456789abcdef" for character in device.fingerprint)


def test_a_wrong_length_key_is_an_argument_error():
    from pamoja import DeviceIdentity, verify

    device = DeviceIdentity.from_seed(bytes(32))
    signature = device.sign(b"x")

    try:
        verify(bytes(8), b"x", signature)
    except ValueError as err:
        assert "public_key must be exactly 32 bytes" in str(err)
    else:
        raise AssertionError("a wrong-length key should raise")


def test_a_document_round_trips_through_cbor():
    from pamoja import from_cbor, to_cbor

    reading = {"id": "probe-1", "c": 21.5, "battery": 88}
    cbor = to_cbor(reading)
    assert len(cbor) < len(str(reading))
    assert from_cbor(cbor) == reading


def test_malformed_cbor_raises():
    from pamoja import PamojaError, from_cbor

    try:
        from_cbor(bytes([0xFF, 0xFF]))
    except PamojaError as err:
        assert "codec error" in str(err)
    else:
        raise AssertionError("malformed CBOR should raise")


def test_readings_pack_smaller_and_decode_to_precision():
    from pamoja import Quantizer, pack_samples, unpack_samples

    samples = [10, 11, 13, 12, 900]
    assert unpack_samples(pack_samples(samples)) == samples

    quantizer = Quantizer(100)
    readings = [20.0, 20.1, 20.2, 20.3]
    packed = quantizer.encode(readings)
    assert len(packed) < len(readings) * 4
    for got, want in zip(quantizer.decode(packed), readings):
        assert abs(got - want) < 0.05


def test_helpers_carry_a_reading_through_to_an_action():
    from pamoja import Calibration, Depletion, Smoother, Thermostat, deadband

    smoother = Smoother(0.5)
    assert smoother.value is None
    smoother.update(10.0)
    smoothed = smoother.update(20.0)
    assert 10.0 < smoothed < 20.0
    smoother.reset()
    assert smoother.value is None

    fridge = Thermostat.cooling(8.0, 1.0)
    assert fridge.update(7.0) is False
    assert fridge.update(9.5) is True
    assert fridge.is_on is True

    tank = Depletion(10.0)
    assert tank.update(100.0) is None
    assert tank.update(90.0) > 0

    probe = Calibration.two_point(0.0, 0.0, 1024.0, 100.0)
    assert abs(probe.apply(512.0) - 50.0) < 0.01

    assert deadband(0.2, 0.0, 0.5) == 0.0


def test_a_geofence_reports_the_single_crossing_fix():
    from pamoja import Boundary, Coordinate, Geofence, distance_between

    centre = Coordinate(-1.2921, 36.8219)
    away = Coordinate(-1.2930, 36.8219)

    pen = Geofence(centre, 50.0)
    assert pen.update(centre) is Boundary.INSIDE
    assert pen.update(away) is Boundary.EXITED
    assert pen.update(away) is Boundary.OUTSIDE
    assert pen.contains(away) is False
    assert distance_between(centre, away) > 50.0


def test_serial_framing_round_trips_and_survives_a_corrupt_frame():
    from pamoja import serial

    payload = bytes([0xC0, 0xDB, 0x00, 0x2A])
    assert serial.slip.decode(serial.slip.encode(payload)) == payload
    assert serial.cobs.decode(serial.cobs.encode(payload)) == payload

    decoder = serial.SlipDecoder()
    frames = decoder.feed(bytes([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]))
    assert frames == [b"ok", b"go"]
    assert decoder.discarded == 1


def test_a_modbus_request_and_the_reply_it_draws():
    from pamoja import modbus

    assert modbus.read_holding_registers(0x11, 0x006B, 3) == bytes(
        [0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]
    )

    body = bytes([0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
    reply = modbus.parse_frame(body + modbus.crc16(body).to_bytes(2, "little"))
    assert reply.exception is None
    assert reply.registers() == [0x022B, 0x0000, 0x0064]

    corrupt = bytearray(body + modbus.crc16(body).to_bytes(2, "little"))
    corrupt[2] ^= 0xFF
    with pytest.raises(PamojaError):
        modbus.parse_frame(bytes(corrupt))


def test_can_frames_and_the_j1939_identifier():
    from pamoja import can

    frame = can.frame(0x20A, bytes([0x01, 0xF4]))
    assert frame.dlc == 2
    assert frame.data == bytes([0x01, 0xF4])

    remote = can.remote_frame(0x20A, 4)
    assert remote.len == 4
    assert remote.data == b""

    with pytest.raises(PamojaError):
        can.frame(0x100, bytes(9))

    assert can.decode_j1939(0x0CF00400).pgn == 61444
    assert can.decode_j1939(0x123, False) is None


def test_on_board_bus_addressing_and_pin_logic():
    from pamoja import gpio

    assert gpio.i2c.address_frame(0x76) == bytes([0xEC])
    assert gpio.i2c.address_frame(0x76, read=True) == bytes([0xED])
    assert gpio.i2c.is_reserved(0x00) and gpio.i2c.is_general_call(0x00)
    assert not gpio.i2c.is_reserved(0x76)

    with pytest.raises(PamojaError):
        gpio.i2c.address_frame(0x80)

    clock = gpio.spi.clock_for(3)
    assert clock.cpol and clock.cpha
    assert gpio.spi.mode_for(True, False) == 2

    assert gpio.pin.level_for(gpio.Polarity.ACTIVE_LOW, True) is gpio.Level.LOW
    assert gpio.pin.is_asserted(gpio.Polarity.ACTIVE_LOW, gpio.Level.LOW)
    assert gpio.pin.triggers(gpio.Edge.RISING, gpio.Level.LOW, gpio.Level.HIGH)
    assert not gpio.pin.triggers(gpio.Edge.RISING, gpio.Level.HIGH, gpio.Level.LOW)
