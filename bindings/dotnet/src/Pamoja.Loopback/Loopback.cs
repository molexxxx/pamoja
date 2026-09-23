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

    /// <summary>Gets or sets whether the broker is in reach.</summary>
    /// <remarks>
    /// Set it to <c>false</c> to put every link on the broker out of range at once,
    /// links a ladder owns included: they fail to connect, send, or subscribe until
    /// it is <c>true</c> again, keeping their connections and filters through the
    /// outage.
    /// </remarks>
    public bool Reachable
    {
        get => _handle.Use(NativeMethods.pamoja_loopback_broker_is_reachable);
        set => Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_loopback_broker_set_reachable(handle, value)));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>One in-process link to a broker.</summary>
/// <remarks>
/// Calls on one link run one at a time, so a send made while a receive is waiting
/// runs once the receive returns. A task that listens should have a link of its own.
/// </remarks>
public sealed class LoopbackTransport : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a native link handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    internal LoopbackTransport(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_loopback_transport_free, "loopback link", serialized: true);
    }

    /// <summary>Marks this link connected so it will carry traffic.</summary>
    /// <remarks>A link that was disconnected keeps its subscriptions when it connects again.</remarks>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task ConnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_loopback_transport_connect(handle)));

    /// <summary>Publishes text to a topic on the broker: words, or a number written out.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="text">The text to publish, as UTF-8.</param>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task SendAsync(string topic, string text) =>
        SendAsync(topic, System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Publishes a payload to a topic on the broker.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to publish.</param>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return _handle.UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_loopback_transport_send(
                    handle, topicPtr, bytes, (nuint)bytes.Length));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Subscribes this link to a topic.</summary>
    /// <param name="topic">The topic to subscribe to.</param>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task SubscribeAsync(string topic) => _handle.UseAsync(handle =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_loopback_transport_subscribe(handle, topicPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message on a subscribed topic.</summary>
    /// <remarks>
    /// A connected link never ends on its own, so this waits until a message arrives.
    /// Use <see cref="ReceiveAsync(TimeSpan)"/> to stop waiting.
    /// </remarks>
    /// <returns>The message.</returns>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync() => _handle.UseAsync(handle =>
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_loopback_transport_recv(handle, out IntPtr message));
        return Messages.Take(message);
    });

    /// <summary>Waits a limited time for the next message on a subscribed topic.</summary>
    /// <remarks>
    /// When the time runs out nothing is lost: a message arriving afterwards waits
    /// for the next receive.
    /// </remarks>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The message.</returns>
    /// <exception cref="TimeoutException">No message arrived in time.</exception>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync(TimeSpan timeout)
    {
        ulong milliseconds = Messages.Milliseconds(timeout);
        return _handle.UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_loopback_transport_recv_within(
                handle, milliseconds, out IntPtr message, out bool timedOut));
            return timedOut ? throw Messages.TimedOut(timeout) : Messages.Take(message);
        });
    }

    /// <summary>Reports whether this link is connected.</summary>
    /// <returns><c>true</c> when connected.</returns>
    public Task<bool> IsConnectedAsync() =>
        _handle.UseAsync(NativeMethods.pamoja_loopback_transport_is_connected);

    /// <summary>Marks this link disconnected, so sends over it fail.</summary>
    /// <remarks>Messages queued for it are dropped; its subscriptions are kept.</remarks>
    public Task DisconnectAsync() =>
        _handle.UseAsync(NativeMethods.pamoja_loopback_transport_disconnect);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
