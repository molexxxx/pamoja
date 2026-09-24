"""The MAVLink guide example; see docs/guides/mavlink.md."""

# ANCHOR: example
from pamoja.mavlink import (
    MavlinkHeader,
    MavlinkMessage,
    MavlinkParser,
    enum_entry,
    enum_names,
    enum_value,
    from_dict,
    schema_for,
)

VEHICLE = 1
AUTOPILOT = 1
STATION = 255
PLANNER = 190


# An enum field travels as a number, and the dialect names each number. Printing the name
# keeps a reader from looking up what 2 or 81 means.
def name(enumeration: str, value: int) -> str:
    return enum_entry(enumeration, value) or str(value)


# Every node broadcasts a heartbeat to say what it is and that it is alive. The frame wraps
# the payload in a header and a checksum seeded with the message's own value.
heartbeat_shape = schema_for("HEARTBEAT")
announce = from_dict(
    heartbeat_shape,
    {
        "type": enum_value("MAV_TYPE_GCS"),
        "autopilot": enum_value("MAV_AUTOPILOT_INVALID"),
        "system_status": enum_value("MAV_STATE_ACTIVE"),
        "mavlink_version": 3,
    },
)
sent = announce.to_frame(MavlinkHeader(STATION, PLANNER, 0))
print(f"sent      {heartbeat_shape.name} in {len(sent.bytes)} bytes")

# The vehicle answers with its own heartbeat, which reaches the station behind some noise and
# a copy with its last byte flipped in flight.
vehicle = from_dict(
    heartbeat_shape,
    {
        "type": enum_value("MAV_TYPE_QUADROTOR"),
        "autopilot": enum_value("MAV_AUTOPILOT_ARDUPILOTMEGA"),
        "base_mode": enum_value("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED")
        | enum_value("MAV_MODE_FLAG_STABILIZE_ENABLED")
        | enum_value("MAV_MODE_FLAG_MANUAL_INPUT_ENABLED"),
        "system_status": enum_value("MAV_STATE_STANDBY"),
        "mavlink_version": 3,
    },
)
good = vehicle.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))
garbled = bytearray(good.bytes)
garbled[-1] ^= 0xFF
delivered = b"???" + bytes(garbled) + good.bytes

# The parser skips whatever does not start a frame and drops a frame whose checksum fails, so
# only the good copy comes out.
frames = MavlinkParser().push(delivered)
print(
    f"parsed    {len(frames)} frame out of {len(delivered)} bytes,"
    " past the noise and the garbled copy"
)
heard = MavlinkMessage.decode(heartbeat_shape, frames[0].payload)
print(
    f"heard     {name('MAV_TYPE', heard.get_int('type'))} on"
    f" {name('MAV_AUTOPILOT', heard.get_int('autopilot'))},"
    f" in {name('MAV_STATE', heard.get_int('system_status'))}"
)

# The base mode is a bitmask, so it names a set of flags rather than one value.
base_mode = heard.get_int("base_mode")
print(f"flags     {' | '.join(enum_names('MAV_MODE_FLAG', base_mode))}")
if base_mode & enum_value("MAV_MODE_FLAG_SAFETY_ARMED") == 0:
    print("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them")
# ANCHOR_END: example

assert heard.payload == vehicle.payload
assert len(frames) == 1
assert frames[0].message_id == heartbeat_shape.id

# ANCHOR: command
from pamoja.mavlink import CommandProtocol

# The vehicle's answers, each naming the command it answers.
ack_shape = schema_for("COMMAND_ACK")


def answer(command: int, result: int, progress: int):
    fields = {"command": command, "result": result, "progress": progress}
    return from_dict(ack_shape, fields).to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))


# A command is not fired and forgotten: the vehicle has to answer, and the sender asks again
# until it does. Each resend carries the next confirmation number, which is how the vehicle
# tells a retry from a second, deliberate command.
arming = CommandProtocol(enum_value("MAV_CMD_COMPONENT_ARM_DISARM"), 3)
arm = from_dict(
    schema_for("COMMAND_LONG"),
    {
        "param1": 1.0,
        "target_system": VEHICLE,
        "target_component": AUTOPILOT,
        "command": arming.command,
        "confirmation": arming.confirmation,
    },
)
arm.to_frame(MavlinkHeader(STATION, PLANNER, 1))
print(
    f"sent      {name('MAV_CMD', arm.get_int('command'))},"
    f" confirmation {arm.get_int('confirmation')}"
)
resend = arming.on_timeout()
if resend is not None:
    print(f"silence   resent with confirmation {resend}")

# An answer names the command it answers, so one for another command leaves this one
# waiting.
for command, result in (
    (enum_value("MAV_CMD_NAV_TAKEOFF"), enum_value("MAV_RESULT_ACCEPTED")),
    (enum_value("MAV_CMD_COMPONENT_ARM_DISARM"), enum_value("MAV_RESULT_ACCEPTED")),
):
    outcome = arming.on_frame(answer(command, result, 0))
    if outcome is not None and outcome.kind == "unrelated":
        print(f"stray     an answer for {name('MAV_CMD', command)} leaves it waiting")
    elif outcome is not None and outcome.kind == "final":
        print(f"armed     {name('MAV_RESULT', outcome.value)}")

# A long command reports progress before its final answer, and a refused one says why.
calibrating = CommandProtocol(enum_value("MAV_CMD_PREFLIGHT_CALIBRATION"), 3)
progress = calibrating.on_frame(
    answer(calibrating.command, enum_value("MAV_RESULT_IN_PROGRESS"), 40)
)
if progress is not None and progress.kind == "in_progress":
    print(f"progress  {name('MAV_CMD', calibrating.command)} is {progress.value}% done")
