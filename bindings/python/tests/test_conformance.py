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
    audit,
    can,
    gpio,
    lora,
    lorawan,
    mesh,
    modbus,
    power,
    profile,
    ros2,
    routing,
    sensors,
    serial,
    session,
    telemetry,
    update,
    zenoh,
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


def _data_rate_of(rate) -> dict | None:
    """Describe a data rate the way the vectors do."""
    if rate is None:
        return None
    return {
        "kind": rate.kind,
        "bitrateBps": rate.bitrate_bps,
        "bandwidthHz": rate.bandwidth_hz,
        "spreadingFactor": rate.spreading_factor,
        "codingRateNumerator": rate.coding_rate_numerator,
        "codingRateDenominator": rate.coding_rate_denominator,
    }


def _payload_of(payload) -> dict | None:
    """Describe a payload limit the way the vectors do."""
    if payload is None:
        return None
    return {"macPayload": payload.mac_payload, "application": payload.application}


def _check_plan(plan, want: dict) -> None:
    """Hold one channel plan to the answers every binding must give."""
    where = want["name"]
    info = plan.info()
    assert plan.name == want["name"], where
    assert info.uplink_data_rate_count == want["uplinkDataRateCount"], where
    assert info.downlink_data_rate_count == want["downlinkDataRateCount"], where
    assert info.default_channel_count == want["defaultChannelCount"], where
    assert info.max_rx1_data_rate_offset == want["maxRx1DataRateOffset"], where
    assert info.has_dwell_time_limit == want["hasDwellTimeLimit"], where
    assert plan.rx2() == (want["rx2"]["frequencyHz"], want["rx2"]["dataRate"]), where

    fastest = want["uplinkDataRateCount"] - 1
    assert _data_rate_of(plan.data_rate(0)) == want["slowestUplink"], where
    assert _data_rate_of(plan.data_rate(fastest)) == want["fastestUplink"], where
    assert _data_rate_of(plan.data_rate(0, "downlink")) == want["slowestDownlink"], where

    assert (
        _payload_of(plan.max_payload(0, "uplink_repeater"))
        == want["payloadAtSlowest"]["repeater"]
    ), where
    assert (
        _payload_of(plan.max_payload(0, "uplink_direct"))
        == want["payloadAtSlowest"]["direct"]
    ), where
    assert (
        _payload_of(plan.max_payload(0, "dwell_limited")) == want["dwellLimitedAtSlowest"]
    ), where

    probe = want["probeFrequencyHz"]
    assert plan.duty_cycle_permille(probe) == want["dutyCyclePermilleAtProbe"], where
    assert plan.max_eirp_dbm(probe) == want["maxEirpDbmAtProbe"], where

    for offset, entry in enumerate(want["rx1RowForSlowest"]):
        assert plan.rx1_data_rate(0, offset) == entry, f"{where} RX1 offset {offset}"

    assert plan.next_backoff_data_rate(fastest) == want["backoffFromFastest"], where
    assert plan.next_backoff_data_rate(0) == want["backoffFromSlowest"], where

    for channel, frequency in enumerate(want["channelFrequencies"]):
        assert plan.channel_frequency_hz(channel) == frequency, f"{where} channel {channel}"

    bands = plan.sub_bands()
    assert len(bands) == len(want["subBands"]), where
    for band, entry in zip(bands, want["subBands"]):
        assert band.start_hz == entry["startHz"], where
        assert band.end_hz == entry["endHz"], where
        assert band.duty_cycle_permille == entry["dutyCyclePermille"], where
        assert band.max_eirp_dbm == entry["maxEirpDbm"], where


def test_lora_region_vectors_match():
    vector = VECTORS["loraRegions"]

    codes = lora.ChannelPlan.regions()
    assert [want["code"] for want in vector["published"]] == codes, (
        "every published region is described, in order"
    )
    for want in vector["published"]:
        _check_plan(lora.plan_for(want["code"]), want)


