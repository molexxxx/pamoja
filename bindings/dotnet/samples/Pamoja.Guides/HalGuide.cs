using System.Globalization;

using Pamoja.Sensors;

using static Guides.Guide;

namespace Guides;

/// <summary>
/// The bus layer: a BME280 read over an I2C device in the shape of System.Device.I2c,
/// answered by a script of what the datasheet says the part sends.
/// </summary>
public static class HalGuide
{
    private const byte Part = Bme280.AddressPrimary;

    private static readonly byte[] CalibrationA =
    [
        0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E,
        0x88, 0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
    ];

    private static readonly byte[] CalibrationB = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];

    private static readonly byte[] Burst = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30];

    // ANCHOR: parts
    /// <summary>One transfer the script expects: a register written or read.</summary>
    private sealed record Step(byte Register, byte? Write = null, byte[]? Reply = null);

    /// <summary>
    /// A device with the two calls this conversation needs, in the shape of
    /// System.Device.I2c's I2cDevice: on a gateway <c>I2cDevice.Create</c> gives the
    /// real one and nothing below changes. This one answers from a script of what a
    /// BME280 sends, in the order the datasheet lists, and refuses any other transfer.
    /// </summary>
    private sealed class ScriptedDevice(Step[] script)
    {
        public int Transfers { get; private set; }

        public bool Done => Transfers == script.Length;

        public void Write(ReadOnlySpan<byte> bytes)
        {
            Step step = Next(bytes[0]);
            if (bytes.Length != 2 || step.Write != bytes[1])
            {
                throw new InvalidOperationException($"unexpected write of 0x{bytes[1]:X2}");
            }
        }

        public void WriteRead(ReadOnlySpan<byte> write, Span<byte> read)
        {
            Step step = Next(write[0]);
            if (step.Reply is null || step.Reply.Length != read.Length)
            {
                throw new InvalidOperationException($"unexpected read of {read.Length} bytes");
            }

            step.Reply.CopyTo(read);
        }

        private Step Next(byte register)
        {
            if (Transfers >= script.Length || script[Transfers].Register != register)
            {
                throw new InvalidOperationException($"unexpected transfer at register 0x{register:X2}");
            }

            return script[Transfers++];
        }
    }
    // ANCHOR_END: parts

    /// <summary>Runs the example.</summary>
    /// <returns>A task that completes when the measurement has been decoded.</returns>
    public static async Task RunAsync()
    {
        // ANCHOR: example
        var bus = new ScriptedDevice(
        [
            new Step(0xE0, Write: 0xB6),
            new Step(0xF3, Reply: [0x00]),
            new Step(0xD0, Reply: [Bme280.ChipId]),
            new Step(0x88, Reply: CalibrationA),
            new Step(0xE1, Reply: CalibrationB),
            new Step(0xF5, Write: 0x00),
            new Step(0xF2, Write: 0x01),
            new Step(0xF4, Write: 0x24),
            new Step(0xF4, Write: 0x25),
            new Step(0xF3, Reply: [0x00]),
            new Step(0xF7, Reply: Burst),
        ]);

        // The datasheet's start-up: the soft reset word, its 2 ms start-up time, and
        // the status register, whose low bit clears once the calibration image loaded.
        bus.Write([0xE0, 0xB6]);
        await Task.Delay(2);
        byte[] status = new byte[1];
        bus.WriteRead([0xF3], status);

        // The chip id says it is a BME280, and the two calibration blocks are read once.
        byte[] id = new byte[1];
        bus.WriteRead([0xD0], id);
        byte[] tempPress = new byte[26];
        byte[] humidity = new byte[7];
        bus.WriteRead([0x88], tempPress);
        bus.WriteRead([0xE1], humidity);
        using var calibration = new Bme280Calibration(tempPress, humidity);
        Console.WriteLine($"calibration  read once: {(id[0] == Bme280.ChipId).ToString().ToLowerInvariant()}");

        // config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes
        // effect after the ctrl_meas write: every measurement at oversampling x1, asleep.
        bus.Write([0xF5, 0x00]);
        bus.Write([0xF2, 0x01]);
        bus.Write([0xF4, 0x24]);

        // One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for
        // these settings, the status read that confirms the part is idle, the burst.
        bus.Write([0xF4, 0x25]);
        await Task.Delay(10);
        bus.WriteRead([0xF3], status);
        byte[] burst = new byte[8];
        bus.WriteRead([0xF7], burst);
        Bme280Measurement measurement = calibration.Compensate(burst);
        string celsius = measurement.Celsius.ToString("F2", CultureInfo.InvariantCulture);
        string hectopascals = measurement.Hectopascals.ToString("F2", CultureInfo.InvariantCulture);
        string humidityPercent = measurement.RelativeHumidityPercent.ToString("F2", CultureInfo.InvariantCulture);
        Console.WriteLine($"measured     {celsius} C, {hectopascals} hPa, {humidityPercent} %");

        // The script is spent: every transfer the datasheet lists was made, and no other.
        Console.WriteLine($"bus          {bus.Transfers} transfers, unexpected: {(!bus.Done).ToString().ToLowerInvariant()}");
        // ANCHOR_END: example

        Expect(celsius == "20.44", "the temperature the calibration compensates to");
        Expect(measurement.Pascals == 84805, "the pressure in pascals");
        Expect(humidityPercent == "44.65", "the humidity in percent");
        Expect(bus.Transfers == 11, "the datasheet lists eleven transfers");
        Expect(bus.Done, "and the script is spent");
    }
}
