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
        // A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
        // that goes on the wire: the address shifts up one and the low bit says whether
        // this transaction reads or writes, which is easiest to get wrong by hand.
        const byte Bme280 = 0x76;
        Console.WriteLine($"write to  0x{I2c.AddressFrame(Bme280)[0]:X2}");
        Console.WriteLine($"read from 0x{I2c.AddressFrame(Bme280, read: true)[0]:X2}");

        // The I2C specification keeps two ranges of addresses for itself, so a part
        // answering in either is a wiring mistake rather than a device.
        Console.WriteLine(
            $"0x{Bme280:X2} reserved: {I2c.IsReserved(Bme280)}, "
            + $"0x{I2c.ReservedFrom:X2} reserved: {I2c.IsReserved(I2c.ReservedFrom)}");

        // A 10-bit address spends a reserved prefix over two bytes rather than one, so a
        // bus driver sends a different number of bytes depending on the address it holds.
        // This is the worked example UM10204 itself prints.
        const ushort TenBitDevice = 0x2A5;
        Console.WriteLine($"a 10-bit address takes {I2c.FrameLen(TenBitDevice, tenBit: true)} bytes");

        // Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the
        // clock high and samples on the trailing edge.
        SpiClock clock = Spi.ClockFor(3);
        Console.WriteLine(
            $"spi mode 3: idles high {clock.Cpol}, samples on the trailing edge {clock.Cpha}");

        // A relay board sold as active low energises when its pin is driven low. The
        // polarity carries that inversion, so no call site has to remember it.
        PinLevel energise = Pin.LevelFor(PinPolarity.ActiveLow, true);
        Console.WriteLine($"to energise an active-low relay, drive the pin {energise}");

        // Releasing it drives the line back high, an edge a falling trigger ignores.
        bool rising = Pin.Triggers(PinEdge.Rising, PinLevel.Low, PinLevel.High);
        bool falling = Pin.Triggers(PinEdge.Falling, PinLevel.Low, PinLevel.High);
        Console.WriteLine(
            $"release seen by a rising trigger: {rising}, by a falling trigger: {falling}");
        // ANCHOR_END: example

        Expect(I2c.AddressFrame(Bme280, read: true)[0] == 0xED, "the read frame sets the low bit");
        Expect(!I2c.IsReserved(Bme280), "the sensor address is a device address");
        Expect(I2c.IsReserved(I2c.ReservedFrom), "and the other range is reserved");
        Expect(I2c.FrameLen(TenBitDevice, tenBit: true) == 2, "a 10-bit address takes two bytes");
        Expect(clock.Cpol && clock.Cpha, "mode 3 is both bits set");
        Expect(Spi.ModeFor(true, false) == 2, "and the pair maps back to a mode number");
        Expect(energise == PinLevel.Low, "active low energises on a low level");
        Expect(Pin.IsAsserted(PinPolarity.ActiveLow, PinLevel.Low), "and reads as asserted");
        Expect(rising, "the release is a rising edge");
        Expect(!falling, "which a falling trigger ignores");
    }
}