def test_a_private_plan_matches_the_same_vectors():
    # A deployment on licensed spectrum, assembled rather than published, must
    # answer what a named band does.
    builder = lora.ChannelPlanBuilder("private-915")
    builder.data_rate(lora.LoraDataRate.lora(12, 125_000, 250))
    builder.data_rate(lora.LoraDataRate.lora(7, 125_000, 5_470))
    for table in ("uplink_repeater", "uplink_direct"):
        builder.max_payload(lora.LoraMaxPayload(59, 51), table)
        builder.max_payload(lora.LoraMaxPayload(230, 222), table)
    builder.channel_block(lora.LoraChannelBlock(915_000_000, 500_000, 4, 0, 1))
    builder.sub_band(lora.LoraSubBand(915_000_000, 917_000_000, 1000, 30))
    builder.power(30, 2, 7)
    builder.rx(915_000_000, 0, 0)
    builder.rx1_row([0])
    builder.rx1_row([1])

    _check_plan(builder.build(), VECTORS["loraRegions"]["custom"])


def test_mavlink_vectors_match():
    """The bytes a sender puts on the wire are pinned, not merely round-tripped."""
    from pamoja import mavlink

    vector = VECTORS["mavlink"]

    for entry in vector["crc16"]:
        assert mavlink.crc16(bytes.fromhex(entry["input"])) == entry["checksum"]

    for entry in vector["knownCrcExtra"]:
        assert mavlink.known_crc_extra(entry["msgid"]) == entry["crcExtra"], entry["msgid"]
    assert mavlink.known_crc_extra(vector["unknownCrcExtra"]) is None

    # A seed derived from a definition must equal the published one, which is
    # what makes a dialect this build has never seen usable.
    for described in vector["derivedCrcExtra"]:
        fields = [
            (field["type"], field["name"], field["arrayLen"]) for field in described["fields"]
        ]
        assert mavlink.message_crc_extra(described["name"], fields) == described["crcExtra"], (
            described["name"]
        )

    header = mavlink.MavlinkHeader(
        vector["header"]["systemId"],
        vector["header"]["componentId"],
        vector["header"]["sequence"],
    )
    payload = bytes.fromhex(vector["payload"])

    for described in vector["frames"]:
        want = bytes.fromhex(described["bytes"])
        if described["version"] == 1:
            built = mavlink.MavlinkFrame.encode_v1(
                header, described["msgid"], payload, described["crcExtra"]
            )
        elif described["msgid"] == 50_000:
            built = mavlink.MavlinkFrame.encode_v2(
                mavlink.MavlinkHeader(9, 1, 0),
                described["msgid"],
                (42).to_bytes(4, "little"),
                described["crcExtra"],
            )
        else:
            built = mavlink.MavlinkFrame.encode_v2(
                header, described["msgid"], payload, described["crcExtra"]
            )
        assert built.bytes == want, described["name"]

        parsed = mavlink.MavlinkFrame.parse(want, described["crcExtra"])
        assert parsed.message_id == described["msgid"], described["name"]
        assert parsed.payload.hex() == described["payload"], described["name"]
        assert parsed.signed == described["signed"], described["name"]
        assert parsed.incompat_flags == described["incompatFlags"], described["name"]

        # A parser fed the same bytes must find the same frame.
        dialect = mavlink.Dialect()
        dialect.add(described["msgid"], described["crcExtra"])
        found = mavlink.MavlinkParser().push(want, dialect)
        assert len(found) == 1, described["name"]
        assert found[0].bytes == want, described["name"]

    # Signing is deterministic given the key, link and timestamp.
    signed = vector["signed"]
    key = bytes.fromhex(signed["key"])
    signer = mavlink.MavlinkSigner(key, signed["linkId"], signed["timestamp"])
    frame = signer.sign(header, signed["msgid"], payload, signed["crcExtra"])
    assert frame.bytes.hex() == signed["bytes"]
    assert frame.signature.hex() == signed["signature"]

    verifier = mavlink.MavlinkVerifier(key)
    verifier.verify(frame)
    with pytest.raises(ValueError):
        verifier.verify(frame)

    for entry in vector["timestamps"]:
        assert (
            mavlink.timestamp_from_unix_micros(entry["unixMicros"]) == entry["timestamp"]
        ), entry["unixMicros"]

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
    assert mesh.SEEN_DEFAULT_CAPACITY == vector["seenCapacity"]

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

    seen = mesh.SeenPackets(vector["seenCapacity"])
    for (src, packet_id), expected in zip(vector["seen"]["keys"], vector["seen"]["new"]):
        assert seen.record(src, packet_id) == expected

    sized = vector["sizedSeen"]
    small = mesh.SeenPackets(sized["capacity"])
    assert small.capacity == sized["capacity"]
    for src, packet_id in sized["keys"]:
        small.record(src, packet_id)
    assert not small.contains(*sized["evicted"])


