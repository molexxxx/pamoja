"""The Python side of the cross-language conformance suite.

Runs the same vectors from ``conformance/vectors.json`` that every other binding
runs, so a facade that drifts here fails rather than quietly disagreeing with
Rust, Node, and .NET.
"""

import json
import math
import re
import pathlib

import pytest

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples
from pamoja.kit import Calibration, Coordinate, Depletion, Geofence, Pid, Smoother, Thermostat, Trigger, deadband
from pamoja.security import DeviceIdentity, verify
from pamoja import actuators, audit, can, gateway, gpio, lora, lorawan, mesh, modbus, power, profile, radios, ros2, routing, sensors, serial, session, telemetry, update, zenoh
from pamoja.core import PamojaError
from pamoja.lorawan import clock, firmware, fragment, multicast, relay
from pamoja.kit import WINDOW_CAPACITY, Anomaly, Median, Trend, Window

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


def close_all(got, want, what):
    """Assert a list of numbers agrees with the vectors, element by element."""
    assert len(got) == len(want), f"{what}: length"
    for index, (value, expected) in enumerate(zip(got, want)):
        assert value == pytest.approx(expected, abs=TOLERANCE), f"{what}[{index}]"


def test_kit_extra_vectors_match():
    import math

    from pamoja.kit import (
        Complementary,
        celsius_to_fahrenheit,
        celsius_to_kelvin,
        dew_point,
        pascals_to_hectopascals,
        pascals_to_psi,
        tilt_from_accel,
    )

    vector = VECTORS["kitExtras"]
    units = vector["units"]
    close_all([celsius_to_fahrenheit(c) for c in units["celsius"]], units["fahrenheit"], "fahrenheit")
    close_all([celsius_to_kelvin(c) for c in units["celsius"]], units["kelvin"], "kelvin")
    close_all([pascals_to_hectopascals(p) for p in units["pascals"]], units["hectopascals"], "hectopascals")
    close_all([pascals_to_psi(p) for p in units["pascals"]], units["psi"], "psi")
    for want in vector["tilts"]:
        tilt = tilt_from_accel(*want["accel"])
        assert tilt.roll == pytest.approx(want["roll"], abs=TOLERANCE)
        assert tilt.pitch == pytest.approx(want["pitch"], abs=TOLERANCE)
    for want in vector["dewPoints"]:
        assert dew_point(want["celsius"], want["humidity"]) == pytest.approx(want["dewPoint"], abs=TOLERANCE)
    complementary = vector["complementary"]
    fused = Complementary(complementary["alpha"], complementary["initial"])
    estimates = [
        fused.update(math.nan if step["rate"] is None else step["rate"], step["absolute"], step["dt"])
        for step in complementary["steps"]
    ]
    close_all(estimates, complementary["estimates"], "complementary")


def test_motion_vectors_match():
    from pamoja.kit import (
        Ackermann,
        Coordinate,
        DhParameters,
        DiffDrive,
        Elbow,
        Esc,
        Limits,
        Mecanum,
        Odometry,
        Quadrature,
        QuadratureScale,
        SafetyGate,
        ServoMap,
        SkidSteer,
        Twist,
        TwoLinkArm,
        WaypointFollower,
        forward_kinematics,
        obstacle_stop,
    )

    vector = VECTORS["motion"]

    drive = DiffDrive(vector["diffDrive"]["track"])
    for want in vector["diffDrive"]["commands"]:
        got = drive.wheel_speeds(want["linear"], want["angular"])
        close_all(got, [want["left"], want["right"]], "differential wheels")

    car = Ackermann(vector["ackermann"]["wheelbase"])
    for want in vector["ackermann"]["steering"]:
        radius = car.turn_radius(want["steering"])
        if want["turnRadius"] is None:
            assert radius == float("inf"), "wheels straight never turn"
        else:
            assert radius == pytest.approx(want["turnRadius"], abs=TOLERANCE)
        assert car.yaw_rate(vector["ackermann"]["linear"], want["steering"]) == pytest.approx(
            want["yawRate"], abs=TOLERANCE
        )
        assert car.curvature(want["steering"]) == pytest.approx(want["curvature"], abs=TOLERANCE)

    skid = vector["skidSteer"]
    sides = SkidSteer(skid["track"], skid["slip"]).wheel_speeds(skid["linear"], skid["angular"])
    close_all(sides, [skid["left"], skid["right"]], "skid steer")

    base = Mecanum(vector["mecanum"]["wheelbase"], vector["mecanum"]["track"])
    for want in vector["mecanum"]["twists"]:
        wheels = base.wheel_speeds(Twist(*want["twist"]))
        close_all(
            [wheels.front_left, wheels.front_right, wheels.rear_left, wheels.rear_right],
            want["wheels"],
            "mecanum wheels",
        )

    arm = TwoLinkArm(vector["arm"]["l1"], vector["arm"]["l2"])
    for want in vector["arm"]["targets"]:
        for elbow, key in ((Elbow.UP, "up"), (Elbow.DOWN, "down")):
            got = arm.joints_for(*want["target"], elbow)
            if want[key] is None:
                assert got is None, f"no {key} solution for {want['target']}"
            else:
                close_all(got, want[key], f"arm {key}")

    chain = [DhParameters(*joint) for joint in vector["forwardKinematics"]["joints"]]
    close_all(forward_kinematics(chain).elements, vector["forwardKinematics"]["transform"], "forward kinematics")

    odometry = Odometry()
    for step, want in zip(vector["odometry"]["steps"], vector["odometry"]["poses"]):
        pose = odometry.integrate(*step)
        close_all([pose.x, pose.y, pose.theta], want, "odometry pose")

    waypoint = vector["waypoint"]
    follower = WaypointFollower(
        waypoint["cruise"], waypoint["arrivalM"], waypoint["headingGain"], waypoint["maxAngular"]
    )
    here = Coordinate(*waypoint["here"])
    for want in waypoint["guidance"]:
        got = follower.guide(here, want["heading"], Coordinate(*want["target"]))
        close_all([got.twist.vx, got.twist.vy, got.twist.omega], want["twist"], "guidance twist")
        assert got.distance_m == pytest.approx(want["distanceM"], abs=TOLERANCE)
        assert got.heading_error_deg == pytest.approx(want["headingErrorDeg"], abs=TOLERANCE)
        assert got.arrived is want["arrived"]

    stop = vector["obstacleStop"]
    moving = Twist(*stop["twist"])
    clear = obstacle_stop(moving, stop["clear"], stop["stopDistance"])
    close_all([clear.vx, clear.vy, clear.omega], stop["clearTwist"], "clear ahead")
    near = obstacle_stop(moving, stop["near"], stop["stopDistance"])
    close_all([near.vx, near.vy, near.omega], stop["nearTwist"], "obstacle ahead")

    gate_vector = vector["safetyGate"]
    gate = SafetyGate(Limits(*gate_vector["limits"]), gate_vector["watchdogTimeout"])
    gate.feed()
    desired = Twist(*gate_vector["desired"])
    for want in gate_vector["commands"]:
        got = gate.command(desired, gate_vector["dt"])
        close_all([got.vx, got.vy, got.omega], want, "gate command")
    silent = gate.command(desired, gate_vector["dt"])
    close_all([silent.vx, silent.vy, silent.omega], gate_vector["afterSilence"], "gate after silence")

    servo = ServoMap.standard()
    close_all([servo.pulse(angle) for angle in vector["servo"]["angles"]], vector["servo"]["pulses"], "servo")
    assert servo.angle(vector["servo"]["pulseBack"]) == pytest.approx(vector["servo"]["angleBack"], abs=TOLERANCE)
    esc = Esc.bidirectional()
    close_all([esc.pulse(throttle) for throttle in vector["esc"]["throttles"]], vector["esc"]["pulses"], "esc")

    q = vector["quadrature"]
    encoder = Quadrature()
    close_all([encoder.update(a, b) for a, b in q["edges"]], q["deltas"], "quadrature deltas")
    assert encoder.count == q["count"]
    scale = QuadratureScale(q["countsPerRev"], q["wheelRadius"])
    assert scale.distance(encoder.count) == pytest.approx(q["distance"], abs=TOLERANCE)
    assert scale.velocity(90, 0.5) == pytest.approx(q["velocity"], abs=TOLERANCE)


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


def test_trigger_vectors_match():
    vector = VECTORS["trigger"]
    trigger = Trigger.below(vector["threshold"], vector["hysteresis"])
    for reading, want in zip(vector["readings"], vector["outputs"]):
        assert trigger.update(reading) == want


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


def test_a_perturbed_vector_is_rejected():
    """A suite that stopped comparing would pass every vector in the file, so this asserts
    the comparison itself: the committed frame matches what the library builds, and a frame
    with one bit moved does not. Every binding's runner carries the same case."""
    read = VECTORS["modbus"]["readHoldingRegisters"]
    built = modbus.read_holding_registers(read["address"], read["start"], read["count"])

    committed = unhex(read["frame"])
    assert built == committed, "the committed vector still matches"

    perturbed = bytearray(committed)
    perturbed[-1] ^= 0x01
    assert built != bytes(perturbed), "a vector with one bit moved must not compare equal"


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
    for ctrl in bme["ctrlMeas"]:
        fields = sensors.Bme280CtrlMeas(ctrl["temperature"], ctrl["pressure"], ctrl["mode"])
        assert sensors.bme280.ctrl_meas_bits(fields) == ctrl["bits"]
        assert sensors.bme280.ctrl_meas_from_bits(ctrl["bits"]) == fields
        assert (
            sensors.bme280.max_measurement_micros(ctrl["temperature"], ctrl["pressure"], 1)
            == ctrl["maxMeasurementMicros"]
        )
    for hum in bme["ctrlHum"]:
        assert sensors.bme280.ctrl_hum_bits(hum["humidity"]) == hum["bits"]
        assert sensors.bme280.ctrl_hum_from_bits(hum["bits"]) == hum["humidity"]
    for config in bme["config"]:
        fields = sensors.Bme280Config(config["standby"], config["filter"], config["spi3Wire"])
        assert sensors.bme280.config_bits(fields) == config["bits"]
        assert sensors.bme280.config_from_bits(config["bits"]) == fields
    sim = bme["simulated"]
    burst = sensors.bme280.sim.burst_for(sim["celsius"], sim["hectopascals"], sim["relativeHumidity"])
    assert burst.hex() == sim["burst"]

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


