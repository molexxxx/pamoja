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
from pamoja import (
    WINDOW_CAPACITY,
    Anomaly,
    Median,
    PamojaError,
    Trend,
    Window,
    actuators,
    can,
    gpio,
    lora,
    lorawan,
    mesh,
    modbus,
    routing,
    sensors,
    serial,
)

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

    # Registers above 0x7FFF, which catch a binding that reads them as signed.
    high = modbus.parse_frame(unhex(vector["highRegisterReply"]["frame"]))
    assert high.registers() == vector["highRegisterReply"]["registers"]

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


def test_sensor_vectors_match():
    vector = VECTORS["sensors"]

    bme = vector["bme280"]
    calibration = sensors.bme280.calibration(
        unhex(bme["calibrationTempPress"]), unhex(bme["calibrationHumidity"])
    )
    reading = calibration.compensate(unhex(bme["measurement"]))
    assert reading.celsius == pytest.approx(bme["celsius"], abs=1e-3)
    assert reading.pascals == bme["pascals"]
    assert reading.hectopascals == pytest.approx(bme["hectopascals"], abs=1e-2)
    assert reading.relative_humidity_percent == pytest.approx(
        bme["relativeHumidityPercent"], abs=1e-3
    )

    ds = vector["ds18b20"]
    decoded = sensors.ds18b20.parse_scratchpad(unhex(ds["scratchpad"]))
    assert decoded.raw_temperature == ds["rawTemperature"]
    assert decoded.micro_celsius == ds["microCelsius"]
    assert decoded.resolution_bits == ds["resolutionBits"]
    assert decoded.alarm_high == ds["alarmHigh"]
    assert sensors.ds18b20.crc8(unhex(ds["crcData"])) == ds["crc"]

    with pytest.raises(PamojaError):
        sensors.ds18b20.parse_scratchpad(unhex(ds["corruptScratchpad"]))
    with pytest.raises(ValueError):
        sensors.ds18b20.config_byte(ds["invalidResolution"])

    for entry in ds["resolutions"]:
        assert sensors.ds18b20.config_byte(entry["bits"]) == entry["configByte"]
        assert sensors.ds18b20.step_micro_celsius(entry["bits"]) == entry["stepMicroCelsius"]
        assert (
            sensors.ds18b20.max_conversion_micros(entry["bits"])
            == entry["maxConversionMicros"]
        )
        assert sensors.ds18b20.resolution_bits(entry["configByte"]) == entry["bits"]

    ina = vector["ina219"]
    lsb = ina["currentLsbMicroamps"]
    assert sensors.ina219.calibration(lsb, ina["shuntMilliohms"]) == ina["calibration"]
    assert (
        sensors.ina219.minimum_current_lsb_microamps(ina["maxExpectedMicroamps"])
        == ina["minimumCurrentLsbMicroamps"]
    )
    assert sensors.ina219.shunt_microvolts(ina["rawShunt"]) == ina["shuntMicrovolts"]
    assert sensors.ina219.bus_millivolts(ina["rawBus"]) == ina["busMillivolts"]
    assert sensors.ina219.current_microamps(ina["rawCurrent"], lsb) == ina["currentMicroamps"]
    assert sensors.ina219.power_microwatts(ina["rawPower"], lsb) == ina["powerMicrowatts"]

    ads = vector["ads1115"]
    reset = sensors.ads1115.config_from_bits(ads["configReset"])
    want = ads["resetConfig"]
    assert reset.start_conversion == want["startConversion"]
    assert reset.mux == want["mux"]
    assert reset.pga == want["pga"]
    assert reset.single_shot == want["singleShot"]
    assert reset.data_rate == want["dataRate"]
    assert reset.window_comparator == want["windowComparator"]
    assert reset.comparator_active_high == want["comparatorActiveHigh"]
    assert reset.comparator_latching == want["comparatorLatching"]
    assert reset.comparator_queue == want["comparatorQueue"]
    assert sensors.ads1115.config_bits(reset) == ads["configReset"]

    for entry in ads["gains"]:
        assert (
            sensors.ads1115.full_scale_microvolts(entry["pga"])
            == entry["fullScaleMicrovolts"]
        )
        assert sensors.ads1115.to_nanovolts(entry["pga"], 32767) == entry["nanovoltsAtFullScale"]
    for entry in ads["rates"]:
        assert (
            sensors.ads1115.samples_per_second(entry["dataRate"])
            == entry["samplesPerSecond"]
        )


