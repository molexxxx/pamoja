// ANCHOR: example
using System.Globalization;

using Pamoja;
using Pamoja.Hal;
using Pamoja.Sensors;

namespace Boards.RaspberryPi;

/// <summary>
/// A greenhouse logger on a Raspberry Pi: the air from an SHT31 on the header's I2C bus, and
/// every DS18B20 soil probe the kernel's 1-Wire driver has found, printed every ten seconds.
/// Wire the SHT31's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, and turn the
/// I2C interface on. Wire each probe's red lead to 3V3, its black lead to ground, and its yellow
/// data lead to GPIO4, with one 4.7 kilohm resistor from GPIO4 to 3V3 for the whole bus; add
/// <c>dtoverlay=w1-gpio</c> to config.txt and reboot.
/// </summary>
public static class Probes
{
    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var air = new Sht3x(bus, Sht3x.AddressA);

        // The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
        // serial. The list is taken once, so a probe plugged in later needs a restart.
        IReadOnlyList<Ds18b20Thermometer> probes = Ds18b20Thermometer.Discover();
        if (probes.Count == 0)
        {
            throw new PamojaException("no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay");
        }

        while (true)
        {
            var now = air.Measure();
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"air           {now.Celsius:F2} C, {now.RelativeHumidity:F1} %"));

            // Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
            // corrupted on a long lead fails its checksum, and the logger says so and reads the
            // probe again next time rather than stopping.
            foreach (Ds18b20Thermometer probe in probes)
            {
                try
                {
                    Ds18b20Reading soil = probe.Read();
                    Console.WriteLine(string.Create(CultureInfo.InvariantCulture, $"{probe.Serial}  {soil.Celsius:F2} C"));
                }
                catch (PamojaException error)
                {
                    Console.WriteLine($"{probe.Serial}  {error.Message}");
                }
            }

            Thread.Sleep(TimeSpan.FromSeconds(10));
        }
    }
}
// ANCHOR_END: example