def _assert_decision(router, want: dict) -> None:
    """Check one routing decision against the vector that describes it."""
    decision = router.forward(want["dst"])
    assert decision.action == want["action"], f"packet for {want['dst']}"
    assert decision.next_hop == want["nextHop"], f"next hop for {want['dst']}"


def test_routing_vectors_match():
    vector = VECTORS["routing"]
    router = routing.router(vector["address"], vector["capacity"])

    assert router.capacity == vector["capacity"]
    assert routing.DEFAULT_CAPACITY == vector["capacity"]

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

    sized = vector["sized"]
    small = routing.router(0x01, sized["capacity"])
    assert small.capacity == sized["capacity"]
    for node in range(sized["offered"]):
        small.observe(node + 0x100, 0x05, 4)
    assert len(small) == sized["learned"]


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


def _assert_grant(vector: dict, app_key: bytes, dev_nonce: int) -> None:
    """Check a grant builds its accept and derives the session both sides share."""
    cflist = vector.get("cflist")
    granted = lorawan.grant(
        vector["appNonce"],
        vector["netId"],
        vector["devAddr"],
        vector["dlSettings"],
        vector["rxDelay"],
        None if cflist is None else unhex(cflist),
    )
    assert granted.accept(app_key, dev_nonce).hex() == vector["accept"]

    # Neither side sent a key, so the proof they agree is that one reads what the
    # other wrote.
    probe = vector["probe"]
    session = granted.session(app_key, dev_nonce)
    assert (
        session.encode_uplink(
            probe["fcnt"], probe["fport"], unhex(probe["payload"])
        ).hex()
        == probe["frame"]
    )


def test_header_vectors_match():
    vector = VECTORS["header"]

    for want in vector["frames"]:
        header = lorawan.parse_header(unhex(want["frame"]))
        assert header.message_type == want["messageType"]
        assert header.is_data == want["isData"]
        assert header.dev_addr == want["devAddr"]
        assert header.fcnt == want["fcnt"]
        assert header.fport == want["fport"]
        assert header.confirmed == want["confirmed"]
        assert header.adr == want["adr"]
        assert header.ack == want["ack"]
        assert header.fpending == want["fpending"]
        assert header.fopts_len == want["foptsLen"]
        assert header.payload_len == want["payloadLen"]

    with pytest.raises(PamojaError):
        lorawan.parse_header(unhex(vector["unsupported"]))
    with pytest.raises(PamojaError):
        lorawan.parse_header(unhex(vector["truncated"]))