def test_actuator_vectors_match():
    vector = VECTORS["actuators"]

    pca = vector["pca9685"]
    assert actuators.pca9685.INTERNAL_OSC_HZ == pca["internalOscHz"]
    assert actuators.pca9685.CHANNELS == pca["channels"]
    assert actuators.pca9685.COUNTS == pca["counts"]
    for entry in pca["channelRegisters"]:
        assert actuators.pca9685.channel_register(entry["channel"]) == entry["register"]
    with pytest.raises(ValueError):
        actuators.pca9685.channel_register(pca["invalidChannel"])
    assert (
        actuators.pca9685.prescale_for_frequency(pca["updateRateHz"], pca["internalOscHz"])
        == pca["prescale"]
    )

    pwm = vector["pwm"]
    assert actuators.pwm.duty(pwm["duty"]["off"]) == unhex(pwm["duty"]["bytes"])
    assert actuators.pwm.servo(
        pwm["servoCentre"]["pulseMicros"], pwm["servoCentre"]["updateRateHz"]
    ) == unhex(pwm["servoCentre"]["bytes"])
    assert actuators.pwm.full_on() == unhex(pwm["fullOn"])
    assert actuators.pwm.full_off() == unhex(pwm["fullOff"])

    motor = vector["stepper"]
    stepper = actuators.Stepper(actuators.Drive.HALF_STEP)
    cycle = [stepper.coils]
    for _ in range(motor["stepCount"]):
        cycle.append(stepper.step(actuators.Direction.FORWARD))
    assert cycle == motor["forwardCycle"]
    assert stepper.steps == motor["stepCount"]
    assert actuators.Drive.HALF_STEP.step_count == motor["stepCount"]
    assert (
        actuators.steps_for_degrees(motor["degrees"], motor["stepsPerRevolution"])
        == motor["stepsForDegrees"]
    )


def test_windowed_helper_vectors_match():
    vector = VECTORS["windows"]
    assert WINDOW_CAPACITY == vector["capacity"]

    window = Window()
    for reading, want in zip(vector["window"]["readings"], vector["window"]["states"]):
        window.push(reading)
        assert len(window) == want["len"]
        assert window.mean() == pytest.approx(want["mean"], abs=TOLERANCE)
        assert window.min() == pytest.approx(want["min"], abs=TOLERANCE)
        assert window.max() == pytest.approx(want["max"], abs=TOLERANCE)
        assert window.range() == pytest.approx(want["range"], abs=TOLERANCE)

    median = Median()
    for reading, want in zip(vector["median"]["readings"], vector["median"]["outputs"]):
        assert median.update(reading) == pytest.approx(want, abs=TOLERANCE)

    trend = Trend()
    for reading, want in zip(vector["trend"]["readings"], vector["trend"]["slopes"]):
        trend.push(reading)
        if want is None:
            assert trend.slope is None
        else:
            assert trend.slope == pytest.approx(want, abs=1e-4)

    anomaly = Anomaly(vector["anomaly"]["sigmas"])
    for reading, want in zip(vector["anomaly"]["readings"], vector["anomaly"]["flags"]):
        assert anomaly.check(reading) == want


def _link_of(described: dict) -> lora.LoraLink:
    """Rebuild the link a vector describes."""
    return lora.link(
        described["spreadingFactor"],
        described["bandwidthHz"],
        described["codingRateDenominator"],
        described["preambleSymbols"],
        described["explicitHeader"],
        described["crc"],
    )


def test_lora_vectors_match():
    vector = VECTORS["lora"]

    for described in vector["links"]:
        link = _link_of(described)
        assert link.symbol_time_us() == described["symbolTimeUs"], described["name"]

        for airtime in described["airtimes"]:
            assert link.airtime_us(airtime["payloadLen"]) == airtime["airtimeUs"], (
                f"airtime of {airtime['payloadLen']} bytes on {described['name']}"
            )

        for budget in described["budgets"]:
            assert (
                link.min_off_time_us(budget["payloadLen"], budget["permille"])
                == budget["offTimeUs"]
            ), f"off time at {budget['permille']} permille on {described['name']}"

    for clamp in vector["clamped"]:
        assert lora.link(clamp["asked"], 125_000).spreading_factor == clamp["used"]

    # Rust saturates the off time when transmitting is forbidden; the facade
    # reports None instead, so a caller cannot mistake it for a real wait.
    forbidden = vector["forbidden"]
    link = _link_of(
        next(entry for entry in vector["links"] if entry["name"] == forbidden["link"])
    )
    assert (
        link.min_off_time_us(forbidden["payloadLen"], forbidden["permille"]) is None
    )
    assert lora.messages_per_hour(link, forbidden["payloadLen"], forbidden["permille"]) == 0


