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
        // A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the
        // wire: the address shifts up one and the read/write bit fills the low bit.
        Expect(
            I2c.AddressFrame(0x76).SequenceEqual(new byte[] { 0xEC }),
            "the write frame carries the address shifted up");
        Expect(
            I2c.AddressFrame(0x76, read: true).SequenceEqual(new byte[] { 0xED }),
            "and the read frame sets the low bit");

        // UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either
        // range is a wiring mistake rather than a device.
        Expect(!I2c.IsReserved(0x76), "the sensor sits in the usable range");
        Expect(I2c.IsReserved(0x78), "0x78 is the 10-bit prefix, not a device");

        // A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix,
        // the top two address bits, and the read/write bit, then the low eight bits.
        Expect(I2c.FrameLen(0x2A5, tenBit: true) == 2, "a 10-bit address takes two bytes");
        Expect(
            I2c.AddressFrame(0x2A5, tenBit: true).SequenceEqual(new byte[] { 0xF4, 0xA5 }),
            "the two bytes the specification works through");
        Expect(
            I2c.AddressFrame(0x2A5, read: true, tenBit: true)
                .SequenceEqual(new byte[] { 0xF5, 0xA5 }),
            "which differ only in the read/write bit");

        // Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) |
        // CPHA, so mode 3 idles the clock high and samples on the trailing edge.
        SpiClock clock = Spi.ClockFor(3);
        Expect(clock.Cpol && clock.Cpha, "mode 3 idles high and samples late");
        Expect(Spi.ModeFor(cpol: true, cpha: false) == 2, "that pair is mode 2");

        // A relay board sold as active low energises when its pin is driven low. The
        // polarity carries that inversion so no call site has to remember it.
        PinLevel energised = Pin.LevelFor(PinPolarity.ActiveLow, asserted: true);
        Expect(energised == PinLevel.Low, "an active-low relay switches on at a low level");
        Expect(
            Pin.IsAsserted(PinPolarity.ActiveLow, energised),
            "and that level reads back as asserted");

        // Releasing the relay drives the line back high, an edge a falling trigger ignores.
        PinLevel released = Pin.Invert(energised);
        Expect(
            Pin.Triggers(PinEdge.Rising, energised, released),
            "releasing it is a low-to-high transition");
        Expect(
            !Pin.Triggers(PinEdge.Falling, energised, released),
            "which a falling trigger ignores");
        // ANCHOR_END: example
    }
}
