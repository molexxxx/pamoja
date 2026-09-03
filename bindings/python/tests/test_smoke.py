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


def test_a_sensor_reading_decodes_and_checks_itself():
    from pamoja import sensors

    scratchpad = bytearray([0x91, 0x01, 0x4B, 0x46, 0x7F, 0xFF, 0x0C, 0x10, 0x00])
    scratchpad[8] = sensors.ds18b20.crc8(bytes(scratchpad[:8]))
    reading = sensors.ds18b20.parse_scratchpad(bytes(scratchpad))
    assert reading.micro_celsius == 25_062_500
    assert reading.resolution_bits == 12

    scratchpad[0] ^= 0xFF
    with pytest.raises(PamojaError):
        sensors.ds18b20.parse_scratchpad(bytes(scratchpad))

    assert sensors.ina219.calibration(1_000, 2) == 0x5000
    assert sensors.ina219.power_microwatts(100, 1_000) == 2_000_000

    reset = sensors.ads1115.config_from_bits(0x8583)
    assert sensors.ads1115.config_bits(reset) == 0x8583
    assert sensors.ads1115.full_scale_microvolts(1) == 4_096_000


def test_an_actuator_command_encodes_to_its_registers():
    from pamoja import actuators

    assert actuators.pwm.full_off()[3] == 0x10
    assert actuators.pca9685.channel_register(0) == 0x06
    with pytest.raises(ValueError):
        actuators.pca9685.channel_register(16)

    motor = actuators.Stepper(actuators.Drive.HALF_STEP)
    first = motor.coils
    for _ in range(actuators.Drive.HALF_STEP.step_count):
        motor.step(actuators.Direction.FORWARD)
    assert motor.coils == first
    assert motor.steps == 8
    assert actuators.steps_for_degrees(90.0, 200) == 50


def test_the_windowed_helpers_summarise_recent_readings():
    from pamoja import WINDOW_CAPACITY, Anomaly, Median, Trend, Window

    window = Window()
    for value in (10.0, 20.0, 30.0):
        window.push(value)
    assert len(window) == 3
    assert window.capacity == WINDOW_CAPACITY
    assert window.mean() == pytest.approx(20.0)
    assert window.range() == pytest.approx(20.0)

    median = Median()
    for value in (20.0, 21.0, 20.5):
        median.update(value)
    assert median.update(900.0) < 30.0

    trend = Trend()
    for value in (1.0, 2.0, 3.0, 4.0):
        trend.push(value)
    assert trend.slope == pytest.approx(1.0, abs=1e-4)

    anomaly = Anomaly(3.0)
    for _ in range(8):
        anomaly.check(20.0)
    assert anomaly.check(900.0)


def test_a_signed_chain_records_what_a_node_did():
    from pamoja import DeviceIdentity, audit

    keeper = DeviceIdentity(bytes([0x21]) * 32)
    log = audit.AuditLog(keeper)
    opened = log.append(b"valve=open")
    shut = log.append(b"valve=shut")

    assert opened.index == 0
    assert shut.previous == opened.digest
    assert opened.payload == b"valve=open"
    assert audit.verify_chain(keeper.public_key, [opened, shut])

    edited = bytearray(shut.to_bytes())
    edited[-1] ^= 0xFF
    tampered = audit.AuditEntry.from_bytes(bytes(edited))
    assert not audit.verify_chain(keeper.public_key, [opened, tampered])

    resumed = audit.AuditLog.resume(keeper, shut)
    assert resumed.append(b"valve=open").index == 2


def test_two_devices_talk_in_confidence_and_refuse_a_replay():
    from pamoja import PamojaError, session

    node = session.AgreementKey(bytes([0x01]) * 32)
    gateway = session.AgreementKey(bytes([0x02]) * 32)
    salt = bytes([0x09]) * 16

    uplink = session.Session(node, gateway.public_key, salt, session.Role.INITIATOR)
    downlink = session.Session(gateway, node.public_key, salt, session.Role.RESPONDER)

    sealed = uplink.seal(b"4.8C", b"pump-3")
    assert sealed.ciphertext != b"4.8C"
    assert downlink.open(sealed, b"pump-3") == b"4.8C"

    with pytest.raises(PamojaError):
        downlink.open(sealed, b"pump-3")

    altered = uplink.seal(b"4.9C", b"pump-3")
    broken = bytearray(altered.ciphertext)
    broken[0] ^= 0xFF
    with pytest.raises(PamojaError):
        downlink.open(
            session.SealedMessage(altered.counter, altered.tag, bytes(broken)),
            b"pump-3",
        )


