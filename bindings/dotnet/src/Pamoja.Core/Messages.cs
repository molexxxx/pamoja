using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>Reading a native message handle and releasing it.</summary>
public static class Messages
{
    /// <summary>Copies a message out and releases the handle.</summary>
    /// <param name="message">The handle, or null when nothing arrived.</param>
    /// <returns>The message, or <c>null</c> when the handle was null.</returns>
    public static TransportMessage? Take(IntPtr message)
    {
        if (message == IntPtr.Zero)
        {
            return null;
        }

        try
        {
            string topic =
                Marshal.PtrToStringUTF8(NativeMethods.pamoja_message_topic(message)) ?? string.Empty;
            int length = checked((int)NativeMethods.pamoja_message_payload_len(message));
            byte[] payload = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_message_payload(message), payload, 0, length);
            }

            return new TransportMessage(topic, payload);
        }
        finally
        {
            NativeMethods.pamoja_message_free(message);
        }
    }

    /// <summary>Converts a receive time limit to the whole milliseconds a native call takes.</summary>
    /// <param name="timeout">The time limit, rounded up to the next millisecond.</param>
    /// <returns>The milliseconds.</returns>
    /// <exception cref="ArgumentOutOfRangeException">The time limit is negative.</exception>
    public static ulong Milliseconds(TimeSpan timeout) => timeout < TimeSpan.Zero
        ? throw new ArgumentOutOfRangeException(
            nameof(timeout), timeout, "a time limit cannot be negative")
        : (ulong)Math.Ceiling(timeout.TotalMilliseconds);

    /// <summary>Builds the exception a receive throws when its time runs out.</summary>
    /// <param name="timeout">The time limit that ran out.</param>
    /// <returns>The exception.</returns>
    public static TimeoutException TimedOut(TimeSpan timeout) =>
        new($"no message arrived within {Milliseconds(timeout)} ms");
}
