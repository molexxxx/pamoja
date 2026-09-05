using Pamoja;
using Pamoja.Sensors;

using static Guides.Guide;

namespace Guides;

/// <summary>The sensor-driver guide example; see docs/guides/sensors.md.</summary>
public static class SensorsGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // Stand-ins for the two parts. On a running node the thermometer's nine bytes
        // come off the 1-Wire bus and the monitor's registers off I2C; here the library
        // builds what each would send, so the program runs with nothing plugged in.
        byte[] thermometer = Ds18b20.BuildScratchpad(25.0625f, 12, 75, -10);

        // The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load
        // its datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
        const uint CurrentLsb = 1_000;
        ushort bus = Ina219.BusRegister(11_980);
        short current = Ina219.CurrentRegister(10_000_000, CurrentLsb);
        ushort power = Ina219.PowerRegister(119_800_000, CurrentLsb);

        // Everything below is the node's own code. The thermometer checksums every read,
        // so a reading is verified before it is believed.
        Ds18b20Reading reading = Ds18b20.ParseScratchpad(thermometer);
        Console.WriteLine($"temperature  {reading.Celsius:F4} C");
        Console.WriteLine($"resolution   {reading.ResolutionBits} bits");
        Console.WriteLine($"alarms       {reading.AlarmHigh} / {reading.AlarmLow} C");

        // The monitor computes nothing until it has been told what shunt it is across.
        Console.WriteLine($"calibration  0x{Ina219.Calibration(CurrentLsb, 2):X4}");
        Console.WriteLine($"bus          {Ina219.BusMillivolts(bus)} mV");
        Console.WriteLine($"current      {Ina219.CurrentMicroamps(current, CurrentLsb) / 1_000} mA");
        Console.WriteLine($"power        {Ina219.PowerMicrowatts(power, CurrentLsb) / 1_000} mW");

        // A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the
        // read instead of logging a temperature a couple of degrees off.
        byte[] corrupted = [.. thermometer];
        corrupted[0] ^= 1;
        try
        {
            Ds18b20.ParseScratchpad(corrupted);
            Console.WriteLine("corrupt read accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"corrupt read rejected: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(reading.RawTemperature == 0x0191, "the temperature register reads back");
        Expect(reading.MicroCelsius == 25_062_500, "the datasheet's temperature row");
        Expect(reading.ResolutionBits == 12, "the configuration byte selects 12 bits");
        Expect(reading.AlarmHigh == 75, "the scratchpad carries its high threshold");
        Expect(reading.AlarmLow == -10, "and its low one");

        // The datasheet's own figures for that design: calibration 0x5000, and registers
        // that read back 11.98 V, 10 A, and 119.8 W.
        Expect(Ina219.Calibration(CurrentLsb, 2) == 0x5000, "the datasheet's calibration");
        Expect(Ina219.BusMillivolts(bus) == 11_980, "11.98 V across the load");
        Expect(Ina219.CurrentMicroamps(current, CurrentLsb) == 10_000_000, "10 A through it");
        Expect(Ina219.PowerMicrowatts(power, CurrentLsb) == 119_800_000, "and 119.8 W");

        // The published check value for CRC-8/MAXIM-DOW, the checksum every 1-Wire part
        // appends to what it sends.
        Expect(Ds18b20.Crc8("123456789"u8) == 0xA1, "the published CRC check value");
    }
}
