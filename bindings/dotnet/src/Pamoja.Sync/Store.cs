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
/// Calls on one store run one at a time.
/// </remarks>
public sealed class Store : IDisposable
{
    private const string Spent = "this store was already given to a ladder";
    private const string Busy = "this store is busy with a call";

    private NativeHandle? _handle;

    /// <summary>Wraps a native store handle.</summary>
    private Store(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_store_free, "store", serialized: true);
    }

    /// <summary>Whether this store is still holdable, or has been given away.</summary>
    public bool IsAvailable => _handle is { IsClosed: false };

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

    /// <summary>Adds text to the end of the buffer: a reading or a line written out.</summary>
    /// <param name="text">The text to hold, as UTF-8.</param>
    /// <exception cref="PamojaException">The store is full, or otherwise refused it.</exception>
    public Task AppendAsync(string text) =>
        AppendAsync(System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Adds a record to the end of the buffer.</summary>
    /// <param name="record">The bytes to hold.</param>
    /// <exception cref="PamojaException">The store is full, or otherwise refused it.</exception>
    public Task AppendAsync(ReadOnlyMemory<byte> record)
    {
        byte[] bytes = record.ToArray();
        return Live().UseAsync(handle => Status.ThrowIfError(
            NativeMethods.pamoja_store_append(handle, bytes, (nuint)bytes.Length)));
    }

    /// <summary>Reads the oldest record without removing it.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PeekAsync() => Live().UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_store_peek(handle, out IntPtr record));
        return record == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(record);
    });

    /// <summary>Removes and returns the oldest record.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> PopAsync() => Live().UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_store_pop(handle, out IntPtr record));
        return record == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(record);
    });

    /// <summary>Reads the oldest record as text, without removing it.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public async Task<string?> PeekTextAsync() =>
        await PeekAsync() is { } record ? System.Text.Encoding.UTF8.GetString(record) : null;

    /// <summary>Removes and returns the oldest record as text.</summary>
    /// <returns>The record, or <c>null</c> when the buffer is empty.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public async Task<string?> PopTextAsync() =>
        await PopAsync() is { } record ? System.Text.Encoding.UTF8.GetString(record) : null;

    /// <summary>Reports how many records the buffer holds.</summary>
    /// <returns>The count.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<int> CountAsync() => Live().UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_store_len(handle, out nuint length));
        return checked((int)length);
    });

    /// <summary>Sends every held record over a transport, oldest first.</summary>
    /// <remarks>
    /// A record is removed only once the transport has taken it, so a link that
    /// fails part-way leaves the rest of the queue intact for the next attempt. The
    /// drain waits for any call already running on the transport.
    /// </remarks>
    /// <param name="transport">The transport to send over, borrowed not consumed.</param>
    /// <param name="topic">The topic to send to.</param>
    /// <returns>How many records went out.</returns>
    /// <exception cref="PamojaException">The link failed part-way through.</exception>
    public Task<int> DrainToAsync(Transport transport, string topic)
    {
        ArgumentNullException.ThrowIfNull(transport);
        NativeHandle store = Live();
        return transport.LendAsync(link => store.UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_store_drain_to(
                    handle, link, topicPtr, out nuint sent));
                return checked((int)sent);
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        }));
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        _handle?.Dispose();
        _handle = null;
    }

    /// <summary>Hands the native handle on, leaving this one spent.</summary>
    /// <returns>The pointer the caller now owns.</returns>
    /// <exception cref="PamojaException">
    /// This store was already given away, or a call on it is still running.
    /// </exception>
    public IntPtr Take()
    {
        IntPtr pointer = Live().Take(Busy);
        _handle = null;
        return pointer;
    }

    /// <summary>Returns the handle, refusing one that has been given away.</summary>
    private NativeHandle Live() => _handle is { IsClosed: false } handle
        ? handle
        : throw new PamojaException(Spent);
}