def test_network_vectors_match():
    vector = VECTORS["network"]
    app_key = unhex(vector["appKey"])

    request = lorawan.parse_join_request(unhex(vector["joinRequest"]["frame"]), app_key)
    assert request.dev_eui.hex() == vector["joinRequest"]["devEui"]
    assert request.app_eui.hex() == vector["joinRequest"]["appEui"]
    assert request.dev_nonce == vector["joinRequest"]["devNonce"]

    with pytest.raises(PamojaError):
        lorawan.parse_join_request(unhex(vector["forgedRequest"]), app_key)

    _assert_grant(vector["grant"], app_key, vector["devNonce"])

    # The captured join: a third party's numbers, so agreement here is not just
    # this implementation agreeing with itself.
    published = vector["published"]
    published_key = unhex(published["appKey"])
    _assert_grant(published, published_key, published["devNonce"])

    device = lorawan.device(bytes(8), bytes(8), published_key)
    accepted = device.accept_join(unhex(published["accept"]), published["devNonce"])
    assert accepted.dev_addr == published["devAddr"]

    probe = published["probe"]
    assert (
        accepted.session()
        .encode_uplink(probe["fcnt"], probe["fport"], unhex(probe["payload"]))
        .hex()
        == probe["frame"]
    )


def test_audit_vectors_match():
    vector = VECTORS["audit"]
    keeper = DeviceIdentity.from_seed(unhex(vector["seed"]))
    assert keeper.public_key == unhex(vector["publicKey"])

    log = audit.AuditLog(keeper)
    entries = []
    for want in vector["entries"]:
        entry = log.append(want["payload"].encode())
        assert entry.index == want["index"]
        assert entry.previous == unhex(want["previous"])
        assert entry.digest == unhex(want["digest"])
        assert entry.signature == unhex(want["signature"])
        assert entry.to_bytes() == unhex(want["bytes"])
        entries.append(entry)

    assert audit.verify_chain(keeper.public_key, entries)
    assert not audit.verify_chain(
        keeper.public_key,
        [entries[0], entries[1], audit.AuditEntry.from_bytes(unhex(vector["tampered"]))],
    )

    resumed = audit.AuditLog.resume(keeper, entries[2])
    after_reboot = resumed.append(vector["resumed"]["payload"].encode())
    assert after_reboot.index == vector["resumed"]["index"]
    assert after_reboot.to_bytes() == unhex(vector["resumed"]["bytes"])


def test_session_vectors_match():
    vector = VECTORS["session"]
    node = session.AgreementKey(unhex(vector["nodeSeed"]))
    gateway = session.AgreementKey(unhex(vector["gatewaySeed"]))

    assert node.public_key == unhex(vector["nodePublicKey"])
    assert gateway.public_key == unhex(vector["gatewayPublicKey"])

    salt = unhex(vector["salt"])
    aad = vector["aad"].encode()
    uplink = session.Session(node, gateway.public_key, salt, session.Role.INITIATOR)
    downlink = session.Session(gateway, node.public_key, salt, session.Role.RESPONDER)

    for want in vector["messages"]:
        message = uplink.seal(want["plaintext"].encode(), aad)
        assert message.counter == want["counter"]
        assert message.tag == unhex(want["tag"])
        assert message.ciphertext == unhex(want["ciphertext"])
        assert downlink.open(message, aad) == want["plaintext"].encode()

    first = vector["messages"][0]
    replayed = session.SealedMessage(
        first["counter"], unhex(first["tag"]), unhex(first["ciphertext"])
    )
    with pytest.raises(PamojaError):
        downlink.open(replayed, aad)

    fresh = session.Session(gateway, node.public_key, salt, session.Role.RESPONDER)
    with pytest.raises(PamojaError):
        fresh.open(replayed, vector["wrongAad"].encode())

    assert session.hmac_sha256(
        vector["hmac"]["key"].encode(), vector["hmac"]["message"].encode()
    ) == unhex(vector["hmac"]["digest"])
    assert session.hkdf_sha256(
        vector["hkdf"]["salt"].encode(),
        vector["hkdf"]["ikm"].encode(),
        vector["hkdf"]["info"].encode(),
        vector["hkdf"]["length"],
    ) == unhex(vector["hkdf"]["output"])


