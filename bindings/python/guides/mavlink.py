"""The MAVLink guide example; see docs/guides/mavlink.md."""

# ANCHOR: example
from pamoja.mavlink import (
    CommandProtocol,
    MavlinkHeader,
    MavlinkMessage,
    MavlinkParser,
    from_dict,
    message,
    schema_for,
)

VEHICLE = 1
AUTOPILOT = 1
STATION = 255

# The values the MAVLink common dialect gives these fields.
MAV_TYPE_GCS = 6
MAV_TYPE_QUADROTOR = 2
MAV_AUTOPILOT_INVALID = 8
MAV_AUTOPILOT_ARDUPILOTMEGA = 3
MAV_STATE_ACTIVE = 4
MAV_STATE_STANDBY = 3
MAV_CMD_COMPONENT_ARM_DISARM = 400
MAV_CMD_NAV_TAKEOFF = 22
MAV_RESULT_ACCEPTED = 0

# Every MAVLink node broadcasts a heartbeat to say what it is and that it is alive. The
# fields are set by name rather than by writing the payload out byte by byte.
announce = message("HEARTBEAT")
announce.set("type", MAV_TYPE_GCS)
announce.set("autopilot", MAV_AUTOPILOT_INVALID)
announce.set("system_status", MAV_STATE_ACTIVE)
announce.set("mavlink_version", 3)
sent = announce.to_frame(MavlinkHeader(STATION, 190, 0))
print(f"sent      HEARTBEAT in {len(sent.bytes)} bytes")

# The vehicle answers with its own heartbeat. This copy arrives after some bytes that were
# already on the wire, and after a copy with one bit flipped in flight.
vehicle_shape = schema_for("HEARTBEAT")
vehicle = from_dict(
    vehicle_shape,
    {
        "type": MAV_TYPE_QUADROTOR,
        "autopilot": MAV_AUTOPILOT_ARDUPILOTMEGA,
        "system_status": MAV_STATE_STANDBY,
        "mavlink_version": 3,
    },
)
good = vehicle.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))
garbled = bytearray(good.bytes)
garbled[-1] ^= 0xFF
delivered = b"???" + bytes(garbled) + good.bytes

# The parser skips whatever does not start a frame and drops one whose checksum fails, so
# the frame it hands back is the good copy rather than the garbled one.
parser = MavlinkParser()
received = parser.push(delivered)[0]
heard = MavlinkMessage.decode(vehicle_shape, received.payload)
print(f"heard     a type-{heard.get_int('type')} vehicle in state {heard.get_int('system_status')}")

# Arming it is a command, not a message a sender fires and forgets: the vehicle has to
# answer, and the sender keeps asking until it does. The protocol numbers each resend,
# which is how a vehicle tells a retry from a second, deliberate command.
arming = CommandProtocol(MAV_CMD_COMPONENT_ARM_DISARM, 3)
command_shape = schema_for("COMMAND_LONG")
arm = from_dict(
    command_shape,
    {
        "param1": 1.0,  # 1 arms, 0 disarms
        "target_system": VEHICLE,
        "target_component": AUTOPILOT,
        "command": arming.command,
        "confirmation": arming.confirmation,
    },
)
arm.to_frame(MavlinkHeader(STATION, 190, 1))
print(f"sent      arm request, confirmation {arming.confirmation}")

# Nothing comes back in time, so it goes again with the next confirmation number.
resend = arming.on_timeout()
print(f"silence, resending with confirmation {resend}")

# An acknowledgement names the command it answers, so one for a different command is not
# this exchange finishing.
ack_shape = schema_for("COMMAND_ACK")


def acknowledgement(command: int) -> object:
    built = from_dict(ack_shape, {"command": command, "result": MAV_RESULT_ACCEPTED})
    return built.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))


stray = arming.on_frame(acknowledgement(MAV_CMD_NAV_TAKEOFF))
print(f"an ack for another command: {stray.kind}")

outcome = arming.on_frame(acknowledgement(MAV_CMD_COMPONENT_ARM_DISARM))
if outcome.kind == "final" and outcome.value == MAV_RESULT_ACCEPTED:
    print("armed     the vehicle is ready")
else:
    print(f"the vehicle answered {outcome.kind} {outcome.value}")
# ANCHOR_END: example

assert heard.get_int("type") == MAV_TYPE_QUADROTOR
assert received.message_id == 0
assert resend == 1
assert stray.kind == "unrelated"
assert outcome.kind == "final" and outcome.value == MAV_RESULT_ACCEPTED
