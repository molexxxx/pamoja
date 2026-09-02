using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>A store-and-forward buffer.</summary>
/// <remarks>
/// The queue a node writes into while it has nowhere to send. An in-memory
/// buffer suits a test or a process that will not outlive it; a file-backed one
/// survives a reboot, which is what a node somewhere without reliable power
/// actually needs.
///
/// Handing a store to a ladder consumes it, because the ladder owns it from then
/// on, so a spent store throws rather than aliasing a buffer it no longer holds.
/// </remarks>
public sealed class Store : IDisposable
{
    private IntPtr _handle;

    /// <summary>Wraps a native store handle.</summary>
    private Store(IntPtr handle)
    {
        if (handle == IntPtr.Zero)
        {
            throw new PamojaException(PamojaCore.LastError() ?? "failed to open the store");
        }

        _handle = handle;
    }

    /// <summary>Whether this store is still holdable, or has been given away.</summary>
    public bool IsAvailable => _handle != IntPtr.Zero;

    /// <summary>Creates a buffer held in memory.</summary>
    /// <remarks>
    /// A full store refuses the next append rather than dropping anything, so a
    /// record is never lost without the caller being told.
    /// </remarks>
    /// <param name="capacity">The most records to hold, or 0 for no bound.</param>
    /// <returns>The buffer.</returns>
    public static Store Memory(int capacity = 0) =>
        new(NativeMethods.pamoja_store_memory((nuint)capacity));

    /// <summary>Opens a buffer backed by a directory, so it survives a restart.</summary>
    /// <param name="dir">The directory to hold records in; created if missing.</param>
    /// <returns>The buffer.</returns>
    /// <exception cref="PamojaException">The directory could not be opened.</exception>
    public static Store File(string dir)
    {
        IntPtr dirPtr = Marshal.StringToCoTaskMemUTF8(dir);
        try
        {
            return new Store(NativeMethods.pamoja_store_file(dirPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(dirPtr);
        }
    }

    /// <summary>Adds a record to the end of the buffer.</summary>
    /// <param name="record">The bytes to hold.</param>
    /// <exception cref="PamojaException">The store is full, or otherwise refused it.</exception>
    public Task AppendAsync(ReadOnlyMemory<byte> record)
    {
        byte[] bytes = record.ToArray();
        return Task.Run(() => PamojaCore.ThrowIfError(
            NativeMethods.pamoja_store_append(Live(), bytes, (nuint)bytes.Length)));
    }

    /// <summary>Reads the oldest record without removing it.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PeekAsync() => Task.Run(() =>
    {
        IntPtr record = IntPtr.Zero;
        PamojaCore.ThrowIfError(NativeMethods.pamoja_store_peek(Live(), out record));
        return record == IntPtr.Zero ? null : Codec.TakeBytes(record);
    });

    /// <summary>Removes and returns the oldest record.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PopAsync() => Task.Run(() =>
    {
        IntPtr record = IntPtr.Zero;
        PamojaCore.ThrowIfError(NativeMethods.pamoja_store_pop(Live(), out record));
        return record == IntPtr.Zero ? null : Codec.TakeBytes(record);
    });

    /// <summary>Reports how many records the buffer holds.</summary>
    /// <returns>The count.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> CountAsync() => Task.Run(() =>
    {
        nuint length = 0;
        PamojaCore.ThrowIfError(NativeMethods.pamoja_store_len(Live(), out length));
        return checked((int)length);
    });

    /// <summary>Sends every held record over a transport, oldest first.</summary>
    /// <remarks>
    /// A record is removed only once the transport has taken it, so a link that
    /// fails part-way leaves the rest of the queue intact for the next attempt.
    /// </remarks>
    /// <param name="transport">The transport to send over, borrowed not consumed.</param>
    /// <param name="topic">The topic to send to.</param>
    /// <returns>How many records went out.</returns>
    /// <exception cref="PamojaException">The link failed part-way through.</exception>
    public Task<int> DrainToAsync(Transport transport, string topic)
    {
        ArgumentNullException.ThrowIfNull(transport);
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                nuint sent = 0;
                PamojaCore.ThrowIfError(NativeMethods.pamoja_store_drain_to(
                    Live(), transport.Borrow(), topicPtr, out sent));
                return checked((int)sent);
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.pamoja_store_free(_handle);
            _handle = IntPtr.Zero;
        }
    }

    /// <summary>Hands the native handle on, leaving this one spent.</summary>
    internal IntPtr Take()
    {
        IntPtr handle = Live();
        _handle = IntPtr.Zero;
        return handle;
    }

    /// <summary>Returns the handle, refusing one that has been given away.</summary>
    private IntPtr Live() => _handle != IntPtr.Zero
        ? _handle
        : throw new PamojaException("this store was already given to a ladder");
}

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
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_rung(handle, transport.Take())));
    }

    /// <summary>Connects every rung, so a send can be tried against each in turn.</summary>
    /// <remarks>
    /// A rung that will not connect is left in the ladder: it may come back, and
    /// a send simply falls through it until it does.
    /// </remarks>
    public Task ConnectAsync() => Task.Run(() => PamojaCore.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_ladder_connect)));

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
                PamojaCore.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_ladder_send(
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
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_flush(handle, out sent)));
        return checked((int)sent);
    });

    /// <summary>Reports how many messages are waiting in the buffer.</summary>
    /// <returns>The count.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> BufferedAsync() => Task.Run(() =>
    {
        nuint count = 0;
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_ladder_buffered(handle, out count)));
        return checked((int)count);
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