def test_update_vectors_match():
    vector = VECTORS["update"]
    publisher = DeviceIdentity.from_seed(unhex(vector["publisherSeed"]))
    assert publisher.public_key == unhex(vector["publisherPublicKey"])

    manifest = update.Manifest(
        sequence=vector["manifest"]["sequence"],
        vendor_id=unhex(vector["vendorId"]),
        class_id=unhex(vector["classId"]),
        storage=vector["manifest"]["storage"],
        digest=unhex(vector["manifest"]["digest"]),
        size=vector["manifest"]["size"],
        expires=vector["manifest"]["expires"],
        format=vector["manifest"]["format"],
        structure_version=vector["manifest"]["structureVersion"],
    )
    image = bytes([vector["imageByte"]]) * vector["imageLen"]

    assert update.encode_manifest(manifest) == unhex(vector["body"])

    envelope = update.sign_manifest(manifest, publisher)
    assert envelope == unhex(vector["envelope"])
    assert update.verify_envelope(envelope, publisher.public_key).digest == unhex(
        vector["manifest"]["digest"]
    )
    with pytest.raises(PamojaError):
        update.verify_envelope(unhex(vector["forgedEnvelope"]), publisher.public_key)

    anchor = DeviceIdentity.from_seed(unhex(vector["anchorSeed"]))
    statement = update.sign_delegation(
        update.Delegation(
            epoch=vector["delegation"]["epoch"],
            release_key=unhex(vector["delegation"]["releaseKey"]),
            expires=vector["delegation"]["expires"],
        ),
        anchor,
    )
    assert statement == unhex(vector["delegation"]["envelope"])

    life = vector["lifecycle"]
    fleet = update.Updater(
        unhex(vector["vendorId"]), unhex(vector["classId"]), publisher.public_key, 2, 4096
    )
    fleet.provision(0, 1)
    assert fleet.begin(envelope) == life["staged"]
    for at in range(0, len(image), life["chunk"]):
        fleet.write(image[at : at + life["chunk"]])
    assert fleet.finish() == life["staged"]

    boot = fleet.on_boot()
    assert boot.action == life["boot"]
    assert boot.slot == life["bootSlot"]
    assert fleet.confirm() == life["confirmed"]

    record = fleet.slot_record(life["confirmed"])
    assert record.state == life["state"]
    assert record.written == life["written"]


def test_power_vectors_match():
    vector = VECTORS["power"]
    plan = power.power_plan(
        vector["plan"]["activeUs"], vector["plan"]["saverUs"], vector["plan"]["criticalUs"]
    )

    assert plan.saver_below == pytest.approx(vector["plan"]["saverBelow"], abs=TOLERANCE)
    assert plan.critical_below == pytest.approx(vector["plan"]["criticalBelow"], abs=TOLERANCE)

    for at, soc in enumerate(vector["charges"]):
        assert plan.mode(soc) == vector["modes"][at]
        assert plan.mode_while_charging(soc, True) == vector["charging"][at]
        assert plan.interval_us(soc) == vector["intervalsUs"][at]

    duty = power.DutyCycle.from_fraction(
        vector["duty"]["periodUs"], vector["duty"]["fraction"]
    )
    assert duty.active_us == vector["duty"]["activeUs"]
    assert duty.sleep_us == vector["duty"]["sleepUs"]


def test_telemetry_vectors_match():
    vector = VECTORS["telemetry"]
    for at, cost in enumerate(vector["costs"]):
        assert telemetry.link_cost_threshold(telemetry.LinkCost(cost)) == vector["thresholds"][at]

    reporter = telemetry.Reporter(telemetry.Level.TRACE)
    reporter.adapt_to(telemetry.LinkCost(vector["adaptedTo"]))
    for at, level in enumerate(vector["levels"]):
        shipped = reporter.record(telemetry.Event(telemetry.Level(level), "vector"))
        assert (shipped is not None) == vector["shipped"][at]

    snapshot = reporter.snapshot()
    for key in ("trace", "debug", "info", "warn", "error", "emitted", "dropped"):
        assert getattr(snapshot, key) == vector["snapshot"][key]


