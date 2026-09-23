"""A greenhouse logger on a Raspberry Pi: the air from an SHT31 on the header's I2C bus, and
every DS18B20 soil probe the kernel's 1-Wire driver has found, printed every ten seconds.

Wire the SHT31's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, and turn the
I2C interface on. Wire each probe's red lead to 3V3, its black lead to ground, and its yellow
data lead to GPIO4, with one 4.7 kilohm resistor from GPIO4 to 3V3 for the whole bus; add
``dtoverlay=w1-gpio`` to config.txt and reboot. See docs/guides/sensors.md.
"""

# ANCHOR: example
import time

from pamoja.core import PamojaError
from pamoja.hal import I2cBus
from pamoja.sensors import Ds18b20Thermometer, Sht3x, sht3x


def main() -> None:
    # The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
    bus = I2cBus.open("/dev/i2c-1")
    air = Sht3x(bus, sht3x.ADDRESS_A)

    # The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
    # serial. The list is taken once, so a probe plugged in later needs a restart.
    probes = Ds18b20Thermometer.discover()
    if not probes:
        raise SystemExit(
            "no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay"
        )

    while True:
        now = air.measure()
        print(f"air           {now.celsius:.2f} C, {now.relative_humidity:.1f} %")

        # Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
        # corrupted on a long lead fails its checksum, and the logger says so and reads the
        # probe again next time rather than stopping.
        for probe in probes:
            try:
                soil = probe.read()
                print(f"{probe.serial}  {soil.celsius:.2f} C")
            except PamojaError as error:
                print(f"{probe.serial}  {error}")
        time.sleep(10)
# ANCHOR_END: example


if __name__ == "__main__":
    main()
