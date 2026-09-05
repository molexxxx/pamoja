using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>Byte buffers a native call hands back for the caller to own.</summary>
/// <remarks>
/// Several native calls return a length-prefixed buffer rather than writing into a
/// span, because the length is not known until the call returns. The handle owns
/// native memory, so it has to be released whether the copy succeeds or throws.
/// </remarks>
public static class OwnedBuffer
{
    /// <summary>Copies a native byte buffer out and releases it.</summary>
    /// <param name="buffer">The buffer handle a native call produced.</param>
    /// <returns>The buffer's contents.</returns>
    public static byte[] Take(IntPtr buffer)
    {
        try
        {
            int length = checked((int)NativeMethods.pamoja_buffer_len(buffer));
            byte[] bytes = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_buffer_data(buffer), bytes, 0, length);
            }

            return bytes;
        }
        finally
        {
            NativeMethods.pamoja_buffer_free(buffer);
        }
    }
}
