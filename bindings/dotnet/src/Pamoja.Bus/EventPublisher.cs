using Pamoja.Native.Interop;

namespace Pamoja.Bus;

/// <summary>A publish-only handle to an event bus.</summary>
/// <remarks>
/// It has no queue of its own, so a part that only announces never fills a buffer
/// it does not read. Publishing never waits and may be called from any thread, so
/// an event handler, such as a pin edge or a timer callback, can announce without
/// awaiting anything.
/// </remarks>
public sealed class EventPublisher : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a bus with no subscribers yet, and a publisher on it.</summary>
    /// <param name="capacity">
    /// How many events a slow subscriber may fall behind before it starts missing
    /// them, rounded up to the next power of two and at most 1048576.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException">The capacity is negative.</exception>
    /// <exception cref="PamojaException">The native bus could not be created.</exception>
    public EventPublisher(int capacity = 64)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_event_publisher_new(EventBus.Capacity(capacity)),
            NativeMethods.pamoja_event_publisher_free,
            "event publisher");
    }

    /// <summary>Wraps a native publisher handle.</summary>
    internal EventPublisher(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_event_publisher_free, "event publisher");
    }

    /// <summary>Hands text to every current subscriber: an event name, or a reading written out.</summary>
    /// <param name="text">The event text, as UTF-8.</param>
    /// <returns>
    /// How many subscribers the event was handed to. An event published while no one
    /// is subscribed is dropped, and the count is 0.
    /// </returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public int Publish(string text) =>
        Publish(System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Hands an event to every current subscriber.</summary>
    /// <param name="payload">The event bytes.</param>
    /// <returns>
    /// How many subscribers the event was handed to. An event published while no one
    /// is subscribed is dropped, and the count is 0.
    /// </returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public int Publish(ReadOnlyMemory<byte> payload)
    {
        nuint reached = _handle.Use(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_event_publisher_publish(
                handle, payload.Span, (nuint)payload.Length, out nuint count));
            return count;
        });
        return (int)Math.Min(reached, (nuint)int.MaxValue);
    }

    /// <summary>Subscribes to the bus.</summary>
    /// <returns>An endpoint that sees events published from now on.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public EventBus Subscribe() =>
        new(_handle.Use(NativeMethods.pamoja_event_publisher_subscribe));

    /// <summary>Takes another publisher on the same bus, for another part that announces.</summary>
    /// <returns>A publisher with a lifetime of its own.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public EventPublisher Publisher() =>
        new(_handle.Use(NativeMethods.pamoja_event_publisher_clone));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