def test_later_sensor_vectors_match():
    """The parts added after the first four, against the vectors Rust asserts."""
    vector = VECTORS["sensors"]

    bmp = vector["bmp280"]
    calibration = sensors.bmp280.calibration(unhex(bmp["calibration"]))
    reading = calibration.compensate(unhex(bmp["measurement"]))
    assert reading.celsius == pytest.approx(bmp["celsius"], abs=1e-3)
    assert reading.pascals == bmp["pascals"]
    assert calibration.to_bytes().hex() == bmp["calibrationRoundTrip"]
    assert sensors.bmp280.CHIP_ID == bmp["chipId"]

    sht = vector["sht3x"]
    decoded = sensors.sht3x.parse_measurement(unhex(sht["measurement"]))
    assert decoded.temperature_raw == sht["temperatureRaw"]
    assert sensors.sht3x.milli_celsius(decoded.temperature_raw) == sht["milliCelsius"]
    assert sensors.sht3x.milli_fahrenheit(decoded.temperature_raw) == sht["milliFahrenheit"]
    assert sensors.sht3x.milli_percent(decoded.humidity_raw) == sht["milliPercent"]
    assert sensors.sht3x.crc(unhex(sht["crcInput"])) == sht["crc"]
    with pytest.raises(PamojaError):
        sensors.sht3x.parse_measurement(unhex(sht["corruptMeasurement"]))
    status = sensors.sht3x.parse_status(unhex(sht["status"]))
    assert status.bits == sht["statusBits"]
    assert status.alert_pending == sht["alertPending"]
    assert status.heater_on == sht["heaterOn"]

    scd = vector["scd4x"]
    air = sensors.scd4x.parse_measurement(unhex(scd["measurement"]))
    assert air.co2_ppm == scd["co2Ppm"]
    assert sensors.scd4x.milli_celsius(air.temperature_raw) == scd["milliCelsius"]
    assert sensors.scd4x.humidity_milli_percent(air.humidity_raw) == scd["humidityMilliPercent"]
    with pytest.raises(PamojaError):
        sensors.scd4x.parse_measurement(unhex(scd["corruptMeasurement"]))
    # The offset scales by 2^16 where the measurement scales by 2^16 - 1, in the same
    # datasheet, so it is pinned separately.
    assert (
        sensors.scd4x.temperature_offset_word(scd["temperatureOffsetMilliCelsius"])
        == scd["temperatureOffsetWord"]
    )
    assert sensors.scd4x.serial_number(unhex(scd["serialFrame"])) == scd["serialNumber"]

    tmp = vector["tmp117"]
    for entry in tmp["readings"]:
        raw = entry["register"] - 0x10000 if entry["register"] > 0x7FFF else entry["register"]
        assert sensors.tmp117.micro_celsius(raw) == entry["microCelsius"]
        assert sensors.tmp117.nano_celsius(raw) == entry["nanoCelsius"]
    assert sensors.tmp117.high_alert(tmp["flagConfig"]) == tmp["highAlert"]
    assert sensors.tmp117.data_ready(tmp["flagConfig"]) == tmp["dataReady"]

    hdc = vector["hdc1080"]
    room = sensors.hdc1080.parse_measurement(unhex(hdc["measurement"]))
    assert room.temperature_raw == hdc["temperatureRaw"]
    assert sensors.hdc1080.milli_celsius(room.temperature_raw) == hdc["milliCelsius"]
    assert sensors.hdc1080.milli_percent(room.humidity_raw) == hdc["milliPercent"]
    with pytest.raises(PamojaError):
        sensors.hdc1080.config_from_register(hdc["invalidConfiguration"])

    opt = vector["opt3001"]
    for entry in opt["results"]:
        assert sensors.opt3001.milli_lux(entry["register"]) == entry["milliLux"]
    for entry in opt["ranges"]:
        assert sensors.opt3001.full_scale_milli_lux(entry["range"]) == entry["fullScaleMilliLux"]
    assert sensors.opt3001.full_scale_milli_lux(opt["invalidRange"]) is None

    ina = vector["ina226"]
    assert (
        sensors.ina226.calibration(ina["currentLsbMicroamps"], ina["shuntMilliohms"])
        == ina["calibration"]
    )
    assert sensors.ina226.shunt_nanovolts(ina["rawShunt"]) == ina["shuntNanovolts"]
    assert sensors.ina226.bus_microvolts(ina["rawBus"]) == ina["busMicrovolts"]
    assert (
        sensors.ina226.current_microamps(ina["rawCurrent"], ina["currentLsbMicroamps"])
        == ina["currentMicroamps"]
    )
    assert (
        sensors.ina226.power_microwatts(ina["rawPower"], ina["currentLsbMicroamps"])
        == ina["powerMicrowatts"]
    )
    with pytest.raises(PamojaError):
        sensors.ina226.identify(ina["manufacturerId"], ina["badDieId"])

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
        pwm["servoCenter"]["pulseMicros"], pwm["servoCenter"]["updateRateHz"]
    ) == unhex(pwm["servoCenter"]["bytes"])
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

    relay_channels = [
        {
            "worFrequencyHz": channel.wor_frequency_hz,
            "ackFrequencyHz": channel.ack_frequency_hz,
            "dataRate": channel.data_rate,
        }
        for channel in plan.relay_channels()
    ]
    assert relay_channels == want["relayChannels"], where

    _check_rules(plan, want["rules"], where)


def _block_of(block) -> dict:
    """Describe a channel block the way the vectors do."""
    return {
        "startHz": block.start_hz,
        "stepHz": block.step_hz,
        "count": block.count,
        "minDataRate": block.min_data_rate,
        "maxDataRate": block.max_data_rate,
    }


def _check_rules(plan, want: dict, where: str) -> None:
    """Hold a plan's channel rules to the answers every binding must give."""
    rules = plan.rules()
    assert {
        "kind": rules.kind,
        "channelList": rules.channel_list,
        "txParamSetup": rules.tx_param_setup,
        "joinSequence": rules.join_sequence,
        "powerReference": rules.power_reference,
        "gainAllowanceDb": rules.gain_allowance_db,
    } == {
        key: want[key]
        for key in (
            "kind",
            "channelList",
            "txParamSetup",
            "joinSequence",
            "powerReference",
            "gainAllowanceDb",
        )
    }, where

    for value, control in enumerate(want["maskControls"]):
        got = plan.mask_control(value)
        assert {
            "kind": got.kind,
            "group": got.group,
            "on": got.on,
            "thenGroup": got.then_group,
        } == control, f"{where} ChMaskCntl {value}"
    assert plan.mask_control(8) is None, where

    assert rules.downlink_channel_block_count == len(want["downlinkChannelBlocks"]), where
    assert [
        _block_of(block) for block in plan.channel_blocks("downlink")
    ] == want["downlinkChannelBlocks"], where
    for channel, frequency in enumerate(want["downlinkChannelFrequencies"]):
        assert plan.downlink_channel_frequency_hz(channel) == frequency, (
            f"{where} downlink channel {channel}"
        )
    downlink_count = sum(block["count"] for block in want["downlinkChannelBlocks"])
    assert (
        plan.downlink_channel_frequency_hz(downlink_count) == want["downlinkChannelPastEnd"]
    ), where

    for probe in want["rx1Frequencies"]:
        assert (
            plan.rx1_frequency_hz(probe["uplinkChannel"], probe["uplinkHz"]) == probe["rx1Hz"]
        ), f"{where} RX1 after uplink channel {probe['uplinkChannel']}"

    assert rules.join_plan_count == len(want["joinPlans"]), where
    assert [
        {
            "channels": _block_of(run.channels),
            "acceptStartHz": run.accept_start_hz,
            "acceptStepHz": run.accept_step_hz,
            "rx2StartHz": run.rx2_start_hz,
            "rx2StepHz": run.rx2_step_hz,
            "plan": run.plan,
        }
        for run in plan.join_plans()
    ] == want["joinPlans"], where
    for entry in want["joinPlaces"]:
        got = plan.join_plan_for_channel(entry["joinChannel"])
        described = (
            None
            if got is None
            else {
                "index": got.index,
                "offset": got.offset,
                "acceptHz": got.accept_hz,
                "rx2Hz": got.rx2_hz,
            }
        )
        assert described == entry["place"], f"{where} join channel {entry['joinChannel']}"


def test_lora_budget_vectors_match():
    vector = VECTORS["lora"]["budget"]
    links = {entry["name"]: entry for entry in VECTORS["lora"]["links"]}

    def hundredths(value: float) -> int:
        return round(value * 100)

    assert hundredths(lora.RADIO_NOISE_FIGURE_DB) == vector["radioNoiseFigureHundredths"]
    assert hundredths(lora.GATEWAY_NOISE_FIGURE_DB) == vector["gatewayNoiseFigureHundredths"]
    assert hundredths(lora.LinkBudget().noise_figure_db) == vector["radioNoiseFigureHundredths"]

    for floor in vector["noiseFloors"]:
        assert hundredths(lora.noise_floor_dbm(floor["bandwidthHz"])) == floor["hundredths"], floor
    for snr in vector["demodulatorSnrs"]:
        got = lora.demodulator_snr_db(snr["spreadingFactor"])
        assert hundredths(got) == snr["hundredths"], snr
    for loss in vector["freeSpaceLosses"]:
        got = lora.free_space_loss_db(loss["distanceM"], loss["frequencyHz"])
        assert hundredths(got) == loss["hundredths"], loss
    for radius in vector["fresnelRadii"]:
        got = lora.fresnel_radius_mm(radius["nearM"], radius["farM"], radius["frequencyHz"])
        assert got == radius["radiusMm"], radius

    for described in vector["budgets"]:
        where = described["name"]
        link = _link_of(links[described["link"]])
        budget = lora.LinkBudget(
            transmit_power_dbm=described["transmitPowerHundredths"] / 100,
            transmit_antenna_gain_dbi=described["transmitAntennaGainHundredths"] / 100,
            transmit_cable_loss_db=described["transmitCableLossHundredths"] / 100,
            receive_antenna_gain_dbi=described["receiveAntennaGainHundredths"] / 100,
            receive_cable_loss_db=described["receiveCableLossHundredths"] / 100,
            noise_figure_db=described["noiseFigureHundredths"] / 100,
        )
        path = described["pathLossHundredths"] / 100
        ceiling = described["ceilingHundredths"] / 100
        assert hundredths(budget.eirp_dbm()) == described["eirpHundredths"], where
        assert hundredths(budget.received_dbm(path)) == described["receivedHundredths"], where
        assert hundredths(budget.sensitivity_dbm(link)) == described["sensitivityHundredths"], where
        assert hundredths(budget.max_path_loss_db(link)) == described["maxPathLossHundredths"], where
        assert hundredths(budget.margin_db(link, path)) == described["marginHundredths"], where
        assert (
            hundredths(budget.max_transmit_power_dbm(ceiling))
            == described["maxTransmitPowerHundredths"]
        ), where

    for rule in vector["fcc"]:
        got = lora.fcc_max_conducted_dbm(rule["antennaGainHundredths"] / 100, rule["hoppingChannels"])
        assert (None if got is None else hundredths(got)) == rule["maxConductedHundredths"], rule


def test_radio_sim_vectors_match():
    vector = VECTORS["radioSim"]
    boards = {
        "Sx126x": lambda: radios.SimulatedLoraChip.sx126x("HighPower"),
        "Sx127x": lambda: radios.SimulatedLoraChip.sx127x("PaBoost"),
    }
    rssi, snr = vector["rssiCentiDbm"] / 100, vector["snrCentiDb"] / 100
    for described in vector["chips"]:
        chip = boards[described["family"]]()
        assert chip.family == described["family"]
        assert chip.tuning().frequency_hz == described["resetFrequencyHz"]
        radio = chip.radio()
        radio.configure(
            vector["frequencyHz"],
            lora.link(vector["spreadingFactor"], vector["bandwidthHz"]),
            vector["outputDbm"],
            sync_word=vector["syncWord"],
        )
        assert radio.transmit(unhex(vector["reading"])) == described["airtimeUs"]
        (sent,) = chip.sent()
        want = described["sent"]
        assert sent.tuning.frequency_hz == want["frequencyHz"]
        assert sent.tuning.link.spreading_factor == want["spreadingFactor"]
        assert sent.tuning.link.bandwidth_hz == want["bandwidthHz"]
        assert sent.tuning.output_dbm == want["outputDbm"]
        assert sent.tuning.sync_word == want["syncWord"]
        assert sent.payload.hex() == want["payload"]

        assert radio.detect(4) == described["activity"]["idle"]
        chip.hear(unhex(vector["answer"]), rssi, snr)
        assert radio.detect(4) == described["activity"]["withAFrame"]
        heard = radio.receive(1_000_000)
        received = described["received"]
        assert heard.outcome == "Frame"
        assert heard.payload.hex() == received["payload"]
        assert heard.rssi_dbm == received["rssiCentiDbm"] / 100
        assert heard.snr_db == received["snrCentiDb"] / 100
        assert heard.signal_rssi_dbm == received["signalRssiCentiDbm"] / 100
        assert radio.receive(1_000_000).outcome == described["quiet"]
        chip.hear_corrupt(rssi, snr)
        assert radio.receive(1_000_000).outcome == described["broken"]
        radio.close()

