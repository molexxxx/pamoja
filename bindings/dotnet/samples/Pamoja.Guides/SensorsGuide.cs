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
        // Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published
        // check value over the ASCII digits 1 to 9 is 0xA1.
        Expect(Ds18b20.Crc8("123456789"u8) == 0xA1, "the published CRC check value");

        // A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over
        // the other eight, so a reading is verified before it is believed.
        byte[] scratchpad = [0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
        scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
        Ds18b20Reading reading = Ds18b20.ParseScratchpad(scratchpad);

        // Register 0x0191 is the +25.0625 degree row of the datasheet's temperature
        // table, each count a sixteenth of a degree, so micro-degrees stay exact.
        Expect(reading.RawTemperature == 0x0191, "the temperature register reads back");
        Expect(reading.MicroCelsius == 25_062_500, "the datasheet's temperature row");
        Expect(reading.ResolutionBits == 12, "the configuration byte selects 12 bits");
        Expect(reading.AlarmHigh == 75, "and the scratchpad carries its alarm threshold");

        // A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a
        // plausible temperature a few degrees off.
        byte[] corrupt = [.. scratchpad];
        corrupt[0] ^= 0x01;
        bool rejected = false;
        try
        {
            Ds18b20.ParseScratchpad(corrupt);
        }
        catch (PamojaException)
        {
            rejected = true;
        }
        Expect(rejected, "a scratchpad corrupted on the bus is rejected");

        // The INA219 datasheet's worked design example: 1 mA per count across a 2
        // milliohm shunt calibrates to 0x5000, and its registers then read 11.98 V,
        // 10 A, and 119.8 W.
        const uint currentLsb = 1_000;
        Expect(Ina219.Calibration(currentLsb, 2) == 0x5000, "the calibration register");
        Expect(Ina219.BusMillivolts(0x5D98) == 11_980, "the bus sits at 11.98 V");
        Expect(Ina219.CurrentMicroamps(0x2710, currentLsb) == 10_000_000, "10 A in the shunt");
        Expect(Ina219.PowerMicrowatts(0x1766, currentLsb) == 119_800_000, "drawing 119.8 W");
        // ANCHOR_END: example
    }
}
