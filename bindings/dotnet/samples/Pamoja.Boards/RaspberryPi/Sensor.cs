// ANCHOR: example
using System.Globalization;

using Pamoja.Hal;
using Pamoja.Sensors;

namespace Boards.RaspberryPi;

/// <summary>
/// The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read through
/// the driver pamoja ships, printed every two seconds. Wire the BME280's SDA to GPIO2, its SCL
/// to GPIO3, VIN to 3V3, and GND to ground, and turn the I2C interface on.
/// </summary>
public static class Sensor
{
    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");

        // The driver runs the datasheet's sequence over that bus: reset, identify, read the
        // calibration, configure, and then a forced measurement per read.
        using var sensor = new Bme280(bus, Bme280.AddressPrimary);
        sensor.Init();

        while (true)
        {
            Bme280Measurement reading = sensor.Measure();
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"{reading.Celsius:F2} C, {reading.Hectopascals:F2} hPa, {reading.RelativeHumidityPercent:F2} % humidity"));
            Thread.Sleep(TimeSpan.FromSeconds(2));
        }
    }
}
// ANCHOR_END: example