def test_ladder_vectors_match():
    import asyncio

    from pamoja import ladder, loopback, sync, transport

    vector = VECTORS["ladder"]

    async def run():
        broker = loopback.LoopbackBroker()
        listener = broker.link()
        await listener.connect()
        await listener.subscribe(vector["topic"])

        offline = ladder.Ladder(sync.Store.memory())
        for at, payload in enumerate(vector["payloads"]):
            assert (
                await offline.send(vector["topic"], payload.encode())
                == vector["withNoRung"]["deliveries"][at]
            )
        assert await offline.buffered() == vector["withNoRung"]["buffered"]

        await offline.rung(broker.rung())
        await offline.connect()
        assert await offline.flush() == vector["afterTheLinkReturns"]["flushed"]
        assert await offline.buffered() == vector["afterTheLinkReturns"]["buffered"]

        rungs = ladder.Ladder(sync.Store.memory())
        await rungs.rung(
            transport.Transport.faulty(
                broker.rung(), vector["fallthrough"]["failuresOnFirstRung"]
            )
        )
        await rungs.rung(broker.rung())
        await rungs.connect()
        assert (
            await rungs.send(vector["topic"], vector["fallthrough"]["payload"].encode())
            == vector["fallthrough"]["delivery"]
        )

    asyncio.run(run())


def test_simulation_vectors_match():
    import asyncio

    from pamoja import sim

    vector = VECTORS["simulation"]

    async def run():
        want = vector["sensor"]
        sensor = sim.SimulatedSensor(
            want["baseline"],
            drift_per_read=want["driftPerRead"],
            noise=want["noise"],
            seed=want["seed"],
        )
        for reading in want["readings"]:
            assert await sensor.read() == pytest.approx(reading, abs=TOLERANCE)

        want = vector["replay"]
        replay = sim.Replay(want["capture"], repeating=want["repeating"])
        for reading in want["readings"]:
            assert await replay.read() == pytest.approx(reading, abs=TOLERANCE)

        want = vector["robot"]
        robot = sim.SimulatedRobot(want["dt"])
        for pose in want["poses"]:
            await robot.apply(vx=want["vx"], omega=want["omega"])
            reached = await robot.pose()
            assert reached.x == pytest.approx(pose["x"], abs=TOLERANCE)
            assert reached.y == pytest.approx(pose["y"], abs=TOLERANCE)
            assert reached.theta == pytest.approx(pose["theta"], abs=TOLERANCE)

    asyncio.run(run())


def _assert_control(policy, want: dict) -> None:
    """Checks a control policy against the flattened form the vectors carry."""
    assert policy.kind == want["kind"]
    if want["kind"] == "Setpoint":
        assert policy.setpoint == pytest.approx(want["setpoint"], abs=TOLERANCE)
        assert policy.hysteresis == pytest.approx(want["hysteresis"], abs=TOLERANCE)
        assert policy.cooling == want["cooling"]
        assert policy.safe_band == pytest.approx(want["safeBand"], abs=TOLERANCE)
    elif want["kind"] == "Level":
        assert policy.empty == pytest.approx(want["empty"], abs=TOLERANCE)
        assert policy.warn_within == want["warnWithin"]
    elif want["kind"] == "Surge":
        assert policy.rising == want["rising"]
        assert policy.limit == pytest.approx(want["limit"], abs=TOLERANCE)


def _assert_reactions(control, reactions: list[dict]) -> None:
    """Walks a controller through a recorded run and checks every decision."""
    for want in reactions:
        reaction = control.evaluate(want["reading"])
        assert reaction.actuator == want["actuator"], (
            f"the output setting at {want['reading']}"
        )

        kind = want["alert"]["kind"]
        if kind == "None":
            assert reaction.alert is None
            continue

        assert reaction.alert is not None
        assert reaction.alert.kind == kind, f"the alert raised at {want['reading']}"
        if kind == "OutOfRange":
            assert reaction.alert.reading == pytest.approx(
                want["alert"]["reading"], abs=TOLERANCE
            )
        elif kind == "RunningOut":
            assert reaction.alert.samples == want["alert"]["samples"]
        elif kind == "ChangingFast":
            assert reaction.alert.rate == pytest.approx(
                want["alert"]["rate"], abs=TOLERANCE
            )


