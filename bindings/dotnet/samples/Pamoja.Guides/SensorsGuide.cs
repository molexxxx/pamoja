using Pamoja;
using Pamoja.Hal;
using Pamoja.Sensors;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>
/// The sensor-driver guide example: a greenhouse bench with every I2C part pamoja drives on one
/// bus, and a DS18B20 read the way Linux serves one; see docs/guides/sensors.md.
/// </summary>
public static class SensorsGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The address plan. Every part answers at the address its pins choose, and two parts on
        // one address garble each other, so a bench of nine is planned around the two that
        // cannot move: the HDC1080 and the SCD41 have one address each.
        const byte Air = Sht3x.AddressA;
        const byte Pressure = Bmp280.AddressSecondary;
        const byte Light = Opt3001.AddressScl;
        const byte Soil = Ads1115.AddressSda;
        const byte Enclosure = Tmp117.AddressAdd0Vplus;
        byte panel = Ina219.Address(Ina219.AddressPin.Ground, Ina219.AddressPin.Supply);
        byte battery = Ina226.Address(Ina226.AddressPin.Supply, Ina226.AddressPin.Supply);

        // The bench with nothing plugged in: each part answers the way its datasheet says, with
        // the reading it is given here, a warm and humid afternoon. The bus keeps a copy of each
        // part, so the program lets go of its own. On a Raspberry Pi the bus is
        // I2cBus.Open("/dev/i2c-1") and nothing after this statement changes.
        SimulatedPart[] parts =
        [
            Sht3x.Sim.Reporting(Air, 24.1f, 62.0f),
            Bmp280.Sim.Reporting(Pressure, 24.1f, 1003.2f),
            Scd4x.Sim.Reporting(1_180, 24.1f, 62.0f),
            Opt3001.Sim.Reporting(Light, 4_200f),
            Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr4_096, 2.35f),
            Tmp117.Sim.Reporting(Enclosure, 31.25f),
            Hdc1080.Sim.Reporting(31.25f, 38.0f),
            Ina219.Sim.Reporting(panel, 100, 3_200_000, 18_400, 1_250_000),
            Ina226.Sim.Reporting(battery, 2, 20_000_000, 12_800_000, -350_000),
        ];
        using I2cBus bus = I2cBus.Simulated(parts);
        foreach (SimulatedPart part in parts)
        {
            part.Dispose();
        }

        // One driver per part. Each holds its own share of the bus, and each runs its
        // datasheet's whole conversation on the first measurement: reset, identify, configure,
        // convert, read.
        using var airSensor = new Sht3x(bus, Air);
        var airNow = airSensor.Measure();
        Console.WriteLine(Invariant($"air          {airNow.Celsius:F2} C, {airNow.RelativeHumidity:F2} %"));

        using var barometer = new Bmp280(bus, Pressure);
        Bmp280Reading weather = barometer.Measure();
        Console.WriteLine(Invariant($"pressure     {weather.Hectopascals:F1} hPa"));

        using var co2Sensor = new Scd4x(bus);
        var co2 = co2Sensor.Measure();
        Console.WriteLine($"co2          {co2.Co2Ppm} ppm");

        using var lightSensor = new Opt3001(bus, Light);
        Opt3001Reading sun = lightSensor.Measure();
        Console.WriteLine(Invariant($"light        {sun.Lux:F0} lux"));

        // A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
        // ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
        using var converter = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd, Ads1115.Pga.Fsr4_096);
        Ads1115Sample probe = converter.Sample();
        Console.WriteLine(Invariant($"soil         {probe.Volts:F3} V"));

        using var boxThermometer = new Tmp117(bus, Enclosure);
        using var boxHygrometer = new Hdc1080(bus);
        Tmp117Reading box = boxThermometer.Measure();
        var boxAir = boxHygrometer.Measure();
        Console.WriteLine(Invariant($"enclosure    {box.Celsius:F2} C, {boxAir.RelativeHumidity:F1} %"));

        // Two current monitors, each calibrated for its own shunt. The fans and the pump draw
        // more than the panel gives, so the battery makes up the rest and its current reads
        // negative: current through a shunt is signed by its direction.
        using var panelMonitor = new Ina219(bus, panel, shuntMilliohms: 100, maxMicroamps: 3_200_000);
        Ina219Reading charge = panelMonitor.Measure();
        Console.WriteLine(Invariant(
            $"panel        {charge.BusMillivolts / 1e3:F2} V, {charge.CurrentMicroamps / 1e6:F2} A, {charge.PowerMicrowatts / 1e6:F2} W"));
        using var batteryMonitor = new Ina226(bus, battery, shuntMilliohms: 2, maxMicroamps: 20_000_000);
        Ina226Reading drain = batteryMonitor.Measure();
        Console.WriteLine(Invariant(
            $"battery      {drain.BusVolts:F2} V, {drain.CurrentAmps:F2} A, {drain.PowerWatts:F2} W"));

        // Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
        // SCD41's command times most of all. The simulated bus counted the waits and slept none.
        Console.WriteLine(Invariant($"waited       {bus.WaitedMicros / 1e6:F2} s across {bus.Transfers} transfers"));

        // The same probe read at the default range. Past 2.048 V the converter pins at its top
        // code, and the sample says so rather than passing the edge of the range off as a reading.
        using (WordPart reprobed = Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr2_048, 2.35f))
        {
            bus.Attach(reprobed);
        }

        using var defaultGain = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd);
        Ads1115Sample pinned = defaultGain.Sample();
        Console.WriteLine(Invariant(
            $"default gain {pinned.Volts:F3} V, clipped: {pinned.Clipped.ToString().ToLowerInvariant()}"));

        // A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
        // device id before anything else, so pointed at the soil probe's converter it refuses
        // rather than reporting that part's registers as a temperature.
        try
        {
            using var misplaced = new Tmp117(bus, Soil);
            misplaced.Init();
            Console.WriteLine("wrong part   accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"wrong part   {error.Message}");
        }

        // A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's
        // 1-Wire driver serves each probe as a file under /sys/bus/w1/devices. The program writes
        // that file itself, with the text the kernel prints, so it runs anywhere.
        string devices = Path.Combine(Path.GetTempPath(), $"pamoja-bench-{Environment.ProcessId}");
        string directory = Path.Combine(devices, $"{Ds18b20.FamilyCode:x2}-000005e2fdc3");
        Directory.CreateDirectory(directory);
        byte[] scratchpad = Ds18b20.BuildScratchpad(19.5f, 12, 30, 5);
        File.WriteAllText(Path.Combine(directory, "w1_slave"), Ds18b20.W1SlaveText(scratchpad));
        var found = new List<Ds18b20Reading>();
        foreach (Ds18b20Thermometer thermometer in Ds18b20Thermometer.Discover(devices))
        {
            using (thermometer)
            {
                Ds18b20Reading reading = thermometer.Read();
                Console.WriteLine(Invariant(
                    $"soil probe   {reading.Celsius:F4} C, alarms at {reading.AlarmLow} and {reading.AlarmHigh} C"));
                found.Add(reading);
            }
        }

        Directory.Delete(devices, recursive: true);
        // ANCHOR_END: example

        Expect(Math.Abs(airNow.Celsius - 24.1f) < 0.003f, "the air temperature");
        Expect(Math.Abs(airNow.RelativeHumidity - 62.0f) < 0.002f, "and its humidity");
        Expect(Math.Abs(weather.Hectopascals - 1003.2f) < 0.01f, "the pressure");
        Expect(co2.Co2Ppm == 1_180, "the carbon dioxide");
        Expect(Math.Abs(sun.Lux - 4_200f) < 1.28f, "the step at that exponent");
        Expect(probe.Raw == 18_800, "2.35 V at 125 uV a count");
        Expect(!probe.Clipped, "inside the 4.096 V range");
        Expect(box.Celsius == 31.25f, "the enclosure temperature");
        Expect(Math.Abs(boxAir.RelativeHumidity - 38.0f) < 0.002f, "and its humidity");
        Expect(charge.BusMillivolts == 18_400, "the panel voltage");
        Expect(Math.Abs(charge.CurrentMicroamps - 1_250_000) < 98, "the panel current");
        Expect(Math.Abs(drain.CurrentMicroamps + 350_000) < 611, "the battery current");
        Expect(pinned.Clipped, "past the 2.048 V range");
        Expect(pinned.Raw == short.MaxValue, "the top code of Table 7-3");
        Expect(found.Count == 1, "one probe on the bench");
        Expect(found[0].MicroCelsius == 19_500_000, "the probe's temperature");
        Expect(panel == 0x41, "Table 1 of the INA219 datasheet: A1 to GND, A0 to VS+");
        Expect(battery == 0x45, "and of the INA226's: A1 and A0 to VS");

        // The decode half under every driver, anchored to its datasheets: the INA219's worked
        // design example calibrates 1 mA a count across 2 milliohms to 0x5000, and the 1-Wire
        // checksum gives the published CRC-8/MAXIM-DOW check value over the digits 1 to 9.
        Expect(Ina219.Calibration(1_000, 2) == 0x5000, "the datasheet's calibration");
        Expect(Ds18b20.Crc8("123456789"u8) == 0xA1, "the published CRC check value");
    }
}
