"""The Python side of the cross-language conformance suite.

Runs the same vectors from ``conformance/vectors.json`` that every other binding
runs, so a facade that drifts here fails rather than quietly disagreeing with
Rust, Node, and .NET.
"""

import json
import pathlib

import pytest

from pamoja import (
    Calibration,
    Coordinate,
    Depletion,
    DeviceIdentity,
    Geofence,
    Pid,
    Quantizer,
    Smoother,
    Thermostat,
    deadband,
    from_cbor,
    pack_samples,
    to_cbor,
    unpack_samples,
    verify,
)
from pamoja import PamojaError, can, gpio, modbus, serial

VECTORS = json.loads(
    (pathlib.Path(__file__).resolve().parents[3] / "conformance" / "vectors.json").read_text(
        encoding="utf-8"
    )
)

# The vectors carry f32 values widened to f64, so they compare exactly; the
# tolerance covers the accumulation order of the iterative helpers.
TOLERANCE = VECTORS["tolerance"]


def unhex(text: str) -> bytes:
    """Decode a lowercase hex string from the vectors."""
    return bytes.fromhex(text)


def test_identity_vectors_match():
    vector = VECTORS["identity"]
    device = DeviceIdentity.from_seed(bytes.fromhex(vector["seed"]))

    assert device.public_key == bytes.fromhex(vector["publicKey"])
    assert device.fingerprint == vector["fingerprint"]
    assert device.sign(vector["payload"]) == bytes.fromhex(vector["signature"])

    public_key = bytes.fromhex(vector["publicKey"])
    signature = bytes.fromhex(vector["signature"])
    assert verify(public_key, vector["payload"], signature) is True
    assert verify(public_key, vector["tamperedPayload"], signature) is False


def test_codec_vectors_match():
    vector = VECTORS["codec"]
    cbor = bytes.fromhex(vector["cbor"])

    assert to_cbor(vector["json"].encode("utf-8")) == cbor
    assert from_cbor(cbor) == json.loads(vector["json"])
    # Keys are sorted on the way through, so the encoding is canonical.
    assert to_cbor(vector["unsortedJson"].encode("utf-8")) == cbor

    deltas = vector["deltas"]
    assert pack_samples(deltas["samples"]) == bytes.fromhex(deltas["packed"])
    assert unpack_samples(bytes.fromhex(deltas["packed"])) == deltas["samples"]

    quantizer_vector = vector["quantizer"]
    quantizer = Quantizer(quantizer_vector["scale"])
    packed = bytes.fromhex(quantizer_vector["packed"])
    assert quantizer.encode(quantizer_vector["readings"]) == packed
    for got, want in zip(quantizer.decode(packed), quantizer_vector["readings"]):
        assert got == pytest.approx(want, abs=quantizer_vector["tolerance"])


def test_smoother_vectors_match():
    vector = VECTORS["smoother"]
    smoother = Smoother(vector["weight"])
    for sample, want in zip(vector["samples"], vector["outputs"]):
        assert smoother.update(sample) == pytest.approx(want, abs=TOLERANCE)


def test_pid_vectors_match():
    vector = VECTORS["pid"]
    controller = Pid(vector["kp"], vector["ki"], vector["kd"])
    for measurement, want in zip(vector["measurements"], vector["outputs"]):
        got = controller.update(vector["setpoint"], measurement, vector["dt"])
        assert got == pytest.approx(want, abs=TOLERANCE)


def test_thermostat_vectors_match():
    vector = VECTORS["thermostat"]
    thermostat = Thermostat.cooling(vector["setpoint"], vector["hysteresis"])
    for reading, want in zip(vector["readings"], vector["outputs"]):
        assert thermostat.update(reading) is want


def test_depletion_vectors_match():
    vector = VECTORS["depletion"]
    depletion = Depletion(vector["threshold"])
    for level, want in zip(vector["levels"], vector["outputs"]):
        assert depletion.update(level) == want