changing = CommandProtocol(enum_value("MAV_CMD_DO_SET_MODE"), 3)
refusal = changing.on_frame(answer(changing.command, enum_value("MAV_RESULT_DENIED"), 0))
if refusal is not None and refusal.kind == "final":
    print(
        f"refused   {name('MAV_CMD', changing.command)}"
        f" answered {name('MAV_RESULT', refusal.value)}"
    )

# A command nobody answers runs out of retries, and the caller stops asking.
returning = CommandProtocol(enum_value("MAV_CMD_NAV_RETURN_TO_LAUNCH"), 3)
sends = 1
while returning.on_timeout() is not None:
    sends += 1
print(f"gave up   {name('MAV_CMD', returning.command)} went unanswered {sends} times")
# ANCHOR_END: command

assert arming.confirmation == 1
assert sends == 4

# ANCHOR: signing
from pamoja.core import PamojaError
from pamoja.mavlink import KEY_LEN, MavlinkSigner, MavlinkVerifier, timestamp_now

# Both ends share a secret key; replace these filler bytes with your own. The signer stamps
# each frame with its link id and a timestamp that only moves forward.
key = bytes([7]) * KEY_LEN
signer = MavlinkSigner(key, 1, timestamp_now())
signed = signer.sign(
    MavlinkHeader(STATION, PLANNER, 2),
    heartbeat_shape.id,
    sent.payload,
    heartbeat_shape.crc_extra,
)
print(
    f"signed    {len(signed.bytes)} bytes: the {len(sent.bytes)} of the frame,"
    f" then a {len(signed.signature)}-byte signature"
)


# The vehicle checks each frame against the same key, and remembers the newest timestamp from
# each sender, so a recording played back later is refused.
def refused_with(check) -> str | None:
    try:
        check()
    except PamojaError as refusal:
        return str(refusal)
    return None


verifier = MavlinkVerifier(key)
if refused_with(lambda: verifier.verify(signed)) is None:
    print("accepted  the same key, and a timestamp it has not seen")
replayed = refused_with(lambda: verifier.verify(signed))
if replayed is not None:
    print(f"replayed  the same frame again is refused: {replayed}")
stranger = MavlinkSigner(bytes([9]) * KEY_LEN, 1, timestamp_now())
forged = stranger.sign(
    MavlinkHeader(STATION, PLANNER, 3),
    heartbeat_shape.id,
    sent.payload,
    heartbeat_shape.crc_extra,
)
forgery = refused_with(lambda: verifier.verify(forged))
if forgery is not None:
    print(f"forged    another key's frame is refused: {forgery}")
unsigned = refused_with(lambda: verifier.verify(sent))
if unsigned is not None:
    print(f"unsigned  a frame with no signature is refused: {unsigned}")
# ANCHOR_END: signing

assert signed.signed
assert not sent.signed

# ANCHOR: mission
from pamoja.mavlink import MissionReceiver, MissionSender, message

# A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions travel as
# degrees times ten million.
latitude, longitude = -33.85678, 151.2153
item_shape = schema_for("MISSION_ITEM_INT")


def item(command: str, x: int, y: int, z: float) -> bytes:
    fields = {
        "command": enum_value(command),
        "frame": enum_value("MAV_FRAME_GLOBAL_RELATIVE_ALT_INT"),
        "x": x,
        "y": y,
        "z": z,
        "autocontinue": 1,
    }
    return from_dict(item_shape, fields).payload


# The station offers the plan, and the vehicle drives the transfer: it asks for each item in
# turn and acknowledges the last one.
station = MavlinkHeader(STATION, PLANNER, 0)
aboard = MavlinkHeader(VEHICLE, AUTOPILOT, 0)
mission_type = enum_value("MAV_MISSION_TYPE_MISSION")
upload = MissionSender(VEHICLE, AUTOPILOT, mission_type)
upload.add_item(item("MAV_CMD_NAV_TAKEOFF", 0, 0, 20.0))
upload.add_item(
    item("MAV_CMD_NAV_WAYPOINT", round(latitude * 1e7), round(longitude * 1e7), 50.0)
)
upload.add_item(item("MAV_CMD_NAV_RETURN_TO_LAUNCH", 0, 0, 0.0))
vehicle_side = MissionReceiver(STATION, PLANNER, mission_type)
to_vehicle = upload.count(station)
print(f"count     the station offers {len(upload)} items")
finished = None
while finished is None:
    step = vehicle_side.on_frame(to_vehicle, aboard)
    if step.accepted is not None:
        arrived = step.accepted
        print(
            f"arrived   item {arrived.get_int('seq')},"
            f" {name('MAV_CMD', arrived.get_int('command'))}"
        )
    if step.kind == "request":
        print(f"request   the vehicle asks for item {vehicle_side.expected}")
    following = upload.on_frame(step.reply, station)
    if following.kind == "finished":
        finished = following.result
    else:
        to_vehicle = following.reply
print(f"done      the vehicle answered {name('MAV_MISSION_RESULT', finished)}")

# A request past the end of the plan is answered with a refusal, not with an item.
past = message("MISSION_REQUEST_INT")
past.set_int("seq", 7)
past.set_int("target_system", STATION)
past.set_int("target_component", PLANNER)
past.set_int("mission_type", mission_type)
reply = upload.on_frame(past.to_frame(aboard), station)
if reply is not None and reply.kind == "reply":
    refused = MavlinkMessage.decode(schema_for("MISSION_ACK"), reply.reply.payload)
    print(
        f"refused   a request for item {past.get_int('seq')} is answered"
        f" {name('MAV_MISSION_RESULT', refused.get_int('type'))}"
    )
# ANCHOR_END: mission

assert vehicle_side.complete
assert finished == enum_value("MAV_MISSION_ACCEPTED")
