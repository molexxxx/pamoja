"""A UART self-test: the Pi's own serial port with TX jumpered to RX, sending COBS frames and
reading each one straight back.

Turn the serial port hardware on and the serial console off (raspi-config, Interface Options,
Serial Port), reboot, and put one jumper between GPIO14 and GPIO15. See docs/guides/serial.md.
"""

# ANCHOR: example
import time

from pamoja.hal import SerialPort, SerialSettings
from pamoja.serial import CobsDecoder, cobs

# The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
PORT = "/dev/serial0"


def main() -> None:
    port = SerialPort.open(PORT, SerialSettings(115_200))
    decoder = CobsDecoder()

    for sequence in range(1, 6):
        payload = sequence.to_bytes(2, "big") + b"ping"
        frame = cobs.encode(payload)
        started = time.perf_counter()
        port.write(frame)

        # The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
        # until the line has been quiet for 100 ms.
        echoed = None
        while echoed is None:
            arrived = port.read(64, timeout=0.1)
            if not arrived:
                break
            payloads = decoder.feed(arrived)
            if payloads:
                echoed = payloads[0]
        if echoed is None:
            print(f"frame {sequence}  nothing came back: check the jumper, and that the console is off")
        elif echoed == payload:
            millis = (time.perf_counter() - started) * 1_000
            print(f"frame {sequence}  {len(frame)} bytes back in {millis:.2f} ms")
        else:
            print(f"frame {sequence}  came back changed: check the speed and the wiring")
        time.sleep(0.5)


if __name__ == "__main__":
    main()
# ANCHOR_END: example
