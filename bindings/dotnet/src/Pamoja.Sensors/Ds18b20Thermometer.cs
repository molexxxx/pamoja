using Pamoja.Native.Interop;

namespace Pamoja.Sensors;

/// <summary>A DS18B20 the Linux kernel serves as a <c>w1_slave</c> file.</summary>
/// <remarks>
/// <para>
/// On a Raspberry Pi the <c>w1-gpio</c> overlay (<c>dtoverlay=w1-gpio</c> in <c>config.txt</c>)
/// puts a 1-Wire bus on GPIO 4, and the kernel's <c>w1_therm</c> driver lists every DS18B20 it
/// finds as a directory named by its family code and serial, such as <c>28-000005e2fdc3</c>.
/// Reading the directory's <c>w1_slave</c> file makes the kernel run a conversion and print
/// the scratchpad, which <see cref="Read"/> decodes.
/// </para>
/// <para>
/// That is the way to reach a DS18B20 from a Linux process, where the bit timing a 1-Wire
/// driver needs cannot be held. The file is only text, so a test can write one anywhere and
/// read it with <see cref="At"/>.
/// </para>
/// </remarks>
public sealed class Ds18b20Thermometer : IDisposable
{
    private readonly NativeHandle _handle;

    private Ds18b20Thermometer(IntPtr thermometer) =>
        _handle = NativeHandle.Create(
            thermometer, NativeMethods.pamoja_ds18b20_thermometer_free, "DS18B20 thermometer");

    /// <summary>The path of the file the thermometer reads.</summary>
    public string Path => OwnedString.Read(_handle.Use(NativeMethods.pamoja_ds18b20_thermometer_path));

    /// <summary>Names a thermometer by the serial in its directory name.</summary>
    /// <param name="serial">The twelve hex digits after <c>28-</c>.</param>
    /// <returns>The thermometer, reading <c>/sys/bus/w1/devices/28-&lt;serial&gt;/w1_slave</c>.</returns>
    /// <exception cref="PamojaException">The serial could not be passed to the native side.</exception>
    public static Ds18b20Thermometer ForSerial(string serial)
    {
        ArgumentNullException.ThrowIfNull(serial);
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_thermometer_new(serial, out IntPtr thermometer));
        return new Ds18b20Thermometer(thermometer);
    }

    /// <summary>Names a thermometer by the path of its <c>w1_slave</c> file.</summary>
    /// <param name="path">The file to read.</param>
    /// <returns>The thermometer.</returns>
    /// <exception cref="PamojaException">The path could not be passed to the native side.</exception>
    public static Ds18b20Thermometer At(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_thermometer_at(path, out IntPtr thermometer));
        return new Ds18b20Thermometer(thermometer);
    }

    /// <summary>Lists every DS18B20 the kernel has found, one per <c>28-</c> directory.</summary>
    /// <param name="devices">The directory the kernel lists its 1-Wire devices in, or null for <c>/sys/bus/w1/devices</c>.</param>
    /// <returns>The thermometers, sorted by directory name.</returns>
    /// <exception cref="PamojaException">
    /// The directory cannot be listed, which usually means the 1-Wire overlay is off.
    /// </exception>
    public static IReadOnlyList<Ds18b20Thermometer> Discover(string? devices = null)
    {
        Status.ThrowIfError(NativeMethods.pamoja_ds18b20_discover(devices, out IntPtr found));
        try
        {
            int count = (int)NativeMethods.pamoja_ds18b20_thermometers_len(found);
            var thermometers = new List<Ds18b20Thermometer>(count);
            for (int index = 0; index < count; index++)
            {
                thermometers.Add(new Ds18b20Thermometer(
                    NativeMethods.pamoja_ds18b20_thermometers_get(found, (nuint)index)));
            }

            return thermometers;
        }
        finally
        {
            NativeMethods.pamoja_ds18b20_thermometers_free(found);
        }
    }

    /// <summary>Reads the file, which makes the kernel run a conversion, and decodes it.</summary>
    /// <returns>The CRC-checked reading.</returns>
    /// <exception cref="PamojaException">
    /// The file cannot be read, because the overlay is off, the probe is gone, or the process may
    /// not read it; or the kernel or this decoder rejects the CRC.
    /// </exception>
    public Ds18b20Reading Read()
    {
        PamojaDs18b20Reading reading = default;
        Status.ThrowIfError(_handle.Use(thermometer =>
            NativeMethods.pamoja_ds18b20_thermometer_read(thermometer, out reading)));
        return Ds18b20.Read(reading);
    }

    /// <summary>Releases the thermometer.</summary>
    public void Dispose() => _handle.Dispose();
}