def test_radios_vectors_match():
    vector = VECTORS["radios"]
    links = {entry["name"]: entry for entry in VECTORS["lora"]["links"]}
    sx126x = radios.sx126x

    def hundredths(value: float) -> int:
        return round(value * 100)

    def amplifier(name: str) -> sx126x.Amplifier:
        return sx126x.Amplifier.LOW_POWER if name == "low" else sx126x.Amplifier.HIGH_POWER

    def pa_config(power) -> str:
        return bytes([power.pa_duty_cycle, power.hp_max, power.device_sel, power.pa_lut]).hex()

    def pascal(name: str) -> str:
        return name[0].upper() + name[1:]

    for entry in vector["frequencyWords"]:
        assert sx126x.frequency_word(entry["frequencyHz"]) == entry["word"], entry
    for entry in vector["timeouts"]:
        assert sx126x.timeout_steps(entry["timeoutUs"]) == entry["steps"], entry
    assert sx126x.RX_CONTINUOUS == vector["rxContinuous"]
    for entry in vector["imageCalibrations"]:
        assert sx126x.image_calibration(entry["lowHz"], entry["highHz"]).hex() == entry["codes"], entry
    for entry in vector["txPowers"]:
        power = sx126x.tx_power(amplifier(entry["amplifier"]), entry["outputDbm"])
        assert pa_config(power) == entry["paConfig"], entry
        assert power.setting_dbm == entry["settingDbm"], entry
    for entry in vector["underCeilings"]:
        budget = lora.LinkBudget(
            transmit_antenna_gain_dbi=entry["transmitAntennaGainHundredths"] / 100,
            transmit_cable_loss_db=entry["transmitCableLossHundredths"] / 100,
        )
        power = sx126x.tx_power_under_ceiling(
            amplifier(entry["amplifier"]), budget, entry["ceilingHundredths"] / 100
        )
        assert pa_config(power) == entry["paConfig"], entry
        assert power.setting_dbm == entry["settingDbm"], entry
    assert f"{sx126x.SYNC_WORD_PUBLIC:04x}" == vector["syncWords"]["public"]
    assert f"{sx126x.SYNC_WORD_PRIVATE:04x}" == vector["syncWords"]["private"]

    commands = vector["commands"]
    assert sx126x.set_standby().hex() == commands["setStandby"]
    assert sx126x.set_packet_type_lora().hex() == commands["setPacketTypeLora"]
    wanted = commands["setRfFrequency"]
    assert sx126x.set_rf_frequency(wanted["frequencyHz"]).hex() == wanted["bytes"]
    wanted = commands["calibrateImage"]
    assert sx126x.calibrate_image(wanted["lowHz"], wanted["highHz"]).hex() == wanted["bytes"]
    high_14 = sx126x.tx_power(sx126x.Amplifier.HIGH_POWER, commands["setPaConfig"]["outputDbm"])
    assert sx126x.set_pa_config(high_14).hex() == commands["setPaConfig"]["bytes"]
    wanted = commands["setTxParams"]
    assert sx126x.set_tx_params(high_14, wanted["rampUs"]).hex() == wanted["bytes"]
    for entry in commands["setLoraModulationParams"]:
        link = _link_of(links[entry["link"]])
        assert sx126x.set_lora_modulation_params(link).hex() == entry["bytes"], entry
    for entry in commands["setLoraPacketParams"]:
        link = _link_of(links[entry["link"]])
        got = sx126x.set_lora_packet_params(link, entry["payloadLen"], entry["invertIq"])
        assert got.hex() == entry["bytes"], entry
    for entry in commands["setDioIrqParams"]:
        assert sx126x.set_dio_irq_params(entry["irq"], entry["dio1"]).hex() == entry["bytes"], entry
    wanted = commands["clearIrqStatus"]
    assert sx126x.clear_irq_status(wanted["irq"]).hex() == wanted["bytes"]
    assert sx126x.set_tx(commands["setTx"]["timeoutUs"]).hex() == commands["setTx"]["bytes"]
    assert sx126x.set_rx(commands["setRx"]["timeoutUs"]).hex() == commands["setRx"]["bytes"]
    assert sx126x.set_rx_continuous().hex() == commands["setRxContinuous"]
    wanted = commands["setSleep"]
    assert sx126x.set_sleep(wanted["warmStart"]).hex() == wanted["bytes"]
    public_word = bytes.fromhex(vector["syncWords"]["public"])
    assert (
        sx126x.write_register(sx126x.REGISTER_LORA_SYNC_WORD, public_word).hex()
        == commands["setSyncWord"]["bytes"]
    )
    wanted = commands["writeRegister"]
    got = sx126x.write_register(wanted["address"], bytes.fromhex(wanted["values"]))
    assert got.hex() == wanted["bytes"]
    wanted = commands["writeBuffer"]
    got = sx126x.write_buffer(wanted["offset"], bytes.fromhex(wanted["payload"]))
    assert got.hex() == wanted["bytes"]

    queries = vector["queries"]

    def same(query, want: dict) -> None:
        assert query.bytes.hex() == want["bytes"], want
        assert query.answer_len == want["answerLen"], want

    same(sx126x.get_status(), queries["getStatus"])
    same(sx126x.get_irq_status(), queries["getIrqStatus"])
    same(sx126x.get_rx_buffer_status(), queries["getRxBufferStatus"])
    same(sx126x.get_packet_status(), queries["getPacketStatus"])
    same(sx126x.get_rssi_inst(), queries["getRssiInst"])
    same(sx126x.get_device_errors(), queries["getDeviceErrors"])
    wanted = queries["readRegister"]
    same(sx126x.read_register(wanted["address"], wanted["length"]), wanted["query"])
    wanted = queries["readBuffer"]
    same(sx126x.read_buffer(wanted["offset"], wanted["length"]), wanted["query"])

    flags = {
        "txDone": sx126x.Irq.TX_DONE,
        "rxDone": sx126x.Irq.RX_DONE,
        "preambleDetected": sx126x.Irq.PREAMBLE_DETECTED,
        "syncWordValid": sx126x.Irq.SYNC_WORD_VALID,
        "headerValid": sx126x.Irq.HEADER_VALID,
        "headerError": sx126x.Irq.HEADER_ERROR,
        "crcError": sx126x.Irq.CRC_ERROR,
        "cadDone": sx126x.Irq.CAD_DONE,
        "cadDetected": sx126x.Irq.CAD_DETECTED,
        "timeout": sx126x.Irq.TIMEOUT,
        "lrFhssHop": sx126x.Irq.LR_FHSS_HOP,
    }
    for name, bits in vector["irqFlags"].items():
        assert int(flags[name]) == bits, name
    for entry in vector["irqs"]:
        bits = sx126x.irq(bytes.fromhex(entry["bytes"]))
        assert int(bits) == entry["bits"], entry
        for name, flag in flags.items():
            assert bool(bits & flag) == (name in entry["flags"]), (name, entry)
    for entry in vector["statuses"]:
        status = sx126x.status(entry["byte"])
        assert status.chip_mode == pascal(entry["chipMode"]), entry
        assert status.command_status == pascal(entry["commandStatus"]), entry
        assert status.error == entry["error"], entry
    for entry in vector["packetStatuses"]:
        status = sx126x.packet_status(bytes.fromhex(entry["bytes"]))
        assert hundredths(status.rssi_dbm) == entry["rssiHundredths"], entry
        assert hundredths(status.snr_db) == entry["snrHundredths"], entry
        assert hundredths(status.signal_rssi_dbm) == entry["signalRssiHundredths"], entry
    for entry in vector["rxBufferStatuses"]:
        status = sx126x.rx_buffer_status(bytes.fromhex(entry["bytes"]))
        assert status.payload_len == entry["payloadLen"], entry
        assert status.start == entry["start"], entry
    for entry in vector["rssiInst"]:
        assert hundredths(sx126x.rssi_inst_dbm(entry["byte"])) == entry["hundredths"], entry
    errors = {
        "rc64kCalibration": sx126x.DeviceError.RC64K_CALIBRATION,
        "rc13mCalibration": sx126x.DeviceError.RC13M_CALIBRATION,
        "pllCalibration": sx126x.DeviceError.PLL_CALIBRATION,
        "adcCalibration": sx126x.DeviceError.ADC_CALIBRATION,
        "imageCalibration": sx126x.DeviceError.IMAGE_CALIBRATION,
        "xoscStart": sx126x.DeviceError.XOSC_START,
        "pllLock": sx126x.DeviceError.PLL_LOCK,
        "paRamp": sx126x.DeviceError.PA_RAMP,
    }
    for entry in vector["deviceErrors"]:
        bits = sx126x.device_errors(bytes.fromhex(entry["bytes"]))
        assert int(bits) == entry["bits"], entry
        for name, flag in errors.items():
            assert bool(bits & flag) == (name in entry["flags"]), (name, entry)

    duty = vector["dutyCycle"]
    guard = radios.DutyCycle(duty["permille"])
    link = _link_of(links[duty["link"]])
    assert guard.transmitted(duty["startedUs"], link, duty["payloadLen"]) == duty["airtimeUs"]
    assert guard.earliest_us == duty["earliestUs"]
    for check in duty["checks"]:
        assert guard.wait_us(check["nowUs"]) == check["waitUs"], check
        assert guard.ready(check["nowUs"]) == check["ready"], check
    forbidden = radios.DutyCycle(duty["forbidden"]["permille"])
    for now_us in duty["forbidden"]["readyAt"]:
        assert forbidden.ready(now_us) == duty["forbidden"]["ready"], now_us
    assert forbidden.earliest_us is None


def test_sx127x_and_llcc68_vectors_match():
    import re

    vector = VECTORS["radios"]["sx127x"]
    links = {entry["name"]: entry for entry in VECTORS["lora"]["links"]}
    sx127x = radios.sx127x

    def upper(name: str) -> str:
        return re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[a-z])(?=[0-9])", "_", name).upper()

    def output(name: str) -> sx127x.PaOutput:
        return sx127x.PaOutput.RFO if name == "rfo" else sx127x.PaOutput.PA_BOOST

    def hundredths(value: float) -> int:
        return round(value * 100)

    for name, address in vector["registers"].items():
        assert int(sx127x.Register[upper(name)]) == address, name
    assert {
        "version": sx127x.VERSION,
        "write": sx127x.WRITE,
        "dio0RxDone": int(sx127x.Dio0.RX_DONE),
        "dio0TxDone": int(sx127x.Dio0.TX_DONE),
        "dio0CadDone": int(sx127x.Dio0.CAD_DONE),
        "paDacDefault": sx127x.PA_DAC_DEFAULT,
        "paDacHighPower": sx127x.PA_DAC_HIGH_POWER,
        "imageCalStart": sx127x.IMAGE_CAL_START,
        "imageCalRunning": sx127x.IMAGE_CAL_RUNNING,
        "syncWordPublic": sx127x.SYNC_WORD_PUBLIC,
        "syncWordPrivate": sx127x.SYNC_WORD_PRIVATE,
        "lnaBoosted": sx127x.LNA_BOOSTED,
        "tcxoInputOn": sx127x.TCXO_INPUT_ON,
    } == vector["constants"]
    for name, bits in vector["irqFlags"].items():
        assert int(sx127x.Irq[upper(name)]) == bits, name
    for entry in vector["modes"]:
        mode = sx127x.Mode[upper(entry["mode"])]
        assert sx127x.lora_op_mode(mode) == entry["lora"], entry
        assert sx127x.fsk_op_mode(mode) == entry["fsk"], entry
        assert sx127x.mode_from_op_mode(entry["lora"]) is mode, entry
    for entry in vector["addresses"]:
        assert sx127x.read_address(entry["address"]) == entry["read"], entry
        assert sx127x.write_address(entry["address"]) == entry["write"], entry
    for entry in vector["frequencyWords"]:
        assert sx127x.frequency_word(entry["frequencyHz"]) == entry["word"], entry
    for entry in vector["modems"]:
        link = _link_of(links[entry["link"]])
        assert sx127x.symbol_timeout(link, 100_000) == entry["symbolTimeout"], entry
        modem = sx127x.modem(link, entry["frequencyHz"], entry["symbolTimeout"])
        assert modem.modem_config_1 == entry["modemConfig1"], entry
        assert modem.modem_config_2 == entry["modemConfig2"], entry
        assert modem.modem_config_3 == entry["modemConfig3"], entry
        assert modem.detection_optimize == entry["detectionOptimize"], entry
        assert modem.detection_threshold == entry["detectionThreshold"], entry
    for entry in vector["modemRefusals"]:
        with pytest.raises(PamojaError):
            sx127x.modem(
                lora.link(entry["spreadingFactor"], entry["bandwidthHz"]), entry["frequencyHz"]
            )
    for entry in vector["symbolTimeouts"]:
        link = _link_of(links[entry["link"]])
        assert sx127x.symbol_timeout(link, entry["timeoutUs"]) == entry["symbols"], entry
    for entry in vector["txPowers"]:
        power = sx127x.tx_power(output(entry["output"]), entry["requestedDbm"])
        got = (power.pa_config, power.pa_dac, power.ocp, power.output_dbm)
        assert got == (entry["paConfig"], entry["paDac"], entry["ocp"], entry["outputDbm"]), entry
    for entry in vector["underCeilings"]:
        budget = lora.LinkBudget(
            transmit_antenna_gain_dbi=entry["transmitAntennaGainHundredths"] / 100,
            transmit_cable_loss_db=entry["transmitCableLossHundredths"] / 100,
        )
        power = sx127x.tx_power_under_ceiling(
            output(entry["output"]), budget, entry["ceilingHundredths"] / 100
        )
        got = (power.pa_config, power.pa_dac, power.ocp, power.output_dbm)
        assert got == (entry["paConfig"], entry["paDac"], entry["ocp"], entry["outputDbm"]), entry
    for entry in vector["ocp"]:
        assert sx127x.ocp_register(entry["milliamps"]) == entry["register"], entry
    for entry in vector["invertIq"]:
        assert sx127x.invert_iq(entry["receive"], entry["transmit"]) == entry["register"], entry
    for entry in vector["invertIq2"]:
        assert sx127x.invert_iq_2(entry["inverted"]) == entry["register"], entry
    for entry in vector["highBwOptimize"]:
        optimize = sx127x.high_bw_optimize(lora.link(7, entry["bandwidthHz"]), entry["frequencyHz"])
        assert optimize.optimize_1 == entry["optimize1"], entry
        assert optimize.optimize_2 == entry["optimize2"], entry
    for entry in vector["spuriousReception"]:
        erratum = sx127x.spurious_reception(lora.link(7, entry["bandwidthHz"]))
        assert erratum.automatic_if == entry["automaticIf"], entry
        assert erratum.if_freq_2 == entry["ifFreq2"], entry
        assert erratum.offset_hz == entry["offsetHz"], entry
    for entry in vector["imageCalStart"]:
        assert sx127x.image_cal_start(entry["current"]) == entry["register"], entry
    for entry in vector["automaticIf"]:
        assert sx127x.automatic_if(entry["current"], entry["on"]) == entry["register"], entry
    for entry in vector["packetStatuses"]:
        status = sx127x.packet_status(bytes.fromhex(entry["bytes"]), entry["frequencyHz"])
        assert hundredths(status.rssi_dbm) == entry["rssiHundredths"], entry
        assert hundredths(status.snr_db) == entry["snrHundredths"], entry
        assert hundredths(status.signal_rssi_dbm) == entry["signalRssiHundredths"], entry
    for entry in vector["rssi"]:
        assert hundredths(sx127x.rssi_dbm(entry["byte"], entry["frequencyHz"])) == entry["hundredths"]
    for entry in vector["modemStatuses"]:
        status = sx127x.modem_status(entry["byte"])
        assert status.coding_rate_denominator == entry["codingRateDenominator"], entry
        assert status.clear == entry["clear"], entry
        assert status.header_valid == entry["headerValid"], entry
        assert status.rx_ongoing == entry["rxOngoing"], entry
        assert status.signal_synchronized == entry["signalSynchronized"], entry
        assert status.signal_detected == entry["signalDetected"], entry
    for entry in VECTORS["radios"]["llcc68"]:
        link = lora.link(entry["spreadingFactor"], entry["bandwidthHz"])
        assert radios.sx126x.llcc68_supports(link) == entry["supported"], entry


