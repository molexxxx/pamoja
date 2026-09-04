using System.Runtime.InteropServices;

using Pamoja.Codec;
using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Modbus;

/// <summary>A received Modbus RTU frame whose CRC has been verified.</summary>
/// <example>
/// <code>
/// using var reply = Modbus.ParseFrame(received);
/// if (reply.Exception is null) Console.WriteLine(string.Join(", ", reply.Registers()));
/// </code>
/// </example>
public sealed class ModbusFrame : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a parsed frame handle.</summary>
    /// <param name="frame">The handle a native parse produced.</param>
    /// <exception cref="PamojaException">The handle is null.</exception>
    internal ModbusFrame(IntPtr frame)
    {
        _handle = NativeHandle.Create(
            frame, NativeMethods.pamoja_modbus_frame_free, "Modbus frame");
    }

    /// <summary>The unit address the frame is addressed to or came from.</summary>
    public byte Address => _handle.Use(NativeMethods.pamoja_modbus_frame_address);

    /// <summary>
    /// The function code. An exception response carries the request's code with its
    /// high bit set, as it appeared on the wire.
    /// </summary>
    public byte FunctionCode => _handle.Use(NativeMethods.pamoja_modbus_frame_function);

    /// <summary>
    /// The exception a device reported, or <c>null</c> when it served the request.
    /// </summary>
    public byte? Exception
    {
        get
        {
            byte code = _handle.Use(NativeMethods.pamoja_modbus_frame_exception);
            return code == 0 ? null : code;
        }
    }

    /// <summary>The protocol data unit: the function code and its data.</summary>
    public byte[] Pdu => _handle.Use(handle =>
    {
        int length = checked((int)NativeMethods.pamoja_modbus_frame_pdu_len(handle));
        byte[] pdu = new byte[length];
        if (length > 0)
        {
            Marshal.Copy(NativeMethods.pamoja_modbus_frame_pdu(handle), pdu, 0, length);
        }

        return pdu;
    });

    /// <summary>Reads the 16-bit registers out of a read-registers reply.</summary>
    /// <returns>The registers, in order.</returns>
    /// <exception cref="PamojaException">
    /// This is not a well-formed read-registers reply.
    /// </exception>
    public ushort[] Registers() => _handle.Use(handle =>
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_modbus_frame_registers(handle, out IntPtr registers));
        try
        {
            int count = checked((int)NativeMethods.pamoja_registers_len(registers));
            ushort[] values = new ushort[count];
            if (count > 0)
            {
                short[] signed = new short[count];
                Marshal.Copy(NativeMethods.pamoja_registers_data(registers), signed, 0, count);
                for (int index = 0; index < count; index++)
                {
                    values[index] = unchecked((ushort)signed[index]);
                }
            }

            return values;
        }
        finally
        {
            NativeMethods.pamoja_registers_free(registers);
        }
    });

    /// <summary>Reads the coils or discrete inputs out of a read-bits reply.</summary>
    /// <param name="count">How many bits to read, the quantity the request asked for.</param>
    /// <returns>One state per coil, in order.</returns>
    /// <exception cref="PamojaException">The reply does not carry that many bits.</exception>
    public bool[] Coils(ushort count) => _handle.Use(handle =>
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_modbus_frame_coils(handle, count, out IntPtr buffer));
        byte[] packed = Pamoja.Codec.Codec.TakeBytes(buffer);
        bool[] coils = new bool[packed.Length];
        for (int index = 0; index < packed.Length; index++)
        {
            coils[index] = packed[index] != 0;
        }

        return coils;
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