def test_calibration_and_deadband_vectors_match():
    vector = VECTORS["calibration"]
    calibration = Calibration.two_point(
        vector["rawLow"], vector["valueLow"], vector["rawHigh"], vector["valueHigh"]
    )
    for raw, want in zip(vector["inputs"], vector["outputs"]):
        assert calibration.apply(raw) == pytest.approx(want, abs=TOLERANCE)

    vector = VECTORS["deadband"]
    for value, want in zip(vector["inputs"], vector["outputs"]):
        got = deadband(value, vector["center"], vector["width"])
        assert got == pytest.approx(want, abs=TOLERANCE)


def test_geofence_vectors_match():
    vector = VECTORS["geofence"]
    fence = Geofence(
        Coordinate(vector["center"]["latitude"], vector["center"]["longitude"]),
        vector["radiusM"],
    )
    for fix, want in zip(vector["fixes"], vector["boundaries"]):
        assert fence.update(Coordinate(fix["latitude"], fix["longitude"])).value == want


def test_serial_vectors_match():
    vector = VECTORS["serial"]
    payload = unhex(vector["payload"])

    assert serial.slip.encode(payload) == unhex(vector["slipFrame"])
    assert serial.slip.decode(unhex(vector["slipFrame"])) == payload
    assert serial.cobs.encode(payload) == unhex(vector["cobsFrame"])
    assert serial.cobs.decode(unhex(vector["cobsFrame"])) == payload

    assert serial.slip.max_encoded_len(len(payload)) == vector["slipMaxEncodedLen"]
    assert serial.cobs.max_encoded_len(len(payload)) == vector["cobsMaxEncodedLen"]

    with pytest.raises(PamojaError):
        serial.slip.decode(unhex(vector["corruptSlipFrame"]))

    stream = vector["slipStream"]
    decoder = serial.SlipDecoder()
    data = unhex(stream["bytes"])
    frames = []
    for at in range(0, len(data), stream["chunk"]):
        frames.extend(decoder.feed(data[at : at + stream["chunk"]]))
    assert frames == [unhex(frame) for frame in stream["frames"]]
    assert decoder.discarded == stream["discarded"]

    cobs_stream = vector["cobsStream"]
    cobs_decoder = serial.CobsDecoder()
    cobs_data = unhex(cobs_stream["bytes"])
    cobs_frames = []
    for at in range(0, len(cobs_data), cobs_stream["chunk"]):
        cobs_frames.extend(cobs_decoder.feed(cobs_data[at : at + cobs_stream["chunk"]]))
    assert cobs_frames == [unhex(frame) for frame in cobs_stream["frames"]]


def test_modbus_vectors_match():
    vector = VECTORS["modbus"]

    read = vector["readHoldingRegisters"]
    assert modbus.read_holding_registers(
        read["address"], read["start"], read["count"]
    ) == unhex(read["frame"])

    coils_request = vector["readCoils"]
    assert modbus.read_coils(
        coils_request["address"], coils_request["start"], coils_request["count"]
    ) == unhex(coils_request["frame"])

    single = vector["writeSingleRegister"]
    assert modbus.write_single_register(
        single["address"], single["register"], single["value"]
    ) == unhex(single["frame"])

    many = vector["writeMultipleRegisters"]
    assert modbus.write_multiple_registers(
        many["address"], many["start"], many["values"]
    ) == unhex(many["frame"])

    bits = vector["writeMultipleCoils"]
    assert modbus.write_multiple_coils(
        bits["address"], bits["start"], bits["values"]
    ) == unhex(bits["frame"])

    assert modbus.crc16(unhex(vector["crc"]["data"])) == vector["crc"]["value"]

    reply = modbus.parse_frame(unhex(vector["reply"]["frame"]))
    assert reply.address == vector["reply"]["address"]
    assert reply.function_code == vector["reply"]["functionCode"]
    assert reply.exception is None
    assert reply.pdu == unhex(vector["reply"]["pdu"])
    assert reply.registers() == vector["reply"]["registers"]

    bit_reply = modbus.parse_frame(unhex(vector["bitReply"]["frame"]))
    assert bit_reply.coils(vector["bitReply"]["count"]) == vector["bitReply"]["coils"]

    refused = modbus.parse_frame(unhex(vector["exceptionReply"]["frame"]))
    assert refused.exception == vector["exceptionReply"]["exception"]
    assert refused.function_code == vector["exceptionReply"]["functionCode"]

    with pytest.raises(PamojaError):
        modbus.parse_frame(unhex(vector["corruptFrame"]))