def test_lora_region_vectors_match():
    vector = VECTORS["loraRegions"]

    codes = lora.ChannelPlan.regions()
    assert [want["code"] for want in vector["published"]] == codes, (
        "every published region is described, in order"
    )
    for want in vector["published"]:
        _check_plan(lora.plan_for(want["code"]), want)

    assert [want["code"] for want in vector["cn470"]] == list(lora.CN470_PLANS)
    assert lora.ChannelPlan.cn470_plans() == list(lora.CN470_PLANS)
    for want in vector["cn470"]:
        _check_plan(lora.cn470_plan(want["code"]), want)


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


def test_a_private_fixed_plan_matches_the_same_vectors():
    # A fixed plan in the manner of the 900 MHz ones, with its channel rules set by hand.
    builder = lora.ChannelPlanBuilder("private-fixed")
    builder.data_rate(lora.LoraDataRate.lora(10, 125_000, 980))
    builder.data_rate(lora.LoraDataRate.lora(8, 500_000, 12_500))
    for table in ("uplink_repeater", "uplink_direct"):
        builder.max_payload(lora.LoraMaxPayload(19, 11), table)
        builder.max_payload(lora.LoraMaxPayload(230, 222), table)
    builder.channel_block(lora.LoraChannelBlock(902_300_000, 200_000, 16, 0, 0))
    builder.channel_block(lora.LoraChannelBlock(903_000_000, 1_600_000, 2, 1, 1))
    builder.channel_block(lora.LoraChannelBlock(923_300_000, 600_000, 4, 1, 1), "downlink")
    builder.sub_band(lora.LoraSubBand(902_000_000, 928_000_000, 1000, 30))
    builder.power(30, 2, 10)
    builder.rx(923_300_000, 1, 0)
    builder.rx1_row([1])
    builder.rx1_row([1])
    builder.kind("fixed")
    builder.tx_param_setup(False)
    reserved = lora.LoraMaskControl.reserved()
    builder.mask_controls(
        [
            lora.LoraMaskControl.one_group(0),
            lora.LoraMaskControl.one_group(1),
            reserved,
            reserved,
            reserved,
            reserved,
            lora.LoraMaskControl.all_channels(True, then_group=1),
            lora.LoraMaskControl.all_channels(False, then_group=1),
        ]
    )
    with pytest.raises(ValueError, match="eight mask controls"):
        builder.mask_controls([reserved])
    with pytest.raises(ValueError, match="not a join sequence"):
        builder.join_sequence("sweep")
    builder.join_sequence("octet_passes")
    builder.power_reference("conducted", 6)
    builder.relay_channel(lora.LoraRelayChannel(916_700_000, 918_300_000, 1))

    _check_plan(builder.build(), VECTORS["loraRegions"]["customFixed"])


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
    with pytest.raises(PamojaError):
        verifier.verify(frame)

    for entry in vector["timestamps"]:
        assert (
            mavlink.timestamp_from_unix_micros(entry["unixMicros"]) == entry["timestamp"]
        ), entry["unixMicros"]


def test_mavlink_enum_vectors_match():
    """The dialect's named values read the same in every binding, whole table and lookups."""
    from pamoja import mavlink

    vector = VECTORS["mavlinkEnums"]

    assert mavlink.known_enums() == [described["name"] for described in vector["table"]]
    for described in vector["table"]:
        assert mavlink.enum_is_bitmask(described["name"]) == described["bitmask"]
        assert [list(entry) for entry in mavlink.enum_entries(described["name"])] == described[
            "entries"
        ], described["name"]
    for want in vector["values"]:
        assert mavlink.enum_value(want["entry"]) == want["value"], want["entry"]
    for want in vector["entries"]:
        assert mavlink.enum_entry(want["enumeration"], want["value"]) == want["entry"]
    for want in vector["names"]:
        assert mavlink.enum_names(want["enumeration"], want["value"]) == want["names"]
    for entry in vector["unknownEntries"]:
        with pytest.raises(ValueError):
            mavlink.enum_value(entry)
    for name in vector["unknownEnumerations"]:
        with pytest.raises(ValueError):
            mavlink.enum_entries(name)


def test_mavlink_schema_vectors_match():
    """Field order, offsets, and the seed they imply are pinned across every binding."""
    from pamoja import mavlink

    vector = VECTORS["mavlinkSchema"]

    for entry in vector["fieldTypes"]:
        assert entry["code"] in [member.value for member in mavlink.FieldType], entry["name"]
    assert mavlink.FieldType.UINT32 == 6
    assert mavlink.FieldType.CHAR == 3

    assert len(mavlink.known_messages()) == vector["messageCount"]
    with pytest.raises(ValueError):
        mavlink.schema_for(vector["unknownMessage"]["msgid"])
    with pytest.raises(ValueError):
        mavlink.schema_for(vector["unknownMessage"]["name"])

    for described in vector["shapes"]:
        shape = mavlink.schema_for(described["name"])
        assert shape.id == described["msgid"], described["name"]
        assert shape.crc_extra == described["crcExtra"], described["name"]
        assert shape.wire_len == described["wireLen"], described["name"]

        fields = shape.fields
        assert len(fields) == len(described["fields"]), described["name"]
        for field, want in zip(fields, described["fields"]):
            where = f"{described['name']}.{want['name']}"
            assert field.name == want["name"], where
            assert field.type_name == want["typeName"], where
            assert field.field_type == want["fieldType"], where
            assert field.array_len == want["arrayLen"], where
            assert field.extension == want["extension"], where
            assert field.offset == want["offset"], where

    # A definition written in declaration order lands in wire order and on the
    # published seed, which is what lets a caller transcribe a dialect as it reads.
    for key in ("declared", "private"):
        described = vector[key]
        builder = mavlink.MessageSchemaBuilder(described["msgid"], described["name"])
        for field in described["fields"]:
            builder.field(field["name"], field["fieldType"], field["arrayLen"])
        built = builder.build()
        assert [field.name for field in built.fields] == described["wireOrder"], key
        assert built.crc_extra == described["crcExtra"], key

    # A message filled in by name puts exactly these bytes on the wire.
    filled = vector["filled"]
    shape = mavlink.schema_for(filled["name"])
    built = mavlink.message(shape)
    for entry in filled["values"]:
        built.set(entry["field"], entry["value"])
    assert built.payload.hex() == filled["payload"]

    frame = built.to_frame(mavlink.MavlinkHeader(1, 1, 7))
    assert frame.bytes.hex() == filled["frame"]

    read = mavlink.MavlinkMessage.decode(shape, frame.payload)
    for entry in filled["values"]:
        assert read.get_int(entry["field"]) == entry["value"], entry["field"]

    # A char array carries text padded with zeros.
    text = vector["text"]
    status = mavlink.schema_for(text["name"])
    written = mavlink.from_dict(
        status, {"severity": text["severity"], text["field"]: text["value"]}
    )
    assert written.payload.hex() == text["payload"]
    assert mavlink.to_dict(written, status)[text["field"]] == text["value"]

    # A value an integer field cannot hold exactly is refused rather than truncated.
    report = mavlink.message(shape)
    for refused in vector["refused"]:
        with pytest.raises(PamojaError):
            report.set(refused["field"], refused["value"])


def test_mavlink_protocol_vectors_match():
    """A whole mission upload, command acks, and setpoints are pinned frame by frame."""
    from pamoja import mavlink

    vector = VECTORS["mavlinkProtocol"]
    vehicle = mavlink.MavlinkHeader(**_header(vector["vehicle"]))
    station = mavlink.MavlinkHeader(**_header(vector["station"]))

    def parse(text: str) -> mavlink.MavlinkFrame:
        return mavlink.MavlinkFrame.parse_known(bytes.fromhex(text))

    # The plan's items are built by field name and must match the pinned payloads.
    item_shape = mavlink.schema_for("MISSION_ITEM_INT")
    upload = mavlink.MissionSender(1, 1, 0)
    for fields, want in zip(vector["plan"], vector["planPayloads"]):
        item = mavlink.message(item_shape)
        for entry in fields:
            item.set(entry["field"], entry["value"])
        assert item.payload.hex() == want
        upload.add_item(item.payload)
    assert len(upload) == len(vector["plan"])

    download = mavlink.MissionReceiver(255, 190, 0)
    request_list = download.request_list(station)
    assert request_list.bytes.hex() == vector["requestList"]
    opened = upload.on_frame(request_list, vehicle)
    assert opened.kind == "reply"
    assert opened.reply.bytes.hex() == vector["count"]

    for index, step in enumerate(vector["exchange"]):
        received = download.on_frame(parse(step["feed"]), station)
        assert received.kind == step["receiverKind"], index
        assert (received.accepted is not None) == step["accepted"], index
        if received.accepted is not None:
            assert received.accepted.get_int("seq") == step["acceptedSeq"], index
        assert received.reply.bytes.hex() == step["reply"], index

        answered = upload.on_frame(received.reply, vehicle)
        assert answered.kind == step["senderKind"], index
        if answered.kind == "reply":
            assert answered.reply.bytes.hex() == step["senderReply"], index
        else:
            assert answered.result == step["senderResult"], index
    assert download.complete

    # A request past the end of the plan is refused with the published result.
    overrun = vector["overrun"]
    refusal = upload.on_frame(parse(overrun["request"]), station)
    assert refusal.reply.bytes.hex() == overrun["reply"]
    ack = mavlink.MavlinkMessage.decode(mavlink.schema_for("MISSION_ACK"), refusal.reply.payload)
    assert ack.get_int("type") == overrun["result"]

    # The command protocol classifies acknowledgments and counts retries.
    command = vector["command"]
    arm = mavlink.CommandProtocol(command["command"], command["maxRetries"])
    kinds = {"unrelated": "unrelated", "inProgress": "in_progress", "final": "final"}
    for described in command["acks"]:
        outcome = arm.on_frame(parse(described["frame"]))
        assert outcome.kind == kinds[described["kind"]]
        assert outcome.value == described["value"]
    for want in command["timeouts"]:
        assert arm.on_timeout() == want

    # A frame none of the machines handle is passed over by all of them.
    ignored = parse(vector["ignored"])
    assert upload.on_frame(ignored, station) is None
    assert download.on_frame(ignored, station) is None
    assert arm.on_frame(ignored) is None

    # Setpoints put exactly these bytes on the wire.
    offboard = vector["offboard"]
    for entry in offboard["typeMasks"]:
        assert mavlink.type_mask(entry["flags"]) == entry["mask"], entry["flags"]
    local = offboard["localPosition"]
    assert (
        mavlink.local_position(
            station,
            local["timeBootMs"],
            local["coordinateFrame"],
            local["targetSystem"],
            local["targetComponent"],
            local["x"],
            local["y"],
            local["z"],
        ).bytes.hex()
        == local["frame"]
    )
    velocity = offboard["localVelocity"]
    assert (
        mavlink.local_velocity(
            station,
            velocity["timeBootMs"],
            velocity["coordinateFrame"],
            velocity["targetSystem"],
            velocity["targetComponent"],
            velocity["vx"],
            velocity["vy"],
            velocity["vz"],
        ).bytes.hex()
        == velocity["frame"]
    )
    global_ = offboard["globalPosition"]
    assert (
        mavlink.global_position(
            station,
            global_["timeBootMs"],
            global_["coordinateFrame"],
            global_["targetSystem"],
            global_["targetComponent"],
            global_["latInt"],
            global_["lonInt"],
            global_["alt"],
        ).bytes.hex()
        == global_["frame"]
    )


