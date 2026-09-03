using Pamoja.Codec;
using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Modbus;

/// <summary>Modbus RTU framing for RS485 field devices.</summary>
/// <remarks>
/// Modbus over RS485 is what cheap industrial sensing speaks: energy meters, soil
/// probes, water-quality transmitters, pump controllers. Each request builder
/// returns a complete frame with its CRC, ready to write to a port, and a reply
/// comes back through <see cref="ParseFrame"/> as an object that reads its own
/// values.
/// </remarks>
public static class Modbus
{
    /// <summary>Computes the CRC-16/MODBUS that every RTU frame ends with.</summary>
    /// <param name="bytes">The frame contents, without the trailing checksum.</param>
    /// <returns>The checksum.</returns>
    public static ushort Crc16(ReadOnlySpan<byte> bytes) =>
        NativeMethods.pamoja_modbus_crc16(bytes, (nuint)bytes.Length);

    /// <summary>Builds a read-coils request (function 0x01).</summary>
    /// <param name="address">The unit address to ask.</param>
    /// <param name="start">The address of the first coil.</param>
    /// <param name="count">How many coils to read.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] ReadCoils(byte address, ushort start, ushort count)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_modbus_read_coils(address, start, count, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a read-discrete-inputs request (function 0x02).</summary>
    /// <param name="address">The unit address to ask.</param>
    /// <param name="start">The address of the first input.</param>
    /// <param name="count">How many inputs to read.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] ReadDiscreteInputs(byte address, ushort start, ushort count)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_read_discrete_inputs(
            address, start, count, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a read-holding-registers request (function 0x03).</summary>
    /// <param name="address">The unit address to ask.</param>
    /// <param name="start">The address of the first register.</param>
    /// <param name="count">How many registers to read.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] ReadHoldingRegisters(byte address, ushort start, ushort count)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_read_holding_registers(
            address, start, count, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a read-input-registers request (function 0x04).</summary>
    /// <param name="address">The unit address to ask.</param>
    /// <param name="start">The address of the first register.</param>
    /// <param name="count">How many registers to read.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] ReadInputRegisters(byte address, ushort start, ushort count)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_read_input_registers(
            address, start, count, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a write-single-coil request (function 0x05).</summary>
    /// <param name="address">The unit address to write to.</param>
    /// <param name="coil">The coil address.</param>
    /// <param name="on">The state to write.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] WriteSingleCoil(byte address, ushort coil, bool on)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_modbus_write_single_coil(address, coil, on, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a write-single-register request (function 0x06).</summary>
    /// <param name="address">The unit address to write to.</param>
    /// <param name="register">The register address.</param>
    /// <param name="value">The 16-bit value to write.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] WriteSingleRegister(byte address, ushort register, ushort value)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_write_single_register(
            address, register, value, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a write-multiple-registers request (function 0x10).</summary>
    /// <param name="address">The unit address to write to.</param>
    /// <param name="start">The address of the first register.</param>
    /// <param name="values">The 16-bit values, at most 123 of them.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">
    /// There are no values, or more than one request can carry.
    /// </exception>
    public static byte[] WriteMultipleRegisters(
        byte address,
        ushort start,
        ReadOnlySpan<ushort> values)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_write_multiple_registers(
            address, start, values, (nuint)values.Length, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a write-multiple-coils request (function 0x0F).</summary>
    /// <param name="address">The unit address to write to.</param>
    /// <param name="start">The address of the first coil.</param>
    /// <param name="values">One state per coil, at most 1968 of them.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">
    /// There are no values, or more than one request can carry.
    /// </exception>
    public static byte[] WriteMultipleCoils(byte address, ushort start, ReadOnlySpan<bool> values)
    {
        byte[] packed = new byte[values.Length];
        for (int index = 0; index < values.Length; index++)
        {
            packed[index] = values[index] ? (byte)1 : (byte)0;
        }

        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_write_multiple_coils(
            address, start, packed, (nuint)packed.Length, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Builds a request from a raw function code and data.</summary>
    /// <param name="address">The unit address to send to.</param>
    /// <param name="functionCode">The function code byte.</param>
    /// <param name="data">The bytes that follow it, used verbatim.</param>
    /// <returns>The frame to send.</returns>
    /// <exception cref="PamojaException">The data is longer than a PDU may be.</exception>
    /// <remarks>The escape hatch for the function codes this SDK does not name.</remarks>
    public static byte[] Raw(byte address, byte functionCode, ReadOnlySpan<byte> data)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_raw(
            address, functionCode, data, (nuint)data.Length, out IntPtr buffer));
        return Pamoja.Codec.Codec.TakeBytes(buffer);
    }

    /// <summary>Parses a received RTU frame, verifying its CRC.</summary>
    /// <param name="bytes">The frame as it came off the wire, checksum included.</param>
    /// <returns>The validated frame, which reads its own registers and coils.</returns>
    /// <exception cref="PamojaException">
    /// The frame is truncated, oversized, or its CRC does not match its contents.
    /// </exception>
    public static ModbusFrame ParseFrame(ReadOnlySpan<byte> bytes)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_modbus_frame_parse(
            bytes, (nuint)bytes.Length, out IntPtr frame));
        return new ModbusFrame(frame);
    }
}
