using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>The engine's own surface: what the runtime is, rather than what it can do.</summary>
public static class PamojaCore
{
    /// <summary>The version of the native pamoja library.</summary>
    public static string Version =>
        Marshal.PtrToStringUTF8(NativeMethods.pamoja_version()) ?? string.Empty;
}
