using Pamoja.Codec;
using Pamoja.Native.Interop;

namespace Pamoja.Bus;

/// <summary>One endpoint on an event bus.</summary>
/// <remarks>
/// One publisher, many subscribers, inside a single process. It is how the parts
/// of a gateway talk to each other without knowing about each other, so a sampler
/// can announce a reading and whatever cares about readings picks it up.
///
/// An endpoint only sees events published after it existed, so subscribe before
/// publishing anything it needs to see.
/// </remarks>
public sealed class EventBus : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an event bus.</summary>
    /// <param name="capacity">
    /// How many events a slow subscriber may fall behind before it starts
    /// missing them.
    /// </param>
    /// <exception cref="PamojaException">The native bus could not be created.</exception>
    public EventBus(int capacity = 64)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_event_bus_new((nuint)capacity),
            NativeMethods.pamoja_event_bus_free,
            "event bus");
    }

    /// <summary>Wraps a native endpoint handle.</summary>
    private EventBus(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_event_bus_free, "event bus endpoint");
    }

    /// <summary>Takes another endpoint on the same bus.</summary>
    /// <returns>The new endpoint.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public EventBus Subscribe() =>
        new(_handle.Use(NativeMethods.pamoja_event_bus_subscribe));

    /// <summary>Publishes an event to every subscriber.</summary>
    /// <param name="payload">The event bytes.</param>
    /// <exception cref="PamojaException">The bus has shut down.</exception>
    public Task PublishAsync(ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() => Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_event_bus_publish(handle, bytes, (nuint)bytes.Length))));
    }

    /// <summary>Waits for the next event on this endpoint.</summary>
    /// <returns>The event, or <c>null</c> once the bus has closed.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]?> NextAsync() => Task.Run(() =>
    {
        IntPtr next = IntPtr.Zero;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_event_bus_next(handle, out next)));
        return next == IntPtr.Zero ? null : Pamoja.Codec.Codec.TakeBytes(next);
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