def test_a_release_stages_in_pieces_and_confirms_once_it_runs():
    import hashlib

    from pamoja import DeviceIdentity, PamojaError, update

    vendor = bytes([0x0A]) * 16
    device_class = bytes([0x0B]) * 16
    publisher = DeviceIdentity(bytes([0x31]) * 32)
    image = bytes([0xA5]) * 600
    manifest = update.Manifest(
        sequence=2,
        vendor_id=vendor,
        class_id=device_class,
        storage=1,
        digest=hashlib.sha256(image).digest(),
        size=len(image),
    )
    envelope = update.sign_manifest(manifest, publisher)
    assert update.verify_envelope(envelope, publisher.public_key).digest == manifest.digest

    fleet = update.Updater(vendor, device_class, publisher.public_key, 2, 4096)
    fleet.provision(0, 1)
    assert fleet.begin(envelope) == 1
    for at in range(0, len(image), 128):
        fleet.write(image[at : at + 128])
    assert fleet.progress().written == len(image)
    assert fleet.finish() == 1

    boot = fleet.on_boot()
    assert boot.action == update.BootAction.TRYING
    assert fleet.confirm() == 1
    assert fleet.slot_record(1).state == update.SlotState.CONFIRMED

    impostor = DeviceIdentity(bytes([0x32]) * 32)
    forged = update.sign_manifest(
        update.Manifest(
            sequence=3,
            vendor_id=vendor,
            class_id=device_class,
            storage=0,
            digest=hashlib.sha256(image).digest(),
            size=len(image),
        ),
        impostor,
    )
    with pytest.raises(PamojaError):
        fleet.stage(forged, image)


def test_a_delegated_key_may_sign_releases():
    import hashlib

    from pamoja import DeviceIdentity, update

    vendor = bytes([0x0C]) * 16
    device_class = bytes([0x0D]) * 16
    anchor = DeviceIdentity(bytes([0x41]) * 32)
    releases = DeviceIdentity(bytes([0x42]) * 32)

    statement = update.sign_delegation(
        update.Delegation(epoch=1, release_key=releases.public_key), anchor
    )
    assert update.open_delegation(statement, anchor.public_key).release_key == releases.public_key

    fleet = update.Updater(vendor, device_class, anchor.public_key, 2, 4096)
    fleet.provision(0, 1)
    fleet.adopt(statement)
    assert fleet.delegation is not None

    image = bytes([7]) * 64
    envelope = update.sign_manifest(
        update.Manifest(
            sequence=2,
            vendor_id=vendor,
            class_id=device_class,
            storage=1,
            digest=hashlib.sha256(image).digest(),
            size=len(image),
        ),
        releases,
    )
    assert fleet.stage(envelope, image) == 1


def test_a_falling_charge_stretches_the_work_interval():
    from pamoja import power

    plan = power.power_plan(60_000_000, 300_000_000, 3_600_000_000)
    assert plan.mode(0.9) == power.PowerMode.ACTIVE
    assert plan.mode(0.3) == power.PowerMode.SAVER
    assert plan.mode(0.1) == power.PowerMode.CRITICAL
    assert plan.mode_while_charging(0.1, True) == power.PowerMode.SAVER
    assert plan.interval_us(0.1) == 3_600_000_000

    duty = power.DutyCycle.from_fraction(1_000_000, 0.25)
    assert duty.active_us == 250_000
    assert duty.period_us == 1_000_000


def test_a_costly_link_drops_detail_but_keeps_the_count():
    from pamoja import telemetry

    reporter = telemetry.Reporter(telemetry.Level.TRACE)
    reporter.adapt_to(telemetry.LinkCost.EXPENSIVE)

    assert reporter.record(telemetry.Event(telemetry.Level.INFO, "loop.tick")) is None
    warned = reporter.record(
        telemetry.Event(telemetry.Level.WARN, "battery.low", 0.18)
    )
    assert warned is not None
    assert warned.code == "battery.low"
    assert warned.value == pytest.approx(0.18)

    counts = reporter.snapshot()
    assert counts.dropped == 1
    assert counts.emitted == 1
    assert telemetry.link_cost_threshold(telemetry.LinkCost.OFFLINE) == telemetry.Level.ERROR


