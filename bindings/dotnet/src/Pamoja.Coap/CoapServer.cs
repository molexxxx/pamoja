using System.Runtime.InteropServices;

using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Coap;

/// <summary>A CoAP server: the end that nodes send their readings to and observe their commands on.</summary>
/// <remarks>
/// A PUT or POST to a path matching one of its <see cref="SubscribeAsync"/> filters is
/// answered 2.04 Changed and delivered to <see cref="ReceiveAsync()"/>, and one to any
/// other path is answered 4.04 Not Found. <see cref="SendAsync(string, string)"/> sets
/// a resource's state, which a GET reads and every observer is notified of. A send
/// does not wait for a receive that is waiting, so a gateway sends commands while it
/// listens for readings; the other calls on one server run one at a time.
/// </remarks>
public sealed class CoapServer : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a server that will listen on a local address.</summary>
    /// <param name="bind">
    /// The local <c>host:port</c>, such as <c>0.0.0.0:5683</c>, or port 0 for a free one.
    /// </param>
    /// <exception cref="PamojaException">The native server could not be created.</exception>
    public CoapServer(string bind)
    {
        ArgumentNullException.ThrowIfNull(bind);
        IntPtr bindPtr = Marshal.StringToCoTaskMemUTF8(bind);
        try
        {
            _handle = NativeHandle.Create(
                NativeMethods.pamoja_coap_server_new(bindPtr),
                NativeMethods.pamoja_coap_server_free,
                "CoAP server");
        }
        finally
        {
            Marshal.FreeCoTaskMem(bindPtr);
        }
    }

    /// <summary>Gets the port the server listens on, or <c>null</c> while it is not connected.</summary>
    /// <remarks>This names the port the system chose when the server bound port 0.</remarks>
    public ushort? LocalPort =>
        _handle.Use(NativeMethods.pamoja_coap_server_local_port) is ushort port and not 0
            ? port
            : null;

    /// <summary>Gets whether the server holds a bound socket.</summary>
    public bool IsConnected => _handle.Use(NativeMethods.pamoja_coap_server_is_connected);

    /// <summary>Binds the socket and starts answering requests.</summary>
    /// <exception cref="PamojaException">The address could not be bound.</exception>
    public Task ConnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_coap_server_connect(handle)));

    /// <summary>Takes the readings sent to the paths a filter matches.</summary>
    /// <param name="filter">The filter, with <c>+</c> for one level and <c>#</c> for the rest.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task SubscribeAsync(string filter) => _handle.UseAsync(handle =>
    {
        IntPtr filterPtr = Marshal.StringToCoTaskMemUTF8(filter);
        try
        {
            Status.ThrowIfError(NativeMethods.pamoja_coap_server_subscribe(handle, filterPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(filterPtr);
        }
    });

    /// <summary>Sets a resource's state to text, such as a command, and notifies its observers.</summary>
    /// <param name="path">The resource's path.</param>
    /// <param name="text">Its new state, as UTF-8.</param>
    /// <exception cref="PamojaException">The server is not connected.</exception>
    public Task SendAsync(string path, string text) =>
        SendAsync(path, System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Sets a resource's state and notifies every observer of it.</summary>
    /// <param name="path">The resource's path.</param>
    /// <param name="payload">Its new state, which a GET now reads.</param>
    /// <exception cref="PamojaException">The server is not connected.</exception>
    public Task SendAsync(string path, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return _handle.UseAsync(handle =>
        {
            IntPtr pathPtr = Marshal.StringToCoTaskMemUTF8(path);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_coap_server_send(
                    handle, pathPtr, bytes, (nuint)bytes.Length));
            }
            finally
            {
                Marshal.FreeCoTaskMem(pathPtr);
            }
        });
    }

    /// <summary>Waits for the next reading sent to a path the server takes.</summary>
    /// <returns>The reading, carrying the path it was sent to.</returns>
    /// <exception cref="PamojaException">The server is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync() => _handle.UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_coap_server_recv(handle, out IntPtr message));
        return Messages.Take(message);
    });

    /// <summary>Waits a limited time for the next reading sent to a path the server takes.</summary>
    /// <remarks>
    /// When the time runs out nothing is lost: a reading arriving afterwards waits for
    /// the next receive.
    /// </remarks>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The reading.</returns>
    /// <exception cref="TimeoutException">No reading arrived in time.</exception>
    /// <exception cref="PamojaException">The server is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync(TimeSpan timeout)
    {
        ulong milliseconds = Messages.Milliseconds(timeout);
        return _handle.UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_coap_server_recv_within(
                handle, milliseconds, out IntPtr message, out bool timedOut));
            return timedOut ? throw Messages.TimedOut(timeout) : Messages.Take(message);
        });
    }

    /// <summary>Counts the clients observing a resource.</summary>
    /// <param name="path">The resource's path.</param>
    /// <returns>How many observers the resource has.</returns>
    public int Observers(string path)
    {
        IntPtr pathPtr = Marshal.StringToCoTaskMemUTF8(path);
        try
        {
            return checked((int)_handle.Use(handle =>
                NativeMethods.pamoja_coap_server_observers(handle, pathPtr)));
        }
        finally
        {
            Marshal.FreeCoTaskMem(pathPtr);
        }
    }

    /// <summary>Closes the socket, keeping the resources and filters.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task DisconnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_coap_server_disconnect(handle)));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
