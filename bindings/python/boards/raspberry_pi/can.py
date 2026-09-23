"""A CAN bus monitor: listens on the Pi's CAN interface, through an MCP2515, and names each
J1939 message it hears.

Load the controller's overlay and bring the interface up at the bus's bit rate first. See
docs/guides/can.md.
"""

# ANCHOR: example
import time

from pamoja.can import CanBus, decode_j1939, signals_from

# The interface the MCP2515 overlay makes.
INTERFACE = "can0"

# Engine speed's parameter group, where the speed sits in it, and its scale.
ENGINE_CONTROLLER_1 = 61_444
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125


def main() -> None:
    # The monitor only listens and sends nothing, so it is safe on a running machine's bus.
    bus = CanBus.open(INTERFACE)
    started = time.perf_counter()
    while time.perf_counter() - started < 10:
        frame = bus.receive(timeout=1)
        if frame is None:
            print("quiet for a second: check the bit rate, the wiring, and the termination")
            continue
        at = time.perf_counter() - started
        message = decode_j1939(frame.id, frame.extended)
        if message is not None and message.pgn == ENGINE_CONTROLLER_1:
            rpm = (signals_from(frame.data).u16(ENGINE_SPEED_AT) or 0) * RPM_PER_BIT
            print(f"{at:7.3f} s  pgn {message.pgn} from {message.source}: {rpm:.1f} rpm")
        elif message is not None:
            print(f"{at:7.3f} s  pgn {message.pgn} from {message.source}, {frame.len} bytes")
        else:
            print(f"{at:7.3f} s  0x{frame.id:03X}, {frame.len} bytes, an 11-bit identifier")
    print(f"{bus.received} frames in ten seconds on {INTERFACE}")


if __name__ == "__main__":
    main()
# ANCHOR_END: example
