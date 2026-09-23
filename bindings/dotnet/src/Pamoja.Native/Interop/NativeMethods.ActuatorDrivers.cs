using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the actuator drivers that run over an I2C bus and their
/// simulated parts, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The address a PCA9685 answers at with its six address pins low.</summary>
    public const byte Pca9685DefaultAddress = 0x40;

    /// <summary>How long a PCA9685's oscillator takes to run once woken, in microseconds.</summary>
    public const uint Pca9685OscillatorStartupMicros = 500;

    /// <summary>The frequency a PCA9685 driver runs at unless given another, in hertz.</summary>
    public const uint Pca9685DefaultFrequencyHz = 200;

    /// <summary>A PCA9685's mode register 1.</summary>
    public const byte Pca9685RegisterMode1 = 0x00;

    /// <summary>A PCA9685's mode register 2.</summary>
    public const byte Pca9685RegisterMode2 = 0x01;

    /// <summary>The first of a PCA9685's channel 0 registers.</summary>
    public const byte Pca9685RegisterLed0OnL = 0x06;

    /// <summary>The first of a PCA9685's four ALL_LED registers.</summary>
    public const byte Pca9685RegisterAllLedOnL = 0xFA;

    /// <summary>A PCA9685's prescaler.</summary>
    public const byte Pca9685RegisterPreScale = 0xFE;

    /// <summary>MODE1's RESTART bit.</summary>
    public const byte Pca9685Mode1Restart = 0x80;

    /// <summary>MODE1's EXTCLK bit.</summary>
    public const byte Pca9685Mode1Extclk = 0x40;

    /// <summary>MODE1's auto-increment bit.</summary>
    public const byte Pca9685Mode1AutoIncrement = 0x20;

    /// <summary>MODE1's SLEEP bit.</summary>
    public const byte Pca9685Mode1Sleep = 0x10;

    /// <summary>A PCA9685's MODE1 at power-up.</summary>
    public const byte Pca9685Mode1Reset = 0x11;

    /// <summary>A PCA9685's MODE2 at power-up.</summary>
    public const byte Pca9685Mode2Reset = 0x04;

    /// <summary>A PCA9685's PRE_SCALE at power-up.</summary>
    public const byte Pca9685PreScaleReset = 0x1E;

    /// <summary>The smallest value a PCA9685 loads into PRE_SCALE.</summary>
    public const byte Pca9685PreScaleMin = 3;

    /// <summary>Returns the settings a PCA9685 driver takes when given none.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPca9685Settings pamoja_pca9685_settings_default();

    /// <summary>Creates a PCA9685 driver on a bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_new(
        IntPtr bus,
        byte address,
        PamojaPca9685Settings settings,
        out IntPtr outDriver);

    /// <summary>Programs the prescale and the output wiring, and wakes the oscillator.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_init(IntPtr driver);

    /// <summary>Loads one channel's four register bytes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_set_channel(IntPtr driver, byte channel, PamojaPwm pwm);

    /// <summary>Loads every channel with the same four bytes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_set_all(IntPtr driver, PamojaPwm pwm);

    /// <summary>Stops the oscillator.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_sleep(IntPtr driver);

    /// <summary>Wakes the oscillator and restarts the channels.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_wake(IntPtr driver);

    /// <summary>Sends the general-call software reset.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_pca9685_software_reset(IntPtr driver);

    /// <summary>Returns the prescale value the driver writes.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_pca9685_prescale(IntPtr driver);

    /// <summary>Returns the frequency the part runs at after the prescaler rounds.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_pca9685_frequency(IntPtr driver);

    /// <summary>Releases a driver and its share of the bus.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_pca9685_free(IntPtr driver);

    /// <summary>Creates a simulated PCA9685 as it powers up.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_pca9685_sim_part(byte address);
}