def _header(described: dict) -> dict:
    return {
        "system_id": described["systemId"],
        "component_id": described["componentId"],
        "sequence": described["sequence"],
    }

def test_lora_vectors_match():
    vector = VECTORS["lora"]

    for described in vector["links"]:
        link = _link_of(described)
        assert link.symbol_time_us() == described["symbolTimeUs"], described["name"]
        assert (
            link.low_data_rate_optimization() == described["lowDataRateOptimization"]
        ), described["name"]

        for airtime in described["airtimes"]:
            assert link.airtime_us(airtime["payloadLen"]) == airtime["airtimeUs"], (
                f"airtime of {airtime['payloadLen']} bytes on {described['name']}"
            )

        for budget in described["budgets"]:
            assert (
                link.min_off_time_us(budget["payloadLen"], budget["permille"])
                == budget["offTimeUs"]
            ), f"off time at {budget['permille']} permille on {described['name']}"
            assert (
                lora.messages_per_hour(link, budget["payloadLen"], budget["permille"])
                == budget["messagesPerHour"]
            ), f"messages an hour at {budget['permille']} permille on {described['name']}"

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

    edges = vector["edges"]
    zero = edges["zeroBandwidth"]
    unset = lora.link(zero["spreadingFactor"], zero["bandwidthHz"])
    assert unset.bandwidth_hz == zero["usedBandwidthHz"]
    assert unset.symbol_time_us() == zero["symbolTimeUs"]
    assert unset.airtime_us(zero["payloadLen"]) == zero["airtimeUs"]
    past = edges["pastOneFrame"]
    sf12 = _link_of(next(entry for entry in vector["links"] if entry["name"] == past["link"]))
    assert sf12.airtime_us(past["payloadLen"]) == past["airtimeUs"]
    for whole in edges["wholeTime"]:
        assert sf12.min_off_time_us(whole["payloadLen"], whole["permille"]) == whole["offTimeUs"]
        assert (
            lora.messages_per_hour(sf12, whole["payloadLen"], whole["permille"])
            == whole["messagesPerHour"]
        )


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
    _assert_routes(router, vector["routes"])

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

    full = vector["full"]
    full_table = routing.router(0x01, full["capacity"])
    for observation in full["observations"]:
        assert (
            full_table.observe(observation["origin"], observation["via"], observation["cost"])
            == observation["changed"]
        ), f"a full table observing {observation['origin']}"
    _assert_routes(full_table, full["routes"])
    for want in full["decisions"]:
        _assert_decision(full_table, want)

    itself = vector["itself"]
    own = routing.router(0x01, 4)
    assert own.observe(0x01, itself["via"], itself["cost"]) == itself["changed"]
    assert len(own) == itself["learned"]
    _assert_decision(own, itself["decision"])

    empty = vector["empty"]
    none = routing.router(0x01, empty["capacity"])
    assert none.observe(empty["origin"], empty["via"], empty["cost"]) == empty["changed"]
    for want in empty["decisions"]:
        _assert_decision(none, want)
    with pytest.raises(OverflowError):
        routing.router(0x01, -1)


def _assert_routes(router, want):
    held = [
        {"dst": route.dst, "nextHop": route.next_hop, "cost": route.cost}
        for route in router.routes()
    ]
    assert held == want, "the routes held, in order"


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
        assert header.adr_ack_req == want["adrAckReq"]
        assert header.class_b == want["classB"]
        assert header.fopts_len == want["foptsLen"]
        assert header.payload_len == want["payloadLen"]

    with pytest.raises(PamojaError):
        lorawan.parse_header(unhex(vector["unsupported"]))
    with pytest.raises(PamojaError):
        lorawan.parse_header(unhex(vector["truncated"]))


def test_chirpstack_vectors_match():
    vector = VECTORS["chirpstack"]
    for entry in vector["events"]:
        got = gateway.ChirpstackUplinkEvent.from_json(entry["json"])
        best = got.best_reception()
        receptions = got.receptions
        best_index = None
        if best is not None:
            best_index = next(
                index
                for index, reception in enumerate(receptions)
                if (reception.gateway, reception.snr_db) == (best.gateway, best.snr_db)
            )
        assert {
            "deduplicationId": got.deduplication_id,
            "time": got.time,
            "applicationId": got.application_id,
            "deviceName": got.device_name,
            "devEui": got.dev_eui,
            "devAddr": got.dev_addr,
            "adr": got.adr,
            "dataRate": got.data_rate,
            "fcnt": got.fcnt,
            "fport": got.fport,
            "confirmed": got.confirmed,
            "data": got.data.hex(),
            "frequencyHz": got.frequency_hz,
            "receptions": [
                {"gateway": reception.gateway, "rssiDbm": reception.rssi_dbm, "snrDb": reception.snr_db}
                for reception in receptions
            ],
            "bestReception": best_index,
        } == entry["event"]
    for text in vector["refused"]:
        with pytest.raises(PamojaError):
            gateway.ChirpstackUplinkEvent.from_json(text)
    assert gateway.chirpstack_uplink_topic(vector["topic"]["applicationId"]) == vector["topic"]["topic"]
    assert gateway.CHIRPSTACK_UPLINK_TOPIC == vector["allApplications"]


def test_lorawan_link_defaults_match():
    d = VECTORS["lorawanLink"]["defaults"]
    assert (
        lorawan.RECEIVE_DELAY1_US,
        lorawan.RECEIVE_DELAY2_US,
        lorawan.JOIN_ACCEPT_DELAY1_US,
        lorawan.JOIN_ACCEPT_DELAY2_US,
        lorawan.RECEIVE_WINDOW_TOLERANCE_US,
        lorawan.MAX_FCNT_GAP,
        lorawan.ADR_ACK_LIMIT,
        lorawan.ADR_ACK_DELAY,
        lorawan.RETRANSMIT_TIMEOUT_MIN_US,
        lorawan.RETRANSMIT_TIMEOUT_MAX_US,
    ) == (
        d["receiveDelay1Us"],
        d["receiveDelay2Us"],
        d["joinAcceptDelay1Us"],
        d["joinAcceptDelay2Us"],
        d["receiveWindowToleranceUs"],
        d["maxFcntGap"],
        d["adrAckLimit"],
        d["adrAckDelay"],
        d["retransmitTimeoutMinUs"],
        d["retransmitTimeoutMaxUs"],
    )


def test_lorawan_backoff_vectors_match():
    for script in VECTORS["lorawanLink"]["backoff"]:
        where = f"{script['version']} limit {script['limit']} delay {script['delay']}"
        backoff = lorawan.Backoff(lorawan.Version(script["version"]), script["limit"], script["delay"])
        assert backoff.version == script["version"], where
        data_rate = script["startDataRate"]
        steps = []
        asked = []
        for sent in range(1, script["uplinks"] + 1):
            step = backoff.uplink(data_rate == 0)
            if step.request_ack:
                asked.append(sent)
            for taken, name in (
                (step.restore_power, "restorePower"),
                (step.lower_data_rate, "lowerDataRate"),
                (step.restore_channels, "restoreChannels"),
            ):
                if taken:
                    steps.append({"uplink": sent, "step": name})
            if step.lower_data_rate:
                data_rate -= 1
        assert steps == script["steps"], where
        assert (asked[0] if asked else None) == script["firstAsked"], where
        assert (asked[-1] if asked else None) == script["lastAsked"], where
        assert backoff.counter == script["counter"], where
        assert data_rate == script["endDataRate"], where
        backoff.downlink()
        after = backoff.uplink(data_rate == 0)
        assert backoff.counter == script["afterDownlink"]["counter"], where
        assert after.request_ack == script["afterDownlink"]["requestAck"], where


def _check_cflist(cflist, want: dict, where: str) -> None:
    """Hold a channel list to the answers every binding must give."""
    assert cflist.bytes.hex() == want["bytes"], where
    assert cflist.kind == want["kind"], where
    assert cflist.type_byte == want["typeByte"], where
    assert cflist.frequencies_hz() == want["frequenciesHz"], where
    assert cflist.channel_mask_groups() == want["channelMaskGroups"], where
    assert cflist.enabled_channels() == want["enabledChannels"], where
    for entry in want["enables"]:
        assert cflist.enables(entry["channel"]) == entry["enabled"], f"{where} {entry}"
    assert lorawan.CfList.from_bytes(unhex(want["bytes"])) == cflist, where


def test_lorawan_cflist_vectors_match():
    lists = VECTORS["lorawanLink"]["cflist"]
    _check_cflist(
        lorawan.CfList.from_frequencies(lists["frequencies"]["input"]),
        lists["frequencies"]["list"],
        "a list of frequencies",
    )
    _check_cflist(
        lorawan.CfList.from_channel_masks(lists["channelMasks"]["input"]),
        lists["channelMasks"]["list"],
        "a list of masks",
    )
    _check_cflist(
        lorawan.CfList.from_bytes(unhex(lists["reserved"]["bytes"])),
        lists["reserved"],
        "a reserved list",
    )
    for hz in lists["refusedFrequencies"]:
        with pytest.raises(PamojaError):
            lorawan.CfList.from_frequencies([hz, 0, 0, 0, 0])
    with pytest.raises(PamojaError):
        lorawan.CfList.from_bytes(bytes(15))


def test_lorawan_join_settings_vectors_match():
    link = VECTORS["lorawanLink"]
    for want in link["joinAccepts"]:
        device = lorawan.device(unhex(link["device"]["devEui"]), bytes(8), unhex(want["appKey"]))
        assert device.dev_eui.hex() == link["device"]["devEui"]
        accept = device.accept_join(unhex(want["frame"]), want["devNonce"])
        assert accept.dl_settings == want["dlSettings"]
        assert accept.rx_delay == want["rxDelay"]
        assert accept.rx1_dr_offset == want["rx1DrOffset"]
        assert accept.rx2_data_rate == want["rx2DataRate"]
        assert accept.receive_delay_us == want["receiveDelayUs"]
        cflist = accept.cflist
        assert (None if cflist is None else cflist.bytes.hex()) == want["cflist"]

    uplink = next(frame for frame in VECTORS["header"]["frames"] if frame["adrAckReq"])
    frames = VECTORS["lorawan"]
    session = lorawan.session(frames["devAddr"], unhex(frames["nwkSKey"]), unhex(frames["appSKey"]))
    frame = session.encode_uplink(uplink["fcnt"], uplink["fport"], b"x", adr=True, adr_ack_req=True)
    assert frame.hex() == uplink["frame"]
    decoded = session.decode(frame, uplink["fcnt"])
    assert decoded.adr_ack_req is True
    assert decoded.class_b is False


def _device_link_of(link) -> dict:
    """Describe link settings the way the vectors do."""
    return {
        "spreadingFactor": link.spreading_factor,
        "bandwidthHz": link.bandwidth_hz,
        "codingRateDenominator": link.coding_rate_denominator,
        "preambleSymbols": link.preamble_symbols,
        "explicitHeader": link.explicit_header,
        "crc": link.crc,
    }


def _device_window_of(window) -> dict:
    """Describe a receive window the way the vectors do."""
    return {
        "delayUs": window.delay_us,
        "frequencyHz": window.frequency_hz,
        "dataRate": window.data_rate,
        "link": _device_link_of(window.link),
    }


def _device_transmission_of(transmission) -> dict:
    """Describe a transmission the way the vectors do."""
    return {
        "frame": transmission.frame.hex(),
        "frequencyHz": transmission.frequency_hz,
        "dataRate": transmission.data_rate,
        "link": _device_link_of(transmission.link),
        "outputDbm": transmission.output_dbm,
        "airtimeUs": transmission.airtime_us,
        "rx1": _device_window_of(transmission.rx1),
        "rx2": _device_window_of(transmission.rx2),
        "carriesPayload": transmission.carries_payload,
        "relay": None
        if transmission.relay is None
        else _device_exchange_of(transmission.relay),
    }


def _device_exchange_of(exchange) -> dict:
    """Describe a wake-on-radio exchange the way the vectors do."""
    ack = exchange.ack
    return {
        "wakeUp": {
            "frame": exchange.wake_up.frame.hex(),
            "startUs": exchange.wake_up.start_us,
            "frequencyHz": exchange.wake_up.carrier.frequency_hz,
            "dataRate": exchange.wake_up.carrier.data_rate,
            "link": _device_link_of(exchange.wake_up.link),
            "outputDbm": exchange.wake_up.output_dbm,
            "airtimeUs": exchange.wake_up.airtime_us,
        },
        "ack": None
        if ack is None
        else {
            "startUs": ack.start_us,
            "frequencyHz": ack.carrier.frequency_hz,
            "dataRate": ack.carrier.data_rate,
            "link": _device_link_of(ack.link),
            "airtimeUs": ack.airtime_us,
        },
        "uplinkStartUs": exchange.uplink_start_us,
        "rxr": _device_window_of(exchange.rxr),
    }