def test_a_message_published_in_process_reaches_a_subscriber():
    from pamoja import loopback

    async def run():
        broker = loopback.LoopbackBroker()
        publisher = broker.link()
        subscriber = broker.link()

        await publisher.connect()
        await subscriber.connect()
        assert await subscriber.is_connected() is True

        await subscriber.subscribe("sensors/1")
        await publisher.send("sensors/1", b"21.5")

        message = await subscriber.recv()
        assert message.topic == "sensors/1"
        assert message.payload == b"21.5"

    asyncio.run(run())


def test_a_buffer_holds_records_until_a_link_returns():
    from pamoja import sync

    async def run():
        store = sync.Store.memory()
        await store.append(b"one")
        await store.append(b"two")

        assert await store.len() == 2
        assert await store.peek() == b"one", "peek leaves the record in place"
        assert await store.pop() == b"one"
        assert await store.pop() == b"two"
        assert await store.pop() is None

        bounded = sync.Store.memory(1)
        await bounded.append(b"one")
        with pytest.raises(PamojaError):
            await bounded.append(b"two")
        assert await bounded.len() == 1, "a full store keeps what it already had"

    asyncio.run(run())


def test_a_ladder_falls_through_a_failing_rung_and_buffers_when_none_work():
    from pamoja import ladder, loopback, sync, transport

    async def run():
        broker = loopback.LoopbackBroker()
        listener = broker.link()
        await listener.connect()
        await listener.subscribe("sensors/1")

        # Nothing to send over yet, so it buffers rather than losing the reading.
        rungless = ladder.Ladder(sync.Store.memory())
        assert await rungless.send("sensors/1", b"21.5") == ladder.Delivery.BUFFERED
        assert await rungless.buffered() == 1

        # The first rung refuses its next send; the second is the same broker.
        rungs = ladder.Ladder(sync.Store.memory())
        await rungs.rung(transport.Transport.faulty(broker.rung(), 1))
        await rungs.rung(broker.rung())
        await rungs.connect()

        assert await rungs.send("sensors/1", b"21.5") == ladder.Delivery.SENT
        message = await listener.recv()
        assert message.payload == b"21.5", "the second rung carried what the first refused"

    asyncio.run(run())


def test_a_flush_replays_what_was_buffered():
    from pamoja import ladder, loopback, sync

    async def run():
        broker = loopback.LoopbackBroker()
        listener = broker.link()
        await listener.connect()
        await listener.subscribe("sensors/1")

        offline = ladder.Ladder(sync.Store.memory())
        await offline.send("sensors/1", b"one")
        await offline.send("sensors/1", b"two")

        # The link comes back.
        await offline.rung(broker.rung())
        await offline.connect()

        assert await offline.flush() == 2
        assert await offline.buffered() == 0

    asyncio.run(run())


def test_a_spent_transport_cannot_be_added_twice():
    from pamoja import ladder, loopback, sync, transport

    async def run():
        broker = loopback.LoopbackBroker()
        rung = broker.rung()
        assert rung.is_available is True

        rungs = ladder.Ladder(sync.Store.memory())
        await rungs.rung(rung)
        assert rung.is_available is False

        with pytest.raises(PamojaError):
            await rungs.rung(rung)

        # A wrapper consumes it the same way.
        wrapped = transport.Transport.faulty(broker.rung(), 1)
        assert wrapped.is_available is True

    asyncio.run(run())


def test_every_subscriber_sees_a_published_event():
    from pamoja import bus

    async def run():
        hub = bus.EventBus(8)
        first = await hub.subscribe()
        second = await hub.subscribe()

        await hub.publish(b"battery.low")

        assert await first.next_event() == b"battery.low"
        assert await second.next_event() == b"battery.low"

    asyncio.run(run())


def test_simulated_devices_run_without_hardware():
    from pamoja import sim

    async def run():
        # The same seed gives the same run, so a test can assert on it.
        first = sim.SimulatedSensor(20.0, drift_per_read=0.5, noise=1.0, seed=42)
        second = sim.SimulatedSensor(20.0, drift_per_read=0.5, noise=1.0, seed=42)
        for _ in range(5):
            assert await first.read() == await second.read()

        replay = sim.Replay([21.0, 21.5, 22.0], repeating=True)
        for _ in range(2):
            for want in (21.0, 21.5, 22.0):
                assert await replay.read() == pytest.approx(want)

        actuator = sim.RecordingActuator()
        for command in (0.0, 0.5, 1.0):
            await actuator.apply(command)
        assert await actuator.commands() == pytest.approx([0.0, 0.5, 1.0])

        robot = sim.SimulatedRobot(1.0)
        await robot.apply(vx=1.0)
        pose = await robot.pose()
        assert pose.x == pytest.approx(1.0, abs=1e-5), "one second at one metre a second"
        assert pose.y == pytest.approx(0.0, abs=1e-5)

    asyncio.run(run())


