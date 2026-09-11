using Pamoja.Gpio;

using static Guides.Guide;

namespace Guides;

/// <summary>The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.</summary>
public static class GpioGuide
{
    // ANCHOR: parts
    /// <summary>
    /// The board's own library drives the line: <c>System.Device.Gpio</c> on a Raspberry
    /// Pi, a vendor SDK on a microcontroller. This stands in for one so the example runs
    /// with nothing plugged in, and it is the only part a real node replaces.
    /// </summary>
    private sealed class Line
    {
        private readonly Queue<PinLevel> _readings;

        public Line(params PinLevel[] readings) => _readings = new Queue<PinLevel>(readings);

        public List<PinLevel> Driven { get; } = new();

        public void Drive(PinLevel level) => Driven.Add(level);

        public PinLevel Read() => _readings.Dequeue();
    }
    // ANCHOR_END: parts

    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // Most relay boards energize when their input is pulled low, and a float switch
        // wired to ground closes the same way. Saying "active low" once, here, is what
        // keeps the inversion out of every line below it.
        const PinPolarity Relay = PinPolarity.ActiveLow;
        const PinPolarity Float = PinPolarity.ActiveLow;
        var pump = new Line();
        var floatSwitch = new Line(PinLevel.High, PinLevel.Low);
        Console.WriteLine(
            $"a pump on an active-low relay runs when its line is {Pin.LevelFor(Relay, true)}");

        // The pump runs while the tank fills. The stand-in line answers open and then
        // closed, so this is the real loop with nothing plugged in.
        pump.Drive(Pin.LevelFor(Relay, true));
        bool whileFilling = Pin.IsAsserted(Float, floatSwitch.Read());
        bool onceFilled = Pin.IsAsserted(Float, floatSwitch.Read());
        Console.WriteLine($"the float reads full: {whileFilling}, then {onceFilled}");

        // The moment the float closes is that line going low, which is a falling edge. A
        // watch armed for the rising one would sleep through the tank filling.
        bool closing = Pin.Triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low);
        Console.WriteLine($"the float closing is a falling edge on that line: {closing}");

        // Full, so the pump stops, and the levels the line was driven to are the whole
        // conversation the board saw.
        pump.Drive(Pin.LevelFor(Relay, false));
        (PinLevel ran, PinLevel stopped) = (pump.Driven[0], pump.Driven[1]);
        Console.WriteLine($"running drove the line {ran} and stopping drove it {stopped}");

        // A part on a shared bus answers to an address, and the byte on the wire is not
        // the address the datasheet prints: it shifts up one and the low bit says read or
        // write.
        byte toWrite = I2c.AddressFrame(0x76)[0];
        byte toRead = I2c.AddressFrame(0x76, read: true)[0];
        Console.WriteLine(
            $"a part at 0x76 is written to as 0x{toWrite:X2} and read from as 0x{toRead:X2}");

        // Two ranges belong to the specification itself, so a part answering in either is
        // a wiring mistake rather than a device.
        bool reserved = I2c.IsReserved(I2c.ReservedFrom);
        Console.WriteLine(
            $"0x{I2c.ReservedFrom:X2} is reserved by the specification: {reserved}");

        // And a datasheet quotes SPI's clock polarity and phase as one mode number.
        SpiClock clock = Spi.ClockFor(3);
        Console.WriteLine(
            $"SPI mode 3 idles high: {clock.Cpol}, samples on the trailing edge: {clock.Cpha}");
        // ANCHOR_END: example

        Expect(Pin.LevelFor(Relay, true) == PinLevel.Low, "an active-low relay runs on a low line");
        Expect(!whileFilling, "an open float is not asserted");
        Expect(onceFilled, "and a closed one is");
        Expect(closing, "the float closing is a falling edge");
        Expect(
            !Pin.Triggers(PinEdge.Rising, PinLevel.High, PinLevel.Low),
            "which a rising watch ignores");
        Expect(ran == PinLevel.Low && stopped == PinLevel.High, "the line moved both ways");
        Expect(toWrite == 0xEC && toRead == 0xED, "the address frames as two different bytes");
        Expect(!I2c.IsReserved(0x76), "0x76 is a device address");
        Expect(reserved, "and the reserved block is not");
        Expect(clock.Cpol && clock.Cpha, "mode 3 is CPOL 1 with CPHA 1");
        Expect(Spi.ModeFor(true, false) == 2, "and CPOL 1 with CPHA 0 is mode 2");
    }
}