def _device_relay_status_of(status) -> dict:
    """Describe what a relay said about itself the way the vectors do."""
    coded = {name: coded for coded, name in _RELAY_NAMES.items()}
    return {
        "cadPeriodicity": coded[status.cad_periodicity],
        "xtalAccuracy": coded[status.xtal_accuracy],
        "cadToRx": coded[status.cad_to_rx],
        "relayDataRate": status.relay_data_rate,
        "forward": coded[status.forward],
    }


def _run_device_step(device, step: dict):
    """Make one scripted call on a device and describe what it returned."""
    call = step["call"]
    if call == "join":
        return _device_transmission_of(device.join(step["devNonce"], step["nowUs"]))
    if call == "send":
        return _device_transmission_of(
            device.send(step["port"], unhex(step["payload"]), step["nowUs"], step["confirmed"])
        )
    if call == "sendEmpty":
        return _device_transmission_of(device.send_empty(step["nowUs"]))
    if call == "repeat":
        return _device_transmission_of(device.repeat(step["nowUs"]))
    if call == "heard":
        window = None if step["window"] is None else lorawan.ReceiveWindow(step["window"])
        heard = device.heard(unhex(step["frame"]), step["snrDb"], window)
        if heard.kind == "joined":
            return {"kind": "joined", "devAddr": heard.dev_addr}
        delivery = heard.delivery
        check = delivery.link_check
        time = delivery.device_time
        return {
            "kind": "data",
            "devAddr": heard.dev_addr,
            "delivery": {
                "port": delivery.port,
                "payload": delivery.payload.hex(),
                "acknowledged": delivery.acknowledged,
                "confirmed": delivery.confirmed,
                "morePending": delivery.more_pending,
                "linkCheck": None if check is None else {"marginDb": check[0], "gateways": check[1]},
                "deviceTime": None if time is None else {"gpsSeconds": time[0], "fraction": time[1]},
            },
        }
    if call == "useRelay":
        return device.use_relay(step["on"])
    if call == "heardWorAck":
        return _device_relay_status_of(device.heard_wor_ack(unhex(step["frame"])))
    if call == "noWorAck":
        nxt = device.no_wor_ack(step["nowUs"])
        return {
            "uplink": nxt.uplink,
            "wakeUp": None if nxt.wake_up is None else _device_exchange_of(nxt.wake_up),
        }
    if call == "relayMode":
        status = device.relay_status
        return {
            "relaying": device.relaying,
            "activation": device.relay_activation,
            "sync": device.relay_sync,
            "worCounter": device.wor_counter,
            "status": None if status is None else _device_relay_status_of(status),
        }
    if call == "nothingHeard":
        next_step = device.nothing_heard(step["nowUs"])
        return {"kind": next_step.kind, "notBeforeUs": next_step.not_before_us}
    if call == "save":
        return device.save(step["nowUs"]).hex()
    if call == "resume":
        device.resume(unhex(step["saved"]), step["nowUs"])
        return True
    if call == "requestLinkCheck":
        device.request_link_check()
        return None
    if call == "requestDeviceTime":
        device.request_device_time()
        return None
    if call == "setBattery":
        battery = step["battery"]
        if battery == "external":
            device.set_battery(external=True)
        elif battery == "unknown":
            device.set_battery()
        else:
            device.set_battery(battery)
        return None
    if call == "status":
        rx2_hz, rx2_rate = device.rx2
        lowest_hz, highest_hz = device.frequency_span
        return {
            "joined": device.is_joined,
            "devAddr": device.dev_addr,
            "dataRate": device.data_rate,
            "fcntUp": device.fcnt_up,
            "fcntDown": device.fcnt_down,
            "transmissions": device.transmissions,
            "rx2": {"frequencyHz": rx2_hz, "dataRate": rx2_rate},
            "receiveDelayUs": device.receive_delay_us,
            "frequencySpan": {"lowestHz": lowest_hz, "highestHz": highest_hz},
            "channels": [
                {
                    "index": channel.index,
                    "uplinkHz": channel.uplink_hz,
                    "downlinkHz": channel.downlink_hz,
                    "minDataRate": channel.min_data_rate,
                    "maxDataRate": channel.max_data_rate,
                }
                for channel in device.channels()
            ],
        }
    raise AssertionError(f"no step {call}")


_RELAY_NAMES = {
    "Ms1000": "ms1000",
    "Ms500": "ms500",
    "Ms250": "ms250",
    "Ms100": "ms100",
    "Ms50": "ms50",
    "Ms20": "ms20",
    "Symbols2": "symbols2",
    "Symbols4": "symbols4",
    "Symbols6": "symbols6",
    "Symbols8": "symbols8",
    "Ppm10": "ppm10",
    "Ppm20": "ppm20",
    "Ppm30": "ppm30",
    "Ppm40": "ppm40",
    "Available": "available",
    "RetryIn30Minutes": "retry_in_30_minutes",
    "RetryIn60Minutes": "retry_in_60_minutes",
    "Disabled": "disabled",
    "Default": "default",
    "Second": "second",
}


def _relay_state(entry: dict):
    return relay.state_sync(
        _RELAY_NAMES[entry["cadToRx"]],
        _RELAY_NAMES[entry["forward"]],
        entry["relayDataRate"],
        _RELAY_NAMES[entry["xtalAccuracy"]],
        _RELAY_NAMES[entry["cadPeriodicity"]],
        entry["tOffsetMs"],
    )


def _relay_carrier(entry: dict):
    return relay.Carrier(entry["frequencyHz"], entry["dataRate"])


def test_lorawan_relay_vectors_match():
    """The Things Stack's root key, Basics Modem's WOR frames, and TS011-1.0.1 appendix 1."""
    vector = VECTORS["lorawanRelay"]
    assert {
        "laFportRelay": relay.LA_FPORT_RELAY,
        "trustedEdNumber": relay.TRUSTED_ED_NUMBER,
        "worAttemptsWoAck": relay.WOR_ATTEMPTS_WO_ACK,
        "worDataDelayUs": relay.WOR_DATA_DELAY_US,
        "worAckDelayUs": relay.WOR_ACK_DELAY_US,
        "relayFwdDelayUs": relay.RELAY_FWD_DELAY_US,
        "rxrDelayUs": relay.RXR_DELAY_US,
        "forwardOverhead": relay.FORWARD_OVERHEAD,
        "minWorPreambleSymbols": relay.MIN_WOR_PREAMBLE_SYMBOLS,
    } == vector["constants"]

    assert relay.root_wor_s_key(unhex(vector["rootKey"]["networkKey"])).hex() == vector["rootKey"]["rootWorSKey"]
    own = vector["session"]
    device = lorawan.session(own["devAddr"], unhex(own["nwkSKey"]), unhex(own["appSKey"]))
    assert device.root_wor_s_key().hex() == own["rootWorSKey"]
    keys = device.wor_keys()
    assert keys.integrity.hex() == own["integrity"]
    assert keys.encryption.hex() == own["encryption"]
    assert relay.wor_keys(unhex(own["rootWorSKey"]), own["devAddr"]) == keys

    for entry in vector["worUplinks"]:
        uplink, wor = _relay_carrier(entry["uplink"]), _relay_carrier(entry["wor"])
        frame = relay.wor_uplink(keys, own["devAddr"], entry["wfcnt"], uplink, wor)
        assert frame.hex() == entry["frame"]
        read = relay.parse_wor(frame)
        assert (read.kind, read.dev_addr, read.wfcnt, read.uplink) == (
            relay.WorKind.UPLINK,
            own["devAddr"],
            entry["wfcnt"] & 0xFFFF,
            None,
        )
        assert relay.open_wor(frame, keys, entry["wfcnt"], wor) == uplink
        with pytest.raises(PamojaError):
            relay.open_wor(frame, keys, entry["wfcnt"] + 0x10000, wor)
    join = vector["worJoinRequest"]
    frame = relay.wor_join_request(_relay_carrier(join["uplink"]))
    assert frame.hex() == join["frame"]
    read = relay.parse_wor(frame)
    assert (read.kind, read.uplink) == (relay.WorKind.JOIN_REQUEST, _relay_carrier(join["uplink"]))
    for refused in vector["refusedWors"]:
        with pytest.raises(PamojaError):
            relay.parse_wor(unhex(refused))

    for entry in vector["worAcks"]:
        ack, uplink = _relay_carrier(entry["ack"]), _relay_carrier(entry["uplink"])
        state = _relay_state(entry["state"])
        frame = relay.wor_ack(keys, own["devAddr"], entry["wfcnt"], ack, uplink, state)
        assert frame.hex() == entry["frame"]
        assert relay.open_wor_ack(frame, keys, own["devAddr"], entry["wfcnt"], ack, uplink) == state

    for entry in vector["forwarded"]:
        metadata = entry["metadata"]
        payload = relay.encode_forward(
            relay.UplinkMetadata(
                _RELAY_NAMES[metadata["worChannel"]], metadata["rssiDbm"], metadata["snrDb"], metadata["dataRate"]
            ),
            entry["frequencyHz"],
            unhex(entry["phyPayload"]),
        )
        assert payload.hex() == entry["payload"]
        read = relay.parse_forward(payload)
        back = entry.get("readBack", metadata)
        assert (read.metadata.wor_channel, read.metadata.rssi_dbm, read.metadata.snr_db, read.metadata.data_rate) == (
            _RELAY_NAMES[back["worChannel"]],
            back["rssiDbm"],
            back["snrDb"],
            back["dataRate"],
        )
        assert (read.frequency_hz, read.phy_payload.hex()) == (entry["frequencyHz"], entry["phyPayload"])

    for entry in vector["unsynchronizedPreambles"]:
        assert (
            relay.unsynchronized_preamble(
                _RELAY_NAMES[entry["cadPeriodicity"]], entry["symbolUs"], _RELAY_NAMES[entry["cadToRx"]]
            )
            == entry["symbols"]
        )
    for entry in vector["tOffsets"]:
        assert (
            relay.t_offset_ms(entry["scanStartUs"], entry["preambleEndUs"]) == entry["offsetMs"]
        )
    timing = vector["synchronization"]
    sync = relay.synchronization(
        timing["worStartUs"], timing["preambleSymbols"], timing["symbolUs"], _relay_state(timing["state"])
    )
    assert sync.reference_us == timing["referenceUs"]
    for entry in timing["slots"]:
        slot = relay.next_wor(sync, entry["nowUs"], entry["deviceXtalPpm"], entry["symbolUs"], entry["otherChannel"])
        got = None if slot is None else {"startUs": slot.start_us, "preambleSymbols": slot.preamble_symbols}
        assert got == entry["slot"], entry["nowUs"]
    for entry in vector["secondChannels"]:
        channel = relay.second_channel(entry["index"], entry["dataRate"], entry["ackOffset"], entry["frequencyHz"])
        got = (
            None
            if channel is None
            else {
                "worFrequencyHz": channel.wor_frequency_hz,
                "ackFrequencyHz": channel.ack_frequency_hz,
                "dataRate": channel.data_rate,
            }
        )
        assert got == entry["channel"]