CHATTER_HASH = "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18"


def test_a_profile_decides_what_a_reading_calls_for():
    from pamoja import profile

    fridge = profile.Profile.vaccine_fridge_monitor()
    assert fridge.name == "vaccine-fridge-monitor"
    assert fridge.control.kind == profile.ControlKind.SETPOINT

    warm = fridge.controller().evaluate(9.0)
    assert warm.actuator is True, "a warm fridge runs the cooler"
    assert warm.alert.kind == profile.AlertKind.OUT_OF_RANGE
    assert warm.alert.reading == pytest.approx(9.0)

    observed = profile.Controller.monitor().evaluate(21.5)
    assert observed.actuator is None, "a monitor drives no output"
    assert observed.alert is None


def test_a_profile_manifest_round_trips():
    from pamoja import profile

    original = profile.Profile.well_level()
    reloaded = profile.Profile.from_json(original.to_json())
    assert reloaded.topic == original.topic
    assert reloaded.control.kind == original.control.kind
    assert reloaded.power.active_secs == original.power.active_secs

    with pytest.raises(ValueError):
        profile.Profile.from_json("{")


def test_ros2_names_map_onto_the_dds_wire():
    from pamoja import ros2

    assert ros2.is_valid_name("/robot1/camera_left/image_raw")
    assert not ros2.is_valid_name("/2foo"), "a token may not start with a digit"
    assert ros2.is_fully_qualified("/chatter")

    assert ros2.dds_topic("/robot1/cmd_vel", ros2.EntityKind.TOPIC) == "rt/robot1/cmd_vel"
    assert ros2.prefix_for(ros2.EntityKind.SERVICE_REQUEST) == "rq"
    assert ros2.dds_type_name("std_msgs/msg/String") == "std_msgs::msg::dds_::String_"

    assert len(ros2.type_hash_digest(CHATTER_HASH)) == 32
    assert ros2.type_hash_digest("not a hash") is None
    assert ros2.entity_key(0, "/chatter", "std_msgs/msg/String", CHATTER_HASH) == (
        f"0/chatter/std_msgs::msg::dds_::String_/{CHATTER_HASH}"
    )


def test_a_twist_survives_a_cdr_round_trip():
    from pamoja import ros2

    encoded = ros2.twist_to_cdr((1.5, 0.0, 0.0), (0.0, 0.0, -0.25))
    linear, angular = ros2.twist_from_cdr(encoded)
    assert linear == pytest.approx((1.5, 0.0, 0.0))
    assert angular == pytest.approx((0.0, 0.0, -0.25))
    assert ros2.twist_from_cdr(b"") is None


def test_cdr_mixed_width_fields_keep_their_alignment():
    from pamoja import ros2

    writer = ros2.CdrWriter()
    writer.write_u32(7)
    writer.write_f64(2.5)
    writer.write_i32(-3)

    reader = ros2.CdrReader(writer.bytes)
    assert reader.read_u32() == 7
    assert reader.read_f64() == pytest.approx(2.5)
    assert reader.read_i32() == -3, "the field after an eight-byte one is not skewed"
    assert reader.read_u32() is None, "reading past the end yields None"

    with pytest.raises(ValueError):
        ros2.CdrReader(b"")


def test_zenoh_key_expressions_address_a_fleet_subtree():
    from pamoja import zenoh

    assert zenoh.is_valid("fleet/*/battery")
    assert zenoh.matches("fleet/*/battery", "fleet/n7/battery")
    assert not zenoh.matches("fleet/*/battery", "fleet/n7/rack/battery")
    assert zenoh.canonize("fleet/**/**/battery") == "fleet/**/battery"
    assert not zenoh.is_canon("fleet/**/**/battery")


