using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the bus layer of the pamoja C ABI - one I2C bus over the
/// kernel's adapter, simulated parts, or a script - mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must be
/// updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>A bus kind: the kernel's adapter.</summary>
    public const byte I2cBusAdapter = 0;

    /// <summary>A bus kind: simulated parts.</summary>
    public const byte I2cBusSimulated = 1;

    /// <summary>A bus kind: a script.</summary>
    public const byte I2cBusScripted = 2;

    /// <summary>A part kind: registers a byte wide.</summary>
    public const byte I2cPartBytes = 0;

    /// <summary>A part kind: registers sixteen bits wide.</summary>
    public const byte I2cPartWords = 1;

    /// <summary>A part kind: commands that leave replies.</summary>
    public const byte I2cPartCommands = 2;

    /// <summary>Creates a simulated part answering at one address from registers a byte wide.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_part_new(byte address);

    /// <summary>Creates a simulated part answering at one address from registers sixteen bits wide.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_word_part_new(byte address);

    /// <summary>Creates a simulated part answering at one address that takes commands.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_command_part_new(byte address, nuint width);

    /// <summary>Returns which kind of part a handle holds.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_i2c_part_kind(IntPtr part);

    /// <summary>Puts a value in one register of a part whose registers are sixteen bits wide.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_part_set_word(IntPtr part, byte register, ushort value);

    /// <summary>Marks bits of one sixteen-bit register as the part's to set.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_part_read_only(IntPtr part, byte register, ushort mask);

    /// <summary>Reads what one register of a part whose registers are sixteen bits wide holds.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_i2c_part_word(IntPtr part, byte register);

    /// <summary>Gives a part that takes commands the reply one command leaves.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_part_answer(
        IntPtr part,
        ReadOnlySpan<byte> command,
        nuint commandLen,
        ReadOnlySpan<byte> reply,
        nuint replyLen);

    /// <summary>Returns how many writes a part that takes commands has received.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_i2c_part_received_count(IntPtr part);

    /// <summary>Copies one write a part that takes commands received, or returns null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_part_received(IntPtr part, nuint index);

    /// <summary>Puts bytes in a part, from a register on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_part_load(
        IntPtr part,
        byte first,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Reads what one of a part's registers holds.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_i2c_part_register(IntPtr part, byte register);

    /// <summary>Reads consecutive registers of a part, from one register on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_part_read(
        IntPtr part,
        byte first,
        Span<byte> outBytes,
        nuint len);

    /// <summary>Returns the address a part answers to.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_i2c_part_address(IntPtr part);

    /// <summary>Returns how many transfers a part has served.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_i2c_part_transfers(IntPtr part);

    /// <summary>Releases a part. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_i2c_part_free(IntPtr part);

    /// <summary>Creates an empty script.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_script_new();

    /// <summary>Adds a step: the driver writes exactly these bytes to the address.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_script_write(
        IntPtr script,
        byte address,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Adds a step: the driver reads from the address and receives the reply.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_script_read(
        IntPtr script,
        byte address,
        ReadOnlySpan<byte> reply,
        nuint len);

    /// <summary>Adds a step: a write and then a read in one transaction.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_script_write_read(
        IntPtr script,
        byte address,
        ReadOnlySpan<byte> bytes,
        nuint len,
        ReadOnlySpan<byte> reply,
        nuint replyLen);

    /// <summary>Adds a step: the next transfer to the address fails.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_script_fault(IntPtr script, byte address, byte fault);

    /// <summary>Returns how many steps a script holds.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_i2c_script_len(IntPtr script);

    /// <summary>Releases a script. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_i2c_script_free(IntPtr script);

    /// <summary>Opens the kernel's I2C adapter.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_i2c_bus_open(string path, out IntPtr outBus);

    /// <summary>Creates a simulated bus with no parts on it yet.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_bus_simulated();

    /// <summary>Puts a copy of a part on a simulated bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_bus_attach(IntPtr bus, IntPtr part);

    /// <summary>Creates a bus that plays a copy of a script.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_bus_scripted(IntPtr script);

    /// <summary>Returns what answers on a bus.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_i2c_bus_kind(IntPtr bus);

    /// <summary>Writes bytes to a part, in one transaction.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_bus_write(
        IntPtr bus,
        byte address,
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Reads bytes from a part, in one transaction.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_bus_read(
        IntPtr bus,
        byte address,
        Span<byte> outBytes,
        nuint len);

    /// <summary>Writes bytes and reads the reply in one transaction.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_bus_write_read(
        IntPtr bus,
        byte address,
        ReadOnlySpan<byte> bytes,
        nuint len,
        Span<byte> outBytes,
        nuint outLen);

    /// <summary>Returns how many transfers have been made on a bus.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_i2c_bus_transfers(IntPtr bus);

    /// <summary>Reports how many steps a scripted bus has left.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_i2c_bus_remaining(IntPtr bus, out nuint outRemaining);

    /// <summary>Returns how long the drivers on a bus have asked to wait, in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_i2c_bus_waited_micros(IntPtr bus);

    /// <summary>Copies what a simulated part holds now, or returns null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_i2c_bus_part(IntPtr bus, byte address);

    /// <summary>Releases the caller's share of a bus. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_i2c_bus_free(IntPtr bus);
}
