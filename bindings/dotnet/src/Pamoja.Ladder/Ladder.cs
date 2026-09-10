using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

using Pamoja.Core;
using Pamoja.Sync;

namespace Pamoja.Ladder;

/// <summary>What became of a message handed to a ladder.</summary>
public enum Delivery
{
    /// <summary>A rung took the message and it is on its way.</summary>
    Sent = 0,

    /// <summary>No rung would take it, so it is in the buffer awaiting a flush.</summary>
    Buffered = 1,
}

/// <summary>An ordered set of transports backed by an offline buffer.</summary>
/// <remarks>
/// A ladder is the answer to a node that has more than one way to reach the
/// network and no single one that always works: rungs are tried in the order they
/// were added, cheapest first, and a message no rung accepts goes into a buffer
/// rather than being lost.
/// </remarks>
public sealed class Ladder : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a ladder with no rungs, buffering into a store.</summary>
    /// <param name="store">The buffer, consumed by this call.</param>
    /// <exception cref="PamojaException">The native ladder could not be created.</exception>
    public Ladder(Store store)
    {
        ArgumentNullException.ThrowIfNull(store);
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_ladder_new(store.Take()),
            NativeMethods.pamoja_ladder_free,
            "ladder");
    }

    /// <summary>Adds a rung, which is tried after the rungs already added.</summary>
    /// <remarks>
    /// Add the cheapest, most-preferred link first and the costliest fallback
    /// last, because a send takes the first rung that accepts it.
    /// </remarks>
    /// <param name="transport">The transport to add, consumed by this call.</param>
    /// <exception cref="PamojaException">The transport was already handed on.</exception>
    public void Rung(Transport transport)
    {
        ArgumentNullException.ThrowIfNull(transport);
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_rung(handle, transport.Take())));
    }

    /// <summary>Connects every rung, so a send can be tried against each in turn.</summary>
    /// <remarks>
    /// A rung that will not connect is left in the ladder: it may come back, and
    /// a send simply falls through it until it does.
    /// </remarks>
    public Task ConnectAsync() => Task.Run(() => Status.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_ladder_connect)));

    /// <summary>Sends text, buffering it if no rung takes it: words, or a number written out.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="text">The text to send, as UTF-8.</param>
    /// <returns>Whether the message went out or was buffered.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<Delivery> SendAsync(string topic, string text) =>
        SendAsync(topic, System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Sends a payload, buffering it if no rung takes it.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <returns>Whether the message went out or was buffered.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<Delivery> SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                PamojaDelivery delivery = PamojaDelivery.Buffered;
                Status.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_ladder_send(
                    handle, topicPtr, bytes, (nuint)bytes.Length, out delivery)));
                return (Delivery)delivery;
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Replays the buffer over the rungs, oldest message first.</summary>
    /// <returns>How many buffered messages went out.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> FlushAsync() => Task.Run(() =>
    {
        nuint sent = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_flush(handle, out sent)));
        return checked((int)sent);
    });

    /// <summary>Reports how many messages are waiting in the buffer.</summary>
    /// <returns>The count.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> BufferedAsync() => Task.Run(() =>
    {
        nuint count = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_buffered(handle, out count)));
        return checked((int)count);
    });

    /// <summary>Subscribes every rung that listens to a topic.</summary>
    /// <remarks>
    /// The filter is kept for the life of the ladder: a rung that is down when it
    /// is placed receives it when it next connects, so subscribing before
    /// <see cref="ConnectAsync"/> is fine and never fails for want of a link.
    /// </remarks>
    /// <param name="topic">The topic filter, in the syntax the rungs understand.</param>
    /// <exception cref="PamojaException">Every connected rung refused the filter.</exception>
    public Task SubscribeAsync(string topic) => Task.Run(() =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_ladder_subscribe(handle, topicPtr)));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message from any rung that listens, whichever delivers first.</summary>
    /// <remarks>
    /// The ladder is held while waiting, so a send from elsewhere waits behind the receive.
    /// </remarks>
    /// <returns>The message.</returns>
    /// <exception cref="PamojaException">
    /// No connected rung listens: none was added, the ladder is not connected, or
    /// every listening link has ended.
    /// </exception>
    public Task<TransportMessage?> ReceiveAsync() => Task.Run(() =>
    {
        IntPtr message = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_recv(handle, out message)));
        return Messages.Take(message);
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
