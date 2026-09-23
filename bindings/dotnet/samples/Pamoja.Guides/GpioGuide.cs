using Pamoja.Gpio;

using static Guides.Guide;

namespace Guides;

/// <summary>The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.</summary>
public static class GpioGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // Most relay boards energize when their input is pulled low, and a float switch
        // wired to ground closes the same way. Saying "active low" once, here, is what
        // keeps the inversion out of every line below it.
        var pump = Switch.ActiveLow(new PinScript());
        var floatSwitch = Contact.ActiveLow(new PinScript(PinLevel.High, PinLevel.Low));
        PinLevel runsOn = Pin.LevelFor(pump.Polarity, true);
        Console.WriteLine($"a pump on an active-low relay runs when its line is {runsOn}");

        // The pump runs while the tank fills. The scripted line answers open and then
        // closed, so this is the real loop with nothing plugged in; on a board the same two
        // lines take a pin from the board's GPIO library instead.
        pump.Set(true);
        bool whileFilling = floatSwitch.IsAsserted();
        bool onceFilled = floatSwitch.IsAsserted();
        static string Full(bool closed) => closed ? "full" : "not full";
        Console.WriteLine($"the float reads {Full(whileFilling)}, then {Full(onceFilled)}");

        // The moment the float closes is that line going low, which is a falling edge. A
        // watch armed for the rising one would sleep through the tank filling.
        bool closing = Pin.Triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low);
        string edge = closing ? "a falling edge" : "not a falling edge";
        Console.WriteLine($"the float closing is {edge} on that line");

        // Full, so the pump stops. Releasing the switch hands the line back, and the levels
        // it was driven to are the whole conversation the board saw.
        pump.Set(false);
        IReadOnlyList<PinLevel> driven = pump.Release().Driven;
        (PinLevel ran, PinLevel stopped) = (driven[0], driven[1]);
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
        string owner = reserved ? "reserved by the specification" : "free for a device";
        Console.WriteLine($"0x{I2c.ReservedFrom:X2} is {owner}");

        // And a datasheet quotes SPI's clock polarity and phase as one mode number.
        SpiClock clock = Spi.ClockFor(3);
        string idle = clock.Cpol ? "high" : "low";
        string sampling = clock.Cpha ? "trailing" : "leading";
        Console.WriteLine($"SPI mode 3 idles {idle} and samples on the {sampling} edge");
        // ANCHOR_END: example

        Expect(runsOn == PinLevel.Low, "an active-low relay runs on a low line");
        Expect(!whileFilling, "an open float is not asserted");
        Expect(onceFilled, "and a closed one is");
        Expect(closing, "the float closing is a falling edge");
        Expect(
            !Pin.Triggers(PinEdge.Rising, PinLevel.High, PinLevel.Low),
            "which a rising watch ignores");
        Expect(ran == PinLevel.Low && stopped == PinLevel.High, "the line moved both ways");
        Expect(!pump.IsAsserted, "and the pump ends off");
        Expect(toWrite == 0xEC && toRead == 0xED, "the address frames as two different bytes");
        Expect(!I2c.IsReserved(0x76), "0x76 is a device address");
        Expect(reserved, "and the reserved block is not");
        Expect(clock.Cpol && clock.Cpha, "mode 3 is CPOL 1 with CPHA 1");
        Expect(Spi.ModeFor(true, false) == 2, "and CPOL 1 with CPHA 0 is mode 2");

        if (Environment.GetEnvironmentVariable("PAMOJA_GPIO_CHIP") is { Length: > 0 } chip)
        {
            OnABoard(chip);
        }
    }

    // ANCHOR: board
    /// <summary>
    /// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
    /// float switch between GPIO27 and ground. Only the two lines change.
    /// </summary>
    /// <param name="chip">The GPIO chip's device file.</param>
    public static void OnABoard(string chip)
    {
        // The relay energizes on a low input, so its line is taken high and the pump stays
        // off until it is asked to run. The float closes to ground against a pull-up.
        using GpioLine relay = GpioLine.OpenOutput(chip, 17, PinLevel.High);
        using GpioLine floatLine = GpioLine.OpenInput(chip, 27);
        var pump = Switch.ActiveLow(relay);
        var floatSwitch = Contact.ActiveLow(floatLine);

        // Run the pump until the float closes, and stop it whatever happens: a pump left
        // running on a failed float is the fault this whole program exists to prevent.
        DateTime deadline = DateTime.UtcNow.AddMinutes(10);
        pump.Set(true);
        try
        {
            while (!floatSwitch.IsAsserted())
            {
                if (DateTime.UtcNow >= deadline)
                {
                    throw new TimeoutException(
                        "the tank did not fill in ten minutes; check the float and the supply");
                }

                Thread.Sleep(100);
            }
        }
        finally
        {
            pump.Set(false);
        }

        Console.WriteLine("the tank is full and the pump is off");
    }
    // ANCHOR_END: board
}