def test_profile_vectors_match():
    vector = VECTORS["profile"]

    cold_chain = vector["coldChain"]
    fridge = profile.Profile.vaccine_fridge_monitor()
    assert fridge.name == cold_chain["name"]
    assert fridge.topic == cold_chain["topic"]
    _assert_control(fridge.control, cold_chain["control"])
    assert fridge.power.active_secs == cold_chain["power"]["activeSecs"]
    assert fridge.power.saver_below == pytest.approx(
        cold_chain["power"]["saverBelow"], abs=TOLERANCE
    )
    _assert_reactions(fridge.controller(), cold_chain["reactions"])

    draining = vector["draining"]
    well = profile.Profile.well_level()
    assert well.name == draining["name"]
    _assert_control(well.control, draining["control"])
    _assert_reactions(well.controller(), draining["reactions"])

    observed = profile.Controller.monitor().evaluate(vector["observed"]["reading"])
    assert observed.actuator is None, "a monitoring profile drives no output"
    assert observed.alert is None
    assert vector["observed"]["alert"]["kind"] == "None"


def test_ros2_vectors_match():
    vector = VECTORS["ros2"]

    for want in vector["names"]:
        assert ros2.is_valid_name(want["name"]) == want["valid"]
        assert ros2.is_fully_qualified(want["name"]) == want["fullyQualified"]

    for want in vector["ddsTopics"]:
        assert ros2.dds_topic(want["fqn"], want["kind"]) == want["topic"]

    for kind, prefix in vector["prefixes"].items():
        assert ros2.prefix_for(kind) == prefix

    assert ros2.percent_mangle(vector["mangled"]["name"]) == vector["mangled"]["mangled"]

    for want in vector["typeNames"]:
        assert ros2.dds_type_name(want["rosType"]) == want["ddsType"]

    digest = ros2.type_hash_digest(vector["typeHash"]["text"])
    assert bytes(digest).hex() == vector["typeHash"]["digest"]

    key = vector["entityKey"]
    assert (
        ros2.entity_key(
            key["domainId"], key["fqn"], key["rosType"], vector["typeHash"]["text"]
        )
        == key["key"]
    )

    twist = vector["twist"]
    encoded = ros2.twist_to_cdr(tuple(twist["linear"]), tuple(twist["angular"]))
    assert bytes(encoded).hex() == twist["cdr"], (
        "a twist encodes to the same CDR everywhere"
    )
    linear, angular = ros2.twist_from_cdr(encoded)
    assert linear == pytest.approx(tuple(twist["linear"]), abs=TOLERANCE)
    assert angular == pytest.approx(tuple(twist["angular"]), abs=TOLERANCE)

    mixed = vector["mixedWidths"]
    reader = ros2.CdrReader(bytes.fromhex(mixed["cdr"]))
    assert reader.read_u32() == mixed["word"]
    assert reader.read_f64() == pytest.approx(mixed["double"], abs=TOLERANCE), (
        "an eight-byte field keeps its alignment"
    )
    assert reader.read_i32() == mixed["signed"], (
        "and the field after it is not skewed"
    )


def test_zenoh_vectors_match():
    vector = VECTORS["zenoh"]

    for want in vector["expressions"]:
        assert zenoh.is_valid(want["key"]) == want["valid"]
        assert zenoh.is_canon(want["key"]) == want["canon"]

    for want in vector["canonized"]:
        assert zenoh.canonize(want["key"]) == want["canonical"]

    for want in vector["matches"]:
        assert zenoh.matches(want["pattern"], want["key"]) == want["matches"]
