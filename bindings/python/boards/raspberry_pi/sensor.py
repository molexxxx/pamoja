"""The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read
through the driver pamoja ships, printed every two seconds.

Wire the BME280's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, and turn
the I2C interface on. See docs/boards/raspberry-pi.md.
"""

# ANCHOR: example
import time

from pamoja.hal import I2cBus
from pamoja.sensors import Bme280, bme280


def main() -> None:
    # The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    bus = I2cBus.open("/dev/i2c-1")

    # The driver runs the datasheet's sequence over that bus: reset, identify, read the
    # calibration, configure, and then a forced measurement per read.
    sensor = Bme280(bus, bme280.ADDRESS_PRIMARY)
    sensor.init()

    while True:
        reading = sensor.measure()
        print(
            f"{reading.celsius:.2f} C, {reading.hectopascals:.2f} hPa, "
            f"{reading.relative_humidity_percent:.2f} % humidity"
        )
        time.sleep(2)
# ANCHOR_END: example


if __name__ == "__main__":
    main()
