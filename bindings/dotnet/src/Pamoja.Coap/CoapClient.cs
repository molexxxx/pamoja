using System.Runtime.InteropServices;

using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Coap;

/// <summary>Whether a CoAP request is acknowledged and retried.</summary>
public enum Reliability
{
    /// <summary>Fire and forget: the request is sent once and not acknowledged.</summary>
    NonConfirmable = 0,

    /// <summary>The request is acknowledged, and retransmitted until an ACK arrives.</summary>
    Confirmable = 1,
}

/// <summary>The settings a CoAP endpoint is built from.</summary>
public sealed class CoapClientOptions
{
    /// <summary>Gets the peer hostname or IP address.</summary>
    public required string Host { get; init; }

    /// <summary>Gets the peer UDP port, conventionally 5683 for plaintext CoAP.</summary>
    public required ushort Port { get; init; }

    /// <summary>Gets the local address to bind, or <c>null</c> for the default.</summary>
    public string? Bind { get; init; }

    /// <summary>Gets whether requests are acknowledged and retried.</summary>
    public Reliability Reliability { get; init; } = Reliability.Confirmable;

    /// <summary>Gets how long to wait for the first acknowledgment, in milliseconds.</summary>
    /// <remarks>
    /// Zero selects RFC 7252's two seconds. Each wait after the first doubles, and the RFC
    /// forbids a first wait shorter than two seconds on a network without congestion control.
    /// </remarks>
    public uint AckTimeoutMs { get; init; }

    /// <summary>Gets how many times to retransmit an unacknowledged request.</summary>
    public uint MaxRetransmits { get; init; }

    /// <summary>Builds the native config and runs an action over it.</summary>
    /// <remarks>
    /// The strings live only for the call, which is all the native side needs:
    /// it copies what it keeps.
    /// </remarks>
    /// <typeparam name="TResult">What the action returns.</typeparam>
    /// <param name="action">The native call to make.</param>
    /// <returns>Whatever the action returned.</returns>
    internal TResult WithNativeConfig<TResult>(NativeConfigAction<TResult> action)
    {
        IntPtr host = Marshal.StringToCoTaskMemUTF8(Host);
        IntPtr bind = Bind is null ? IntPtr.Zero : Marshal.StringToCoTaskMemUTF8(Bind);
        try
        {
            PamojaCoapConfig config = new()
            {
                Host = host,
                Port = Port,
                Bind = bind,
                Reliability = (PamojaCoapReliability)NamedValue.Require(Reliability, nameof(Reliability)),
                AckTimeoutMs = AckTimeoutMs,
                MaxRetransmits = MaxRetransmits,
            };
            return action(ref config);
        }
        finally
        {
            Marshal.FreeCoTaskMem(host);
            if (bind != IntPtr.Zero)
            {
                Marshal.FreeCoTaskMem(bind);
            }
        }
    }
}

/// <summary>A native call taking the blittable CoAP config by reference.</summary>
/// <typeparam name="TResult">What the call returns.</typeparam>
/// <param name="config">The config to pass.</param>
/// <returns>Whatever the call returned.</returns>
internal delegate TResult NativeConfigAction<out TResult>(ref PamojaCoapConfig config);

/// <summary>A CoAP endpoint.</summary>
/// <remarks>
/// CoAP is the transport for links where MQTT is more than the budget allows: it
/// runs over UDP, its headers are a handful of bytes, and a node can fire a
/// reading and forget it rather than holding a session open.
///
/// Calls on one endpoint run one at a time, so a send made while a receive is
/// waiting runs once the receive returns.
/// </remarks>
public sealed class CoapClient : ILink, IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a disconnected endpoint from the given settings.</summary>
    /// <param name="options">The endpoint settings.</param>
    /// <exception cref="PamojaException">The native endpoint could not be created.</exception>
    /// <exception cref="ArgumentOutOfRangeException">The reliability is not one of the <see cref="Reliability"/> values.</exception>
    public CoapClient(CoapClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        _handle = options.WithNativeConfig((ref PamojaCoapConfig config) => NativeHandle.Create(
            NativeMethods.pamoja_coap_client_new(ref config),
            NativeMethods.pamoja_coap_client_free,
            "CoAP endpoint",
            serialized: true));
    }

    /// <summary>Binds the local socket so the endpoint can carry traffic.</summary>
    /// <exception cref="PamojaException">The socket could not be bound.</exception>
    public Task ConnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_coap_client_connect(handle)));

    /// <summary>Sends text to a resource path: words, or a number written out.</summary>
    /// <param name="topic">The resource path.</param>
    /// <param name="text">The text to send, as UTF-8.</param>
    /// <exception cref="PamojaException">The request could not be sent.</exception>
    public Task SendAsync(string topic, string text) =>
        SendAsync(topic, System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Sends a payload to a resource path.</summary>
    /// <param name="topic">The resource path.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <exception cref="PamojaException">The request could not be sent.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return _handle.UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_coap_client_send(
                    handle, topicPtr, bytes, (nuint)bytes.Length));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Observes a resource path, so messages published to it arrive.</summary>
    /// <param name="topic">The resource path.</param>
    /// <exception cref="PamojaException">The observation was refused.</exception>
    public Task SubscribeAsync(string topic) => _handle.UseAsync(handle =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(NativeMethods.pamoja_coap_client_subscribe(handle, topicPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message on an observed path.</summary>
    /// <returns>The message, or <c>null</c> once the endpoint is closed.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<TransportMessage?> ReceiveAsync() => _handle.UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_coap_client_recv(handle, out IntPtr message));
        return Messages.Take(message);
    });

    /// <summary>Waits a limited time for the next message on an observed path.</summary>
    /// <remarks>
    /// When the time runs out nothing is lost: a message arriving afterwards waits
    /// for the next receive.
    /// </remarks>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The message, or <c>null</c> once the endpoint is closed.</returns>
    /// <exception cref="TimeoutException">No message arrived in time.</exception>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<TransportMessage?> ReceiveAsync(TimeSpan timeout)
    {
        ulong milliseconds = Messages.Milliseconds(timeout);
        return _handle.UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_coap_client_recv_within(
                handle, milliseconds, out IntPtr message, out bool timedOut));
            return timedOut ? throw Messages.TimedOut(timeout) : Messages.Take(message);
        });
    }

    /// <summary>Reports whether the local socket is bound.</summary>
    /// <returns><c>true</c> when bound.</returns>
    public Task<bool> IsConnectedAsync() =>
        _handle.UseAsync(NativeMethods.pamoja_coap_client_is_connected);

    /// <summary>Releases the socket the endpoint holds.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task DisconnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_coap_client_disconnect(handle)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