def test_a_published_region_reports_what_its_band_allows():
    from pamoja import lora

    plan = lora.plan_for("EU868")
    assert plan.name == "EU863-870"
    assert plan.link_settings(0).spreading_factor == 12
    assert plan.duty_cycle_permille(868_100_000) == 10, "the 868.1 MHz sub-band is 1%"
    assert plan.max_eirp_dbm(868_100_000) == 16
    assert plan.max_payload(5).application == 242
    assert plan.rx1_data_rate(5, 0) == 5, "RX1 at offset 0 mirrors the uplink rate"
    assert plan.rx2() == (869_525_000, 0)
    assert plan.next_backoff_data_rate(0) is None, "DR0 has nothing slower"

    # Every code the plan lists resolves, in either form and any case.
    for code in lora.ChannelPlan.regions():
        assert lora.plan_for(code.lower()).name
    assert lora.plan_for("EU863-870").name == "EU863-870"

    with pytest.raises(ValueError, match="not a published region"):
        lora.plan_for("Atlantis")


def test_a_reserved_data_rate_is_told_from_one_the_plan_lacks():
    from pamoja import lora

    # EU868 defines all fourteen of its numbers, DR9 among them.
    eu868 = lora.plan_for("EU868")
    assert eu868.data_rate(9).kind == "lr_fhss"
    assert eu868.data_rate(9).coding_rate_numerator == 2
    assert eu868.data_rate(200) is None, "a number past the table is absent"

    # US915 numbers its downlink rates from DR8, so DR2 is a reserved slot.
    us915 = lora.plan_for("US915")
    assert us915.data_rate(2, "downlink").kind == "reserved"
    assert us915.data_rate(8, "downlink").kind == "lora"
    assert us915.duty_cycle_permille(903_000_000) is None, (
        "the FCC caps dwell time rather than duty cycle, so US915 has no sub-band"
    )
    assert lora.plan_for("AU915").info().has_dwell_time_limit


def test_the_message_budget_a_band_leaves():
    from pamoja import lora

    plan = lora.plan_for("EU868")
    assert lora.messages_per_hour_at(plan, 5, 20, 868_100_000) > 0
    assert lora.messages_per_hour_at(plan, 5, 20, 700_000_000) is None, (
        "a frequency outside the band has no duty cycle to budget against"
    )


def test_a_private_plan_answers_what_a_published_one_does():
    from pamoja import lora

    # A deployment on licensed spectrum, which may hold the channel continuously.
    builder = lora.ChannelPlanBuilder("private-915")
    builder.data_rate(lora.LoraDataRate.lora(12, 125_000, 250))
    builder.data_rate(lora.LoraDataRate.lora(7, 125_000, 5_470))
    builder.max_payload(lora.LoraMaxPayload(59, 51))
    builder.max_payload(lora.LoraMaxPayload(230, 222))
    builder.channel_block(lora.LoraChannelBlock(915_000_000, 500_000, 4, 0, 1))
    builder.sub_band(lora.LoraSubBand(915_000_000, 917_000_000, 1000, 30))
    builder.rx(915_000_000)
    builder.rx1_row([0])
    builder.rx1_row([1])
    plan = builder.build()

    assert plan.name == "private-915"
    assert plan.channel_frequency_hz(3) == 916_500_000
    assert plan.duty_cycle_permille(915_500_000) == 1000, "licensed spectrum is unrestricted"
    assert plan.max_eirp_dbm(915_500_000) == 30
    assert plan.max_payload(1, "downlink_direct").application == 222, (
        "an empty downlink table mirrors the uplink one"
    )
    assert plan.next_backoff_data_rate(1) == 0, "an unset chain steps down one rate"
    assert len(plan.sub_bands()) == 1
    assert plan.channel_blocks()[0].count == 4

    # A builder is spent once built.
    with pytest.raises(ValueError, match="already been built"):
        builder.build()


def test_an_inconsistent_plan_is_refused_where_it_is_built():
    from pamoja import lora

    # Offsets up to 5 mean every RX1 row needs six entries; this one has one.
    builder = lora.ChannelPlanBuilder("too-narrow")
    builder.data_rate(lora.LoraDataRate.lora(12, 125_000, 250))
    builder.rx(915_000_000, 0, 5)
    builder.rx1_row([0])
    with pytest.raises(ValueError, match="RX1 row"):
        builder.build()

    # Listening at a data rate the plan never defines could not work either.
    builder = lora.ChannelPlanBuilder("bad-rx2")
    builder.data_rate(lora.LoraDataRate.lora(12, 125_000, 250))
    builder.rx(915_000_000, 3, 0)
    builder.rx1_row([0])
    with pytest.raises(ValueError, match="RX2 listens"):
        builder.build()


# HEARTBEAT announcing an onboard controller in an active state.
_HEARTBEAT = bytes([0, 0, 0, 0, 18, 0, 0, 4, 3])


