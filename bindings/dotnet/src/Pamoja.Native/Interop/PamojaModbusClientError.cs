using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>Why a Modbus client call failed, as the C ABI reports it.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaModbusClientError
{
    /// <summary>One of the <c>ModbusClient</c> kinds on <see cref="NativeMethods"/>, 0 on success.</summary>
    public byte Kind;

    /// <summary>The unit the call asked.</summary>
    public byte Unit;

    /// <summary>The function the call asked, for an exception or a reply to another function.</summary>
    public byte Function;

    /// <summary>What the reply named instead: the unit that answered, or the function it answered.</summary>
    public byte Found;

    /// <summary>The exception code the device answered with.</summary>
    public byte Exception;

    /// <summary>How many bytes of a reply had arrived before a timeout.</summary>
    public uint Received;
}
