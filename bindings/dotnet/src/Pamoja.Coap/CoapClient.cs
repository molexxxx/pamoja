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

    /// <summary>Gets how long to wait for an acknowledgement, in milliseconds.</summary>
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
                Reliability = (PamojaCoapReliability)Reliability,
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
/// </remarks>
public sealed class CoapClient : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a disconnected endpoint from the given settings.</summary>
    /// <param name="options">The endpoint settings.</param>
    /// <exception cref="PamojaException">The native endpoint could not be created.</exception>
    public CoapClient(CoapClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        _handle = options.WithNativeConfig((ref PamojaCoapConfig config) => NativeHandle.Create(
            NativeMethods.pamoja_coap_client_new(ref config),
            NativeMethods.pamoja_coap_client_free,
            "CoAP endpoint"));
    }

    /// <summary>Binds the local socket so the endpoint can carry traffic.</summary>
    /// <exception cref="PamojaException">The socket could not be bound.</exception>
    public Task ConnectAsync() => Task.Run(() => Status.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_coap_client_connect)));

    /// <summary>Sends a payload to a resource path.</summary>
    /// <param name="topic">The resource path.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <exception cref="PamojaException">The request could not be sent.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(_handle.Use(handle =>
                    NativeMethods.pamoja_coap_client_send(
                        handle, topicPtr, bytes, (nuint)bytes.Length)));
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
    public Task SubscribeAsync(string topic) => Task.Run(() =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_coap_client_subscribe(handle, topicPtr)));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message on an observed path.</summary>
    /// <returns>The message, or <c>null</c> once the endpoint is closed.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<TransportMessage?> ReceiveAsync() => Task.Run(() =>
    {
        IntPtr message = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_coap_client_recv(handle, out message)));
        return Messages.Take(message);
    });

    /// <summary>Reports whether the local socket is bound.</summary>
    /// <returns><c>true</c> when bound.</returns>
    public Task<bool> IsConnectedAsync() => Task.Run(() =>
        _handle.Use(NativeMethods.pamoja_coap_client_is_connected));

    /// <summary>Releases the socket the endpoint holds.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task DisconnectAsync() => Task.Run(() => Status.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_coap_client_disconnect)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
