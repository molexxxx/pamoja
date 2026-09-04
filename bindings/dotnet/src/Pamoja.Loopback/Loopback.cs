using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

using Pamoja.Core;

namespace Pamoja.Loopback;

/// <summary>An in-process broker.</summary>
/// <remarks>
/// Every transport built from one broker shares its traffic, so a message one
/// publishes reaches the others that subscribed to the topic. It exists so a
/// caller can exercise a whole message flow with no broker, no network, and no
/// hardware, which is what makes that flow testable from a unit test.
/// </remarks>
public sealed class LoopbackBroker : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a broker with no traffic.</summary>
    /// <exception cref="PamojaException">The native broker could not be created.</exception>
    public LoopbackBroker()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_loopback_broker_new(),
            NativeMethods.pamoja_loopback_broker_free,
            "loopback broker");
    }

    /// <summary>Creates a link to this broker, for driving directly.</summary>
    /// <returns>The link.</returns>
    public LoopbackTransport Link() =>
        new(_handle.Use(NativeMethods.pamoja_loopback_transport_new));

    /// <summary>Creates a link to this broker as a composable transport.</summary>
    /// <returns>The transport, ready to add as a rung.</returns>
    public Transport Rung() => new(
        _handle.Use(NativeMethods.pamoja_transport_loopback),
        "loopback transport");

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>One in-process link to a broker.</summary>
public sealed class LoopbackTransport : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a native link handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    internal LoopbackTransport(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_loopback_transport_free, "loopback link");
    }

    /// <summary>Marks this link connected so it will carry traffic.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task ConnectAsync() => Task.Run(() => Status.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_loopback_transport_connect)));

    /// <summary>Publishes a payload to a topic on the broker.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to publish.</param>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(_handle.Use(handle =>
                    NativeMethods.pamoja_loopback_transport_send(
                        handle, topicPtr, bytes, (nuint)bytes.Length)));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Subscribes this link to a topic.</summary>
    /// <param name="topic">The topic to subscribe to.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task SubscribeAsync(string topic) => Task.Run(() =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_loopback_transport_subscribe(handle, topicPtr)));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message on a subscribed topic.</summary>
    /// <returns>The message, or <c>null</c> once the link is closed.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<TransportMessage?> ReceiveAsync() => Task.Run(() =>
    {
        IntPtr message = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_loopback_transport_recv(handle, out message)));
        return Messages.Take(message);
    });

    /// <summary>Reports whether this link is connected.</summary>
    /// <returns><c>true</c> when connected.</returns>
    public Task<bool> IsConnectedAsync() => Task.Run(() =>
        _handle.Use(NativeMethods.pamoja_loopback_transport_is_connected));

    /// <summary>Marks this link disconnected, so sends over it fail.</summary>
    public Task DisconnectAsync() => Task.Run(() => _handle.Use(handle =>
    {
        NativeMethods.pamoja_loopback_transport_disconnect(handle);
        return 0;
    }));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