def test_can_vectors_match():
    vector = VECTORS["can"]

    classic = can.frame(
        vector["classic"]["id"], unhex(vector["classic"]["data"]), vector["classic"]["extended"]
    )
    assert classic.dlc == vector["classic"]["dlc"]
    assert classic.data == unhex(vector["classic"]["data"])

    wide = can.fd_frame(vector["fd"]["id"], unhex(vector["fd"]["data"]), vector["fd"]["extended"])
    assert wide.dlc == vector["fd"]["dlc"]
    assert wide.fd and wide.extended

    remote = can.remote_frame(
        vector["remote"]["id"], vector["remote"]["requested"], vector["remote"]["extended"]
    )
    assert remote.len == vector["remote"]["len"]
    assert len(remote.data) == vector["remote"]["dataLen"]

    with pytest.raises(PamojaError):
        can.frame(0x100, bytes(vector["tooLongForClassic"]))
    with pytest.raises(PamojaError):
        can.fd_frame(0x100, bytes(vector["invalidFdLength"]))

    for entry in vector["lengths"]:
        assert can.len_to_dlc(entry["len"]) == entry["dlc"]
    for entry in vector["codes"]:
        assert can.dlc_to_len(entry["dlc"]) == entry["len"]

    for entry in vector["j1939"]:
        message = can.decode_j1939(entry["id"])
        assert message.pgn == entry["pgn"]
        assert message.priority == entry["priority"]
        assert message.source == entry["source"]
        assert message.destination == entry["destination"]
        assert message.broadcast == entry["broadcast"]
        assert (
            can.compose_j1939(
                entry["priority"], entry["pgn"], entry["source"], entry["destination"] or 0
            )
            == entry["id"]
        )

    assert can.decode_j1939(vector["standardIsNotJ1939"], False) is None


def test_gpio_vectors_match():
    vector = VECTORS["gpio"]

    for entry in vector["i2c"]:
        address, ten_bit = entry["address"], entry["tenBit"]
        assert gpio.i2c.address_frame(address, ten_bit=ten_bit) == unhex(entry["writeFrame"])
        assert gpio.i2c.address_frame(address, read=True, ten_bit=ten_bit) == unhex(
            entry["readFrame"]
        )
        assert gpio.i2c.frame_len(address, ten_bit) == entry["frameLen"]
        assert gpio.i2c.is_reserved(address, ten_bit) == entry["reserved"]
        assert gpio.i2c.is_general_call(address, ten_bit) == entry["generalCall"]

    with pytest.raises(PamojaError):
        gpio.i2c.address_frame(vector["outOfRangeSevenBit"])
    with pytest.raises(PamojaError):
        gpio.i2c.address_frame(vector["outOfRangeTenBit"], ten_bit=True)

    for entry in vector["spi"]:
        clock = gpio.spi.clock_for(entry["mode"])
        assert clock.cpol == entry["cpol"]
        assert clock.cpha == entry["cpha"]
        assert gpio.spi.mode_for(entry["cpol"], entry["cpha"]) == entry["mode"]

    with pytest.raises(ValueError):
        gpio.spi.clock_for(vector["invalidSpiMode"])

    for entry in vector["edges"]:
        got = gpio.pin.triggers(
            gpio.Edge(entry["edge"]), gpio.Level(entry["from"]), gpio.Level(entry["to"])
        )
        assert got == entry["triggered"]

    for entry in vector["polarities"]:
        polarity = gpio.Polarity(entry["polarity"])
        level = gpio.pin.level_for(polarity, entry["asserted"])
        assert level.value == entry["level"]
        assert gpio.pin.is_asserted(polarity, level) == entry["isAsserted"]
