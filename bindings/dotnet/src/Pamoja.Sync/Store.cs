using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

using Pamoja.Codec;
using Pamoja.Core;

namespace Pamoja.Sync;

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
            throw new PamojaException(Status.LastError() ?? "failed to open the store");
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
        return Task.Run(() => Status.ThrowIfError(
            NativeMethods.pamoja_store_append(Live(), bytes, (nuint)bytes.Length)));
    }

    /// <summary>Reads the oldest record without removing it.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PeekAsync() => Task.Run(() =>
    {
        IntPtr record = IntPtr.Zero;
        Status.ThrowIfError(NativeMethods.pamoja_store_peek(Live(), out record));
        return record == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(record);
    });

    /// <summary>Removes and returns the oldest record.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PopAsync() => Task.Run(() =>
    {
        IntPtr record = IntPtr.Zero;
        Status.ThrowIfError(NativeMethods.pamoja_store_pop(Live(), out record));
        return record == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(record);
    });

    /// <summary>Reports how many records the buffer holds.</summary>
    /// <returns>The count.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> CountAsync() => Task.Run(() =>
    {
        nuint length = 0;
        Status.ThrowIfError(NativeMethods.pamoja_store_len(Live(), out length));
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
                Status.ThrowIfError(NativeMethods.pamoja_store_drain_to(
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
    public IntPtr Take()
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
