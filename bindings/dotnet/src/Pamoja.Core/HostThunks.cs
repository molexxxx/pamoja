using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>The native-callable functions that forward host callbacks to an <see cref="ITransportHandlers"/>.</summary>
/// <remarks>
/// The user data is a <see cref="GCHandle"/> on the handlers object. An exception
/// never escapes into native code: it is reported as a status, with its message set
/// as the thread's last error so the caller sees it.
/// </remarks>
internal static unsafe class HostThunks
{
    /// <summary>Connects the handlers.</summary>
    /// <param name="userData">The handle on the handlers.</param>
    /// <returns>The status of the call.</returns>
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static PamojaStatus Connect(IntPtr userData) =>
        Run(userData, handlers => handlers.ConnectAsync());

    /// <summary>Sends a payload through the handlers.</summary>
    /// <param name="userData">The handle on the handlers.</param>
    /// <param name="topic">The null-terminated UTF-8 topic.</param>
    /// <param name="payload">The payload bytes.</param>
    /// <param name="payloadLen">The payload length.</param>
    /// <returns>The status of the call.</returns>
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static PamojaStatus Send(IntPtr userData, IntPtr topic, byte* payload, nuint payloadLen)
    {
        string topicText = Marshal.PtrToStringUTF8(topic) ?? string.Empty;
        byte[] bytes = new byte[checked((int)payloadLen)];
        if (bytes.Length > 0)
        {
            Marshal.Copy((IntPtr)payload, bytes, 0, bytes.Length);
        }

        return Run(userData, handlers => handlers.SendAsync(topicText, bytes));
    }

    /// <summary>Subscribes the handlers to a topic.</summary>
    /// <param name="userData">The handle on the handlers.</param>
    /// <param name="topic">The null-terminated UTF-8 topic filter.</param>
    /// <returns>The status of the call.</returns>
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static PamojaStatus Subscribe(IntPtr userData, IntPtr topic)
    {
        string topicText = Marshal.PtrToStringUTF8(topic) ?? string.Empty;
        return Run(userData, handlers => handlers.SubscribeAsync(topicText));
    }

    /// <summary>Waits for the next message the handlers deliver.</summary>
    /// <param name="userData">The handle on the handlers.</param>
    /// <param name="outMessage">Receives a message handle, or null once the link ended.</param>
    /// <returns>The status of the call.</returns>
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static PamojaStatus Receive(IntPtr userData, IntPtr* outMessage)
    {
        *outMessage = IntPtr.Zero;
        try
        {
            var handlers = (IReceivingTransportHandlers)Target(userData);
            TransportMessage? message = handlers.ReceiveAsync().GetAwaiter().GetResult();
            if (message is null)
            {
                return PamojaStatus.Ok;
            }

            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(message.Topic);
            try
            {
                *outMessage = NativeMethods.pamoja_message_new(
                    topicPtr, message.Payload, (nuint)message.Payload.Length);
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }

            return *outMessage == IntPtr.Zero ? PamojaStatus.InvalidArgument : PamojaStatus.Ok;
        }
        catch (Exception error)
        {
            return Fail(error);
        }
    }

    /// <summary>Releases the handle on the handlers.</summary>
    /// <param name="userData">The handle on the handlers.</param>
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void Release(IntPtr userData) => GCHandle.FromIntPtr(userData).Free();

    private static ITransportHandlers Target(IntPtr userData) =>
        (ITransportHandlers)(GCHandle.FromIntPtr(userData).Target
            ?? throw new InvalidOperationException("the transport handlers were collected"));

    private static PamojaStatus Run(IntPtr userData, Func<ITransportHandlers, Task> call)
    {
        try
        {
            call(Target(userData)).GetAwaiter().GetResult();
            return PamojaStatus.Ok;
        }
        catch (Exception error)
        {
            return Fail(error);
        }
    }

    private static PamojaStatus Fail(Exception error)
    {
        IntPtr message = Marshal.StringToCoTaskMemUTF8(error.Message);
        try
        {
            NativeMethods.pamoja_last_error_set(message);
        }
        finally
        {
            Marshal.FreeCoTaskMem(message);
        }

        return error is PamojaException ? PamojaStatus.Other : PamojaStatus.Transport;
    }
}