def test_a_frame_reaches_an_autopilot_and_reads_back():
    from pamoja import mavlink

    header = mavlink.MavlinkHeader(1, 1, 7)
    assert mavlink.known_crc_extra(0) == 50, "HEARTBEAT's published CRC_EXTRA"
    assert mavlink.known_crc_extra(9999) is None, "an id outside the common dialect"

    sent = mavlink.frame(header, 0, _HEARTBEAT)
    assert sent.version == 2, "v2 is the current wire format"
    assert sent.message_id == 0
    assert not sent.signed, "an ordinary frame carries no signature"
    assert sent.signature is None

    back = mavlink.MavlinkFrame.parse_known(sent.bytes)
    assert back.payload == _HEARTBEAT
    assert back.header.system_id == 1
    assert back.header.sequence == 7

    # A frame mangled in transit is refused rather than acted on.
    mangled = bytearray(sent.bytes)
    mangled[12] ^= 0xFF
    with pytest.raises(ValueError):
        mavlink.MavlinkFrame.parse_known(bytes(mangled))

    # A message the common dialect does not define cannot be built blind.
    with pytest.raises(ValueError, match="not in the common dialect"):
        mavlink.frame(header, 50_000, b"\x00")


def test_the_parser_joins_a_stream_already_in_progress():
    from pamoja import mavlink

    wire = mavlink.frame(mavlink.MavlinkHeader(2, 1), 0, _HEARTBEAT).bytes
    parser = mavlink.MavlinkParser()

    assert parser.push(b"\x11\x22\x33") == [], "noise between frames is skipped"
    assert parser.push(wire[:5]) == [], "half a frame is not a frame"
    found = parser.push(wire[5:])
    assert len(found) == 1, "the rest of it completes one"
    assert found[0].message_id == 0

    # The queueing form, for a caller that drains on its own schedule.
    parser.feed(wire)
    assert parser.pending == 1
    assert parser.next_frame().message_id == 0
    assert parser.next_frame() is None, "an empty parser means feed it more"


def test_a_private_dialect_is_checked_once_its_seed_is_derived():
    from pamoja import mavlink

    header = mavlink.MavlinkHeader(9, 1)
    fields = [("uint32_t", "uptime", 0)]

    dialect = mavlink.Dialect()
    seed = dialect.add_message(50_000, "PRIVATE_STATUS", fields)
    assert seed == mavlink.message_crc_extra("PRIVATE_STATUS", fields), (
        "the seed is derived, not invented"
    )
    assert dialect.crc_extra(50_000) == seed
    assert dialect.crc_extra(0) == 50, "and the common dialect still answers"

    sent = mavlink.MavlinkFrame.raw(header, 50_000, seed, (42).to_bytes(4, "little"))

    with pytest.raises(ValueError):
        mavlink.MavlinkFrame.parse_known(sent.bytes)

    back = mavlink.MavlinkFrame.parse_known(sent.bytes, dialect)
    assert back.message_id == 50_000, "the dialect makes it checkable"
    # MAVLink 2 drops trailing zero bytes, so a four-byte field holding 42
    # arrives as one byte; a decoder zero-extends it.
    assert back.payload == bytes([42])


def test_a_signed_frame_proves_its_sender_and_refuses_a_replay():
    from pamoja import mavlink

    key = bytes([7]) * mavlink.KEY_LEN
    header = mavlink.MavlinkHeader(1, 1)
    signer = mavlink.MavlinkSigner(key, link_id=1, timestamp=mavlink.timestamp_now())
    assert signer.link_id == 1

    signed = signer.sign(header, 0, _HEARTBEAT, 50)
    assert signed.signed
    assert len(signed.signature) == mavlink.SIGNATURE_LEN
    assert signed.signature[0] == 1, "the link id leads the signature block"

    verifier = mavlink.MavlinkVerifier(key)
    verifier.verify(signed)
    with pytest.raises(ValueError):
        verifier.verify(signed)

    # A different key is a different sender, and an unsigned frame is never
    # silently treated as authentic.
    with pytest.raises(ValueError):
        mavlink.MavlinkVerifier(bytes([9]) * mavlink.KEY_LEN).verify(signed)
    with pytest.raises(ValueError):
        mavlink.MavlinkVerifier(key).verify(mavlink.frame(header, 0, _HEARTBEAT))

    with pytest.raises(ValueError, match="signing key"):
        mavlink.MavlinkSigner(b"short", 1, 0)