def test_lorawan_device_vectors_match():
    vector = VECTORS["lorawanDevice"]
    assert lorawan.SAVED_LEN == vector["savedLen"]
    fields = {
        "join": "transmission",
        "send": "transmission",
        "sendEmpty": "transmission",
        "repeat": "transmission",
        "heard": "heard",
        "nothingHeard": "next",
        "save": "saved",
        "resume": "resumed",
        "status": "status",
        "useRelay": "taken",
        "heardWorAck": "relayStatus",
        "noWorAck": "worNext",
        "relayMode": "relayMode",
    }
    for script in vector["scripts"]:
        plan_name = script["plan"]
        plan = (
            lora.plan_for(plan_name["region"])
            if "region" in plan_name
            else lora.cn470_plan(plan_name["cn470"])
        )
        settings_vector = script["settings"]
        settings = lorawan.DeviceSettings(
            settings_vector["minOutputDbm"],
            settings_vector["maxOutputDbm"],
            version=settings_vector["version"],
            adr=settings_vector["adr"],
            antenna_gain_db=settings_vector["antennaGainDb"],
            lowest_hz=settings_vector["lowestHz"],
            highest_hz=settings_vector["highestHz"],
            regional_duty_cycle=settings_vector["regionalDutyCycle"],
            behind_repeater=settings_vector["behindRepeater"],
            seed=settings_vector["seed"],
        )
        counters = script["counters"] or {"up": 0, "down": None}
        activation = script["activation"]
        if "overTheAir" in activation:
            keys = activation["overTheAir"]
            device = lorawan.end_device(
                plan,
                unhex(keys["devEui"]),
                unhex(keys["joinEui"]),
                unhex(keys["appKey"]),
                settings,
                counters["up"],
                counters["down"],
            )
        else:
            keys = activation["personalized"]
            device = lorawan.EndDevice.personalized(
                plan,
                lorawan.session(keys["devAddr"], unhex(keys["nwkSKey"]), unhex(keys["appSKey"])),
                settings,
                counters["up"],
                counters["down"],
            )

        for index, step in enumerate(script["steps"]):
            where = f"step {index} ({step['call']}) of {script['name']}"
            if "error" in step:
                with pytest.raises(lorawan.DeviceError) as raised:
                    _run_device_step(device, step)
                error = raised.value
                assert {
                    "kind": error.kind,
                    "untilUs": error.until_us,
                    "max": error.max,
                    "dataRate": error.data_rate,
                    "state": error.state,
                    "format": error.format,
                } == step["error"], where
                continue
            got = _run_device_step(device, step)
            field = fields.get(step["call"])
            if field is not None:
                assert got == step[field], where


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

    audit.verify_chain(keeper.public_key, entries)
    with pytest.raises(PamojaError):
        audit.verify_chain(
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

    assert plan.hysteresis == pytest.approx(vector["plan"]["hysteresis"], abs=TOLERANCE)
    walk = vector["walk"]
    for governor, want in ((plan, walk["modes"]), (plan.with_hysteresis(0), walk["flatModes"])):
        mode = walk["start"]
        for at, soc in enumerate(walk["charges"]):
            mode = governor.next_mode_while_charging(mode, soc, walk["charging"][at])
            assert mode == want[at], f"step {at} of the walk, at {soc}"

    duty = power.DutyCycle.from_fraction(
        vector["duty"]["periodUs"], vector["duty"]["fraction"]
    )
    assert duty.active_us == vector["duty"]["activeUs"]
    assert duty.sleep_us == vector["duty"]["sleepUs"]

    tenth_want = vector["tenth"]
    tenth = power.DutyCycle.from_fraction(tenth_want["periodUs"], tenth_want["fraction"])
    assert tenth.active_us == tenth_want["activeUs"]
    assert tenth.sleep_us == tenth_want["sleepUs"]
    assert tenth.period_us == tenth_want["periodUs"]

    unreadable = vector["unreadable"]
    assert plan.mode(float("nan")) == unreadable["mode"]
    assert plan.mode_while_charging(float("nan"), True) == unreadable["charging"]
    assert plan.interval_us(float("nan")) == unreadable["intervalUs"]
    asleep = power.DutyCycle.from_fraction(vector["duty"]["periodUs"], float("nan"))
    assert asleep.active_us == unreadable["dutyActiveUs"]
    assert asleep.sleep_us == unreadable["dutySleepUs"]


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

    from pamoja import core, ladder, loopback, sync

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
            core.Transport.faulty(
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
    elif want["kind"] == "Custom":
        assert policy.custom_kind == want["customKind"]
        assert policy.params == want["params"]


def _assert_reactions(control, reactions: list[dict]) -> None:
    """Walks a controller through a recorded run and checks every decision.

    A reading JSON cannot hold, such as NaN, is written as text.
    """
    for want in reactions:
        reading = float(want["reading"])
        reaction = control.evaluate(reading)
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
        elif kind == "Custom":
            assert reaction.alert.code == want["alert"]["code"]
            assert reaction.alert.value == pytest.approx(want["alert"]["value"], abs=TOLERANCE)
        elif kind == "ChangingFast":
            assert reaction.alert.rate == pytest.approx(
                want["alert"]["rate"], abs=TOLERANCE
            )
        elif kind == "InvalidReading":
            assert math.isnan(reaction.alert.reading) == math.isnan(reading)


def test_rules_vectors_match():
    vector = VECTORS["rules"]
    evaluator = profile.RuleEvaluator.from_json(vector["file"])
    assert evaluator.topics == vector["topics"]
    assert evaluator.actuators == vector["actuators"]

    def written(action):
        """Writes an action back in the rule file's own shape, to compare with the vector."""
        if action.kind == profile.RuleActionKind.DRIVE:
            return {"do": "drive", "actuator": action.actuator, "on": action.on}
        return {"do": "publish", "topic": action.topic, "payload": action.payload}

    for step in vector["steps"]:
        fired = [
            {"rule": one.rule, "edge": one.edge, "actions": [written(a) for a in one.actions]}
            for one in evaluator.evaluate(step["topic"], step["reading"])
        ]
        assert fired == step["fired"], f"what {step['topic']} at {step['reading']} fired"
    for state in vector["states"]:
        assert evaluator.is_set(state["rule"]) == state["set"]
    assert evaluator.is_set("nowhere") is None

    for refused in vector["refused"]:
        with pytest.raises(PamojaError, match=re.escape(refused["reason"])):
            profile.RuleEvaluator.from_json(refused["file"])

    invalid = vector["notANumber"]
    with pytest.raises(PamojaError, match=re.escape(invalid["reason"])):
        evaluator.evaluate(invalid["topic"], float("nan"))
    assert evaluator.evaluate(invalid["unwatched"], float("nan")) == []
    assert json.loads(profile.RuleEvaluator.from_json(evaluator.to_json()).to_json()) == json.loads(
        evaluator.to_json()
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
    assert fridge.power.hysteresis == pytest.approx(
        cold_chain["power"]["hysteresis"], abs=TOLERANCE
    )
    _assert_reactions(fridge.controller(), cold_chain["reactions"])

    plan = fridge.power_plan()
    for charge in cold_chain["plan"]:
        assert plan.mode(charge["soc"]) == charge["mode"]
        assert plan.interval_us(charge["soc"]) == charge["intervalUs"]

    failed = vector["failedProbe"]["control"]
    heater = profile.Controller.setpoint(
        failed["setpoint"], failed["hysteresis"], failed["cooling"], failed["safeBand"]
    )
    _assert_reactions(heater, vector["failedProbe"]["reactions"])

    for case in vector["refused"]:
        with pytest.raises(PamojaError, match=re.escape(case["reason"])):
            profile.Profile.from_json(case["manifest"])

    draining = vector["draining"]
    well = profile.Profile.well_level()
    assert well.name == draining["name"]
    _assert_control(well.control, draining["control"])
    _assert_reactions(well.controller(), draining["reactions"])

    observed = profile.Controller.monitor().evaluate(vector["observed"]["reading"])
    assert observed.actuator is None, "a monitoring profile drives no output"
    assert observed.alert is None
    assert vector["observed"]["alert"]["kind"] == "None"

    custom = vector["custom"]
    orchard = profile.Profile.from_json(custom["manifest"])
    assert orchard.name == custom["name"]
    _assert_control(orchard.control, custom["control"])
    _assert_reactions(orchard.controller(), custom["reactions"])

    for built in vector["built"]:
        want = built["control"]
        control = profile.ControlPolicy(
            want["kind"],
            setpoint=want.get("setpoint"),
            hysteresis=want.get("hysteresis"),
            cooling=want.get("cooling"),
            safe_band=want.get("safeBand"),
            custom_kind=want.get("customKind"),
            params=want.get("params"),
        )
        power = built["power"]
        schedule = profile.PowerScheduleSpec(
            power["activeSecs"],
            power["saverSecs"],
            power["criticalSecs"],
            saver_below=power["saverBelow"],
            critical_below=power["criticalBelow"],
            hysteresis=power["hysteresis"],
        )
        made = profile.Profile(built["name"], built["topic"], control, schedule)
        assert made.to_json() == built["manifest"], f"the manifest {built['name']} writes"


def test_ros2_vectors_match():
    vector = VECTORS["ros2"]

    for want in vector["names"]:
        assert ros2.is_valid_name(want["name"]) == want["valid"]
        assert ros2.is_fully_qualified(want["name"]) == want["fullyQualified"]

    for want in vector["ddsTopics"]:
        assert ros2.dds_topic(want["fqn"], want["kind"]) == want["topic"]

    for kind, prefix in vector["prefixes"].items():
        assert ros2.prefix_for(kind) == prefix
    for kind, suffix in vector["suffixes"].items():
        assert ros2.suffix_for(kind) == suffix

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

    for want in vector["joined"]:
        assert zenoh.join(want["prefix"], want["suffix"]) == want["joined"]

    for want in vector["matches"]:
        assert zenoh.matches(want["pattern"], want["key"]) == want["matches"]

    for want in vector["relations"]:
        assert zenoh.intersects(want["a"], want["b"]) == want["intersects"]
        assert zenoh.intersects(want["b"], want["a"]) == want["intersects"]
        assert zenoh.includes(want["a"], want["b"]) == want["includes"]


def test_gateway_network_vectors_match():
    vector = VECTORS["gatewayNetwork"]
    dr = lora.link(vector["spreadingFactor"], vector["bandwidthHz"])

    site = gateway.Network(
        lora.plan_for("EU868"), vector["netId"], first_dev_addr=vector["devAddr"]
    )
    site.register(unhex(vector["devEui"]), unhex(vector["appEui"]), unhex(vector["appKey"]))

    joiner = lorawan.device(
        unhex(vector["devEui"]), unhex(vector["appEui"]), unhex(vector["appKey"])
    )
    request = joiner.join_request(vector["devNonce"])
    assert request.hex() == vector["join"]["request"]

    joined = site.uplink(
        gateway.Rxpk(
            vector["frequencyHz"], request, link=dr, timestamp_us=vector["join"]["heardAtUs"]
        )
    )
    assert joined.outcome == "joined"
    assert joined.dev_addr == vector["devAddr"]
    assert joined.accept.payload.hex() == vector["join"]["accept"]
    assert joined.accept.timestamp_us == vector["join"]["timestampUs"]

    granted = joiner.accept_join(joined.accept.payload, vector["devNonce"])
    sent = granted.session().encode_uplink(
        vector["uplink"]["fcnt"], vector["uplink"]["fport"], vector["uplink"]["payload"].encode()
    )
    assert sent.hex() == vector["uplink"]["frame"]

    carried = site.uplink(
        gateway.Rxpk(
            vector["frequencyHz"], sent, link=dr, timestamp_us=vector["uplink"]["heardAtUs"]
        )
    )
    assert carried.outcome == "data"
    assert carried.payload.decode() == vector["uplink"]["payload"]
    assert carried.slot.timestamp_us == vector["uplink"]["slotTimestampUs"]

    answer = site.answer(carried.dev_addr, carried.slot, vector["uplink"]["fport"], b"ok")
    assert answer.payload.hex() == vector["downlink"]["frame"]
    assert answer.timestamp_us == vector["downlink"]["timestampUs"]


def test_station_vectors_match():
    vector = VECTORS["station"]
    levels = gateway.StationLevels(rctx=0, xtime=1_000_000, rssi=-35.0, snr=5.1)

    # The station names itself the way the protocol prefers, and reads any form back.
    assert gateway.station_id6(vector["router"]) == vector["routerId6"]
    assert gateway.station_eui_of(vector["routerId6"]) == vector["router"]
    assert gateway.station_discovery(vector["router"]) == vector["discovery"]

    routed = gateway.station_router_parse(vector["routerAnswer"])
    assert routed.router == vector["router"]
    assert routed.muxs == vector["muxs"]
    assert routed.uri == vector["uri"]

    # The server side of the same exchange: it reads who asked, and answers or refuses.
    assert gateway.station_discovery_parse(vector["discovery"]) == vector["router"]
    answer = gateway.station_router_accepted(vector["router"], vector["muxs"], vector["uri"])
    assert answer == vector["routerAnswer"]
    refusal = vector["refusal"]
    assert gateway.station_router_refused(vector["router"], refusal["error"]) == refusal["json"]
    assert gateway.station_router_parse(refusal["json"]).error == refusal["error"]

    # A station clock value is built from its parts and taken apart again, to the microsecond.
    clock = vector["clock"]
    built = gateway.station_xtime(clock["unit"], clock["session"], clock["micros"])
    assert built == int(clock["value"])
    parts = gateway.station_xtime_parts(built)
    assert (parts.unit, parts.session, parts.micros) == (
        clock["unit"],
        clock["session"],
        clock["micros"],
    )

    # A join request the radio heard, split into the fields the protocol names.
    join = gateway.station_heard(
        bytes.fromhex(vector["join"]["frame"]),
        vector["dataRate"],
        vector["frequencyHz"],
        levels,
    )
    assert join.msgtype == "jreq"
    assert join.join_eui == vector["join"]["joinEui"]
    assert join.dev_eui == vector["join"]["devEui"]
    assert join.dev_nonce == vector["join"]["devNonce"]
    assert join.mic == vector["join"]["mic"]
    assert gateway.station_encode(join) == vector["join"]["message"]

    # Then a data frame, whose payload stays encrypted as it passes through.
    uplink = gateway.station_heard(
        bytes.fromhex(vector["uplink"]["frame"]),
        vector["dataRate"],
        vector["frequencyHz"],
        levels,
    )
    assert uplink.msgtype == "updf"
    assert uplink.dev_addr == vector["uplink"]["devAddr"]
    assert uplink.fcnt == vector["uplink"]["fcnt"]
    assert uplink.fport == vector["uplink"]["fport"]
    assert uplink.payload.hex() == vector["uplink"]["payload"]
    assert uplink.mic == vector["uplink"]["mic"]
    assert gateway.station_encode(uplink) == vector["uplink"]["message"]

    # What arrives on the websocket reads back into the same fields.
    read = gateway.station_parse(vector["uplink"]["message"])
    assert read.dev_addr == vector["uplink"]["devAddr"]
    assert read.fcnt == vector["uplink"]["fcnt"]

    # Every kind of message reads and writes back unchanged, so no field is dropped either way.
    for wanted in vector["messages"]:
        message = gateway.station_parse(wanted["json"])
        kind = wanted["kind"]
        assert message.msgtype == kind
        assert gateway.station_encode(message) == wanted["json"], kind
        if "xtime" in wanted:
            xtime = message.levels.xtime if message.levels else message.xtime
            assert xtime == int(wanted["xtime"]), kind
        if "dataRates" in wanted:
            read_rates = [
                None
                if rate is None
                else {
                    "spreadingFactor": rate.spreading_factor,
                    "bandwidthHz": rate.bandwidth_hz,
                    "downlinkOnly": rate.downlink_only,
                }
                for rate in message.data_rates
            ]
            assert read_rates == wanted["dataRates"], kind
        if "filtersNetworks" in wanted:
            assert (message.net_id is not None) == wanted["filtersNetworks"], kind
        if "netIds" in wanted:
            assert message.net_id == wanted["netIds"], kind
        if "joinEuiRanges" in wanted:
            ranges = [[bounds.first, bounds.last] for bounds in message.join_eui_ranges]
            assert ranges == wanted["joinEuiRanges"], kind
        for field, window in (("rx1", "rx1"), ("rx2", "rx2"), ("ping_slot", "pingSlot")):
            if window in wanted:
                read_window = getattr(message, field)
                assert {
                    "dataRate": read_window.data_rate,
                    "frequencyHz": read_window.frequency_hz,
                } == wanted[window], kind
        if "gpstime" in wanted:
            assert message.gpstime == wanted["gpstime"], kind


def test_gateway_vectors_match():
    vector = VECTORS["gateway"]
    heard = vector["pushData"]["rxpk"]
    reported = vector["pushData"]["stat"]
    link = lora.link(
        heard["spreadingFactor"],
        heard["bandwidthHz"],
        coding_rate_denominator=heard["codingRateDenominator"],
    )

    push = gateway.encode(
        gateway.Packet(
            gateway.PacketKind.PUSH_DATA,
            vector["pushData"]["token"],
            gateway=vector["gateway"],
            packets=[
                gateway.Rxpk(
                    heard["frequencyHz"],
                    heard["payload"].encode(),
                    link=link,
                    rssi_dbm=heard["rssiDbm"],
                    snr_db=heard["snrDb"],
                    timestamp_us=heard["timestampUs"],
                    received_at_us=heard["receivedAtUs"],
                    channel=heard["channel"],
                )
            ],
            status=gateway.Stat(
                time_s=reported["timeS"],
                latitude_deg=reported["latitudeDeg"],
                longitude_deg=reported["longitudeDeg"],
                altitude_m=reported["altitudeM"],
                received=reported["received"],
                received_ok=reported["receivedOk"],
                forwarded=reported["forwarded"],
                acknowledged_percent=reported["acknowledgedPercent"],
                downlinks=reported["downlinks"],
                transmitted=reported["transmitted"],
            ),
        )
    )
    assert push.hex() == vector["pushData"]["datagram"]

    read = gateway.parse(push)
    assert read.gateway == vector["gateway"]
    assert read.packets[0].frequency_hz == heard["frequencyHz"]
    assert read.packets[0].payload == heard["payload"].encode()
    assert read.packets[0].link.spreading_factor == heard["spreadingFactor"]
    assert read.status.altitude_m == reported["altitudeM"]
    assert gateway.encode(gateway.acknowledgment(read)).hex() == vector["pushAck"]

    pull = gateway.Packet(gateway.PacketKind.PULL_DATA, 0x0304, gateway=vector["gateway"])
    assert gateway.encode(pull).hex() == vector["pullData"]
    assert (
        gateway.encode(gateway.Packet(gateway.PacketKind.PULL_ACK, 0x0304)).hex()
        == vector["pullAck"]
    )

    asked = vector["pullResp"]["txpk"]
    downlink = gateway.encode(
        gateway.Packet(
            gateway.PacketKind.PULL_RESP,
            vector["pullResp"]["token"],
            transmit=gateway.Txpk(
                asked["frequencyHz"],
                asked["payload"].encode(),
                link=lora.link(asked["spreadingFactor"], asked["bandwidthHz"]),
                timestamp_us=asked["timestampUs"],
                power_dbm=asked["powerDbm"],
                invert_polarity=asked["invertPolarity"],
                without_crc=asked["withoutCrc"],
            ),
        )
    )
    assert downlink.hex() == vector["pullResp"]["datagram"]
    assert gateway.parse(downlink).transmit.power_dbm == asked["powerDbm"]

    refused = gateway.encode(
        gateway.Packet(
            gateway.PacketKind.TX_ACK,
            vector["txAck"]["token"],
            gateway=vector["gateway"],
            tx_status=vector["txAck"]["status"],
        )
    )
    assert refused.hex() == vector["txAck"]["datagram"]
    assert gateway.parse(refused).tx_status == vector["txAck"]["status"]

    for status in vector["statuses"]:
        datagram = gateway.encode(
            gateway.Packet(
                gateway.PacketKind.TX_ACK, 1, gateway=vector["gateway"], tx_status=status
            )
        )
        assert gateway.parse(datagram).tx_status == status


def test_lorawan_mac_command_vectors_match():
    """Every command reads in the direction it names, and writes back the same bytes."""
    vector = VECTORS["lorawan"]["mac"]

    def facing(name: str) -> str:
        return "Downlink" if name == "downlink" else "Uplink"

    # The same bytes are a request going down and an answer coming up, and the two are not
    # the same length, so a reader that guesses the direction walks off the end.
    both = vector["bothDirections"]
    raw = unhex(both["bytes"])
    down = lorawan.mac_parse("Downlink", raw)
    up = lorawan.mac_parse("Uplink", raw)
    assert down[0].cid == both["cid"]
    assert up[0].cid == both["cid"]
    assert len(down[0].encode()) == both["downlinkLength"]
    assert len(up[0].encode()) == both["uplinkLength"]

    for entry in vector["commands"]:
        raw = unhex(entry["bytes"])
        read = lorawan.mac_parse(facing(entry["direction"]), raw)
        assert len(read) == 1, entry["bytes"]
        assert read[0].cid == entry["cid"], entry["bytes"]
        assert read[0].encode() == raw, entry["bytes"]

    sequence = vector["sequence"]
    read = lorawan.mac_parse(facing(sequence["direction"]), unhex(sequence["bytes"]))
    assert [command.cid for command in read] == sequence["cids"]

    # Nothing says how long an unknown command is, so reading stops rather than guessing.
    stops = vector["stops"]
    read = lorawan.mac_parse(facing(stops["direction"]), unhex(stops["bytes"]))
    assert len(read) == stops["readable"]

    truncated = vector["truncated"]
    read = lorawan.mac_parse(facing(truncated["direction"]), unhex(truncated["bytes"]))
    assert read == []

    for entry in vector["relay"]:
        raw = unhex(entry["bytes"])
        read = lorawan.mac_parse(facing(entry["direction"]), raw)
        assert len(read) == 1, entry["bytes"]
        assert read[0].cid == entry["cid"], entry["bytes"]
        assert read[0].encode() == raw, entry["bytes"]
        for field, want in entry.get("fields", {}).items():
            got = getattr(read[0], re.sub(r"(?<!^)(?=[A-Z])", "_", field).lower())
            assert (got.hex() if isinstance(got, bytes) else got) == want, (field, entry["bytes"])


def test_lorawan_package_vectors_match():
    """The four application layer packages: the key chains, the parity matrix, a whole
    fragmentation session, and one command of each kind read and written back."""
    vector = VECTORS["lorawanPackages"]
    ports = vector["ports"]
    assert clock.PORT == ports["clock"]
    assert fragment.PORT == ports["fragment"]
    assert multicast.PORT == ports["multicast"]
    assert firmware.PORT == ports["firmware"]
    assert fragment.MAX_FRAGMENTS == vector["maxFragments"]

    mc = vector["multicast"]
    root_key = unhex(mc["rootKey"])
    assert multicast.root_key(root_key).hex() == mc["mcRootKey"]
    assert multicast.root_key(root_key, True).hex() == mc["mcRootKey11"]
    ke_key = multicast.ke_key(unhex(mc["mcRootKey"]))
    assert ke_key.hex() == mc["mcKeKey"]
    assert multicast.wrap_key(ke_key, unhex(mc["groupKey"])).hex() == mc["wrapped"]
    assert multicast.key(ke_key, unhex(mc["wrapped"])).hex() == mc["groupKey"]
    assert multicast.app_s_key(unhex(mc["groupKey"]), mc["mcAddr"]).hex() == mc["mcAppSKey"]
    assert multicast.nwk_s_key(unhex(mc["groupKey"]), mc["mcAddr"]).hex() == mc["mcNwkSKey"]

    frag = vector["fragment"]
    for step in frag["prbs23"]:
        assert fragment.prbs23(step["x"]) == step["next"]
    for line in frag["parityLines"]:
        assert fragment.parity_line(line["coded"], line["nbFrag"]) == line["fragments"]
    for shape in frag["sessions"]:
        cut = fragment.session(shape["blockLen"], shape["fragSize"])
        assert (cut.nb_frag, cut.padding) == (shape["nbFrag"], shape["padding"])

    block = unhex(frag["block"])
    for piece in frag["fragments"]:
        assert fragment.fragment(block, 32, piece["n"]).hex() == piece["bytes"]

    # The same session put back together over a link that drops every fourth fragment.
    nb_frag = frag["received"]["nbFrag"]
    receiver = fragment.Defragmenter(nb_frag, 32, nb_frag)
    sent = 0
    coded = 0
    for n in range(1, nb_frag * 2 + 1):
        if n % 4 == 0:
            continue
        sent += 1
        if n > nb_frag:
            coded += 1
        if receiver.fragment(n, fragment.fragment(block, 32, n)):
            break
    assert (sent, coded) == (frag["received"]["sent"], frag["received"]["coded"])
    assert receiver.block[: len(block)].hex() == frag["block"]

    block_key = fragment.data_block_int_key(unhex(mc["rootKey"]))
    assert block_key.hex() == frag["dataBlockIntKey"]
    code = fragment.BlockMic(
        block_key,
        frag["mic"]["sessionCnt"],
        frag["mic"]["fragIndex"],
        frag["mic"]["descriptor"].encode(),
        frag["mic"]["blockLen"],
    )
    code.update(block)
    assert code.finish().hex() == frag["mic"]["bytes"]

    for entry in vector["commands"]:
        raw = unhex(entry["bytes"])
        read = lorawan.package_parse(entry["port"], entry["uplink"], raw)
        assert read.kind == camel_to_snake(entry["kind"]), entry["bytes"]
        assert read.encode() == raw, entry["bytes"]
        for field, want in entry.get("fields", {}).items():
            got = getattr(read, camel_to_snake(field))
            assert (got.hex() if isinstance(got, bytes) else got) == want, (
                field,
                entry["bytes"],
            )

    for entry in vector["statusItems"]:
        raw = unhex(entry["bytes"])
        read = lorawan.package_status_item(raw)
        assert (read.mc_group_id, read.mc_addr) == (entry["mcGroupId"], entry["mcAddr"])
        assert read.encode() == raw

    # Nothing says how long an unknown command is, so reading stops rather than guessing.
    stopped = lorawan.package_parse_all(
        vector["stops"]["port"], vector["stops"]["uplink"], unhex(vector["stops"]["bytes"])
    )
    assert len(stopped) == vector["stops"]["readable"]

    with pytest.raises(PamojaError):
        lorawan.package_parse(
            vector["notAPackage"]["port"],
            vector["notAPackage"]["uplink"],
            unhex(vector["notAPackage"]["bytes"]),
        )


def camel_to_snake(name: str) -> str:
    """The vectors name fields the way JavaScript does; Python names them its own way."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()
