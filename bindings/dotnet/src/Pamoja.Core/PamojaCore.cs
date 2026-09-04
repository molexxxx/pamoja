using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>Top-level entry points for the pamoja SDK.</summary>
public static class PamojaCore
{
    /// <summary>The version of the native pamoja library.</summary>
    public static string Version =>
        Marshal.PtrToStringUTF8(NativeMethods.pamoja_version()) ?? string.Empty;

    /// <summary>
    /// Reads the calling thread's most recent native error message.
    /// </summary>
    /// <returns>The message, or <c>null</c> if none has been recorded.</returns>
    /// <remarks>
    /// The native last-error slot is thread-local, so this must be read on the same
    /// thread that made the failing call.
    /// </remarks>
    public static string? LastError()
    {
        IntPtr message = NativeMethods.pamoja_last_error_message();
        return message == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(message);
    }

    /// <summary>Throws a <see cref="PamojaException"/> when <paramref name="status"/> is not OK.</summary>
    /// <param name="status">The status returned by a native call.</param>
    /// <remarks>Call this on the same thread as the native call so the last-error message resolves.</remarks>
    public static void ThrowIfError(PamojaStatus status)
    {
        if (status != PamojaStatus.Ok)
        {
            throw new PamojaException(LastError() ?? $"pamoja call failed with status {status}");
        }
    }
}
