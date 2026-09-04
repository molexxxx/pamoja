using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>Reads and releases the owned strings the C ABI produces.</summary>
public static class OwnedString
{
    /// <summary>Copies an owned string out and releases it.</summary>
    /// <param name="text">The native string handle.</param>
    /// <returns>The string.</returns>
    /// <exception cref="PamojaException">The native call produced no string.</exception>
    public static string Read(IntPtr text)
    {
        string? read = ReadOrNull(text);
        return read ?? throw new PamojaException(
            PamojaCore.LastError() ?? "the call produced no string");
    }

    /// <summary>Copies an owned string out and releases it, allowing none.</summary>
    /// <param name="text">The native string handle, which may be null.</param>
    /// <returns>The string, or <c>null</c> when the call produced none.</returns>
    public static string? ReadOrNull(IntPtr text)
    {
        if (text == IntPtr.Zero)
        {
            return null;
        }

        try
        {
            return Marshal.PtrToStringUTF8(NativeMethods.pamoja_string_data(text));
        }
        finally
        {
            NativeMethods.pamoja_string_free(text);
        }
    }
}