def test_mesh_vectors_match():
    vector = VECTORS["mesh"]

    assert mesh.MAX_FRAME == vector["maxFrame"]
    assert mesh.MAX_PAYLOAD == vector["maxPayload"]
    assert mesh.BROADCAST == vector["broadcastAddress"]
    assert mesh.SEEN_CAPACITY == vector["seenCapacity"]

    unicast = vector["unicast"]
    built = mesh.frame(
        unicast["src"],
        unicast["dst"],
        unicast["id"],
        unhex(unicast["payload"]),
        unicast["hopLimit"],
    )
    assert built.bytes.hex() == unicast["bytes"]

    broadcast = vector["broadcast"]
    built = mesh.broadcast(broadcast["src"], broadcast["id"], unhex(broadcast["payload"]))
    assert built.bytes.hex() == broadcast["bytes"]

    parsed = mesh.parse(unhex(broadcast["bytes"]))
    assert parsed.broadcast
    assert parsed.payload.hex() == broadcast["payload"]

    relayed = mesh.relayed(unhex(broadcast["bytes"]))
    assert relayed.bytes.hex() == vector["relayed"]["bytes"]
    assert relayed.hop_limit == vector["relayed"]["hopLimit"]

    assert mesh.relayed(unhex(vector["exhausted"])) is None

    with pytest.raises(PamojaError):
        mesh.parse(unhex(vector["corrupt"]))

    crc = vector["crc"]
    assert mesh.crc16(unhex(crc["check"])) == crc["checkValue"]
    assert mesh.crc16(unhex(crc["data"])) == crc["value"]

    seen = mesh.SeenPackets()
    for (src, packet_id), expected in zip(vector["seen"]["keys"], vector["seen"]["new"]):
        assert seen.record(src, packet_id) == expected


def _assert_decision(router, want: dict) -> None:
    """Check one routing decision against the vector that describes it."""
    decision = router.forward(want["dst"])
    assert decision.action == want["action"], f"packet for {want['dst']}"
    assert decision.next_hop == want["nextHop"], f"next hop for {want['dst']}"


def test_routing_vectors_match():
    vector = VECTORS["routing"]
    router = routing.router(vector["address"])

    assert router.capacity == vector["capacity"]
    assert routing.TABLE_CAPACITY == vector["capacity"]

    for observation in vector["observations"]:
        assert (
            router.observe(observation["origin"], observation["via"], observation["cost"])
            == observation["changed"]
        ), f"observing {observation['origin']} via {observation['via']}"

    assert len(router) == vector["learned"]

    route = router.route(vector["route"]["dst"])
    assert route.next_hop == vector["route"]["nextHop"]
    assert route.cost == vector["route"]["cost"]

    for want in vector["decisions"]:
        _assert_decision(router, want)

    router.forget(vector["afterForgetting"]["dst"])
    _assert_decision(router, vector["afterForgetting"]["decision"])
    assert len(router) == vector["afterForgetting"]["learned"]


def test_lorawan_vectors_match():
    vector = VECTORS["lorawan"]
    session = lorawan.session(
        vector["devAddr"], unhex(vector["nwkSKey"]), unhex(vector["appSKey"])
    )
    assert session.dev_addr == vector["devAddr"]

    up = vector["uplink"]
    uplink = session.encode_uplink(
        up["fcnt"],
        up["fport"],
        unhex(up["payload"]),
        confirmed=up["confirmed"],
        adr=up["adr"],
        ack=up["ack"],
    )
    assert uplink.hex() == up["frame"]

    rx = session.decode(uplink, up["fcnt"])
    assert rx.direction == lorawan.Direction.UPLINK
    assert rx.confirmed == up["confirmed"]
    assert rx.adr == up["adr"]
    assert rx.ack == up["ack"]
    assert rx.payload.hex() == up["payload"]

    down = vector["downlink"]
    downlink = session.encode_downlink(
        down["fcnt"],
        down["fport"],
        unhex(down["payload"]),
        ack=down["ack"],
        fpending=down["fpending"],
        fopts=unhex(down["fopts"]),
    )
    assert downlink.hex() == down["frame"]

    received = session.decode(downlink, down["fcnt"])
    assert received.direction == lorawan.Direction.DOWNLINK
    assert received.fpending == down["fpending"]
    assert received.fopts.hex() == down["fopts"]

    with pytest.raises(PamojaError):
        session.decode(unhex(vector["forgedUplink"]), up["fcnt"])
    with pytest.raises(PamojaError):
        session.decode(uplink, vector["wrongCounter"])

    join = vector["join"]
    device = lorawan.device(
        unhex(join["devEui"]), unhex(join["appEui"]), unhex(join["appKey"])
    )
    assert device.join_request(join["devNonce"]).hex() == join["request"]
    with pytest.raises(PamojaError):
        device.accept_join(unhex(join["forgedAccept"]), join["devNonce"])
