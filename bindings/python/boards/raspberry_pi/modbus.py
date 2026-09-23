"""A Modbus line scan: asks every unit address on an RS485 line, through a USB adapter, which
devices are there.

Wire the adapter's A and B to every device's A and B, join the grounds, and set the line's
format below to the one the devices use. See docs/guides/modbus.md.
"""

# ANCHOR: example
from pamoja.hal import Parity, SerialPort, SerialSettings
from pamoja.modbus import ModbusClient, ModbusClientError

# A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
# adapter that presents itself as a modem.
PORT = "/dev/ttyUSB0"


def main() -> None:
    # The format every device on the line uses, from their manuals: 19200 8E1 is the default
    # the specification sets, and many meters ship at 9600 8N1 instead.
    settings = SerialSettings(19_200, Parity.EVEN)
    port = SerialPort.open(PORT, settings)

    # A device that is there answers within a few milliseconds, so a short response timeout
    # keeps the scan of all 247 addresses under half a minute.
    client = ModbusClient(port, response_timeout=0.1)

    found = 0
    for unit in range(1, 248):
        # Holding register 0 is a question any device can answer, with its value or with an
        # exception, and either proves the device is there.
        try:
            [value] = client.read_holding_registers(unit, 0, 1)
            print(f"unit {unit:3}  holding register 0 is {value}")
            found += 1
        except ModbusClientError as error:
            if error.kind == "exception":
                print(f"unit {unit:3}  there, and refused register 0 with exception {error.exception:#04x}")
                found += 1
            elif not (error.kind == "timeout" and error.received == 0):
                print(f"unit {unit:3}  {error}: check the format and the wiring")
    print(f"{found} units answered on {PORT} at {settings}")


if __name__ == "__main__":
    main()
# ANCHOR_END: example
