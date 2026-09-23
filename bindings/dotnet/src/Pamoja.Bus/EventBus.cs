using Pamoja.Native.Interop;

namespace Pamoja.Bus;

/// <summary>One endpoint on an event bus.</summary>
/// <remarks>
/// One publisher, many subscribers, inside a single process. It is how the parts
/// of a gateway talk to each other without knowing about each other, so a sampler
/// can announce a reading and whatever cares about readings picks it up.
///
/// An endpoint both publishes and receives, and it receives what it publishes
/// itself. It only sees events published after it existed, so subscribe before
/// publishing anything it needs to see. Only <see cref="NextAsync()"/> and
/// <see cref="NextTextAsync()"/> wait: publishing and subscribing return at once,
/// even while a wait on the same endpoint is open.
/// </remarks>
public sealed class EventBus : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an event bus.</summary>
    /// <param name="capacity">
    /// How many events a slow subscriber may fall behind before it starts missing
    /// them, rounded up to the next power of two and at most 1048576.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException">The capacity is negative.</exception>
    /// <exception cref="PamojaException">The native bus could not be created.</exception>
    public EventBus(int capacity = 64)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_event_bus_new(Capacity(capacity)),
            NativeMethods.pamoja_event_bus_free,
            "event bus");
    }

    /// <summary>Wraps a native endpoint handle.</summary>
    internal EventBus(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_event_bus_free, "event bus endpoint");
    }

    /// <summary>How many events this endpoint lost by falling behind, as of its last completed wait.</summary>
    public long Missed
    {
        get
        {
            ulong missed = _handle.Use(NativeMethods.pamoja_event_bus_missed);
            return (long)Math.Min(missed, long.MaxValue);
        }
    }

    /// <summary>Takes another endpoint on the same bus.</summary>
    /// <returns>An endpoint that sees events published from now on.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public EventBus Subscribe() =>
        new(_handle.Use(NativeMethods.pamoja_event_bus_subscribe));

    /// <summary>Takes a publish-only handle on the same bus, for a part that announces and never reads.</summary>
    /// <returns>The publisher.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public EventPublisher Publisher() =>
        new(_handle.Use(NativeMethods.pamoja_event_bus_publisher));

    /// <summary>Publishes text to every subscriber, this endpoint included: an event name, or a reading written out.</summary>
    /// <remarks>
    /// It never waits: a subscriber that has fallen behind loses its oldest event
    /// rather than holding up the publisher.
    /// </remarks>
    /// <param name="text">The event text, as UTF-8.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public void Publish(string text) =>
        Publish(System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Publishes an event to every subscriber, this endpoint included.</summary>
    /// <remarks>
    /// It never waits: a subscriber that has fallen behind loses its oldest event
    /// rather than holding up the publisher.
    /// </remarks>
    /// <param name="payload">The event bytes.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public void Publish(ReadOnlyMemory<byte> payload) =>
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_event_bus_publish(handle, payload.Span, (nuint)payload.Length)));

    /// <summary>Waits for the next event on this endpoint.</summary>
    /// <remarks>
    /// The wait lasts until an event arrives, and it holds a thread-pool thread
    /// while it does. <see cref="NextAsync(TimeSpan)"/> gives up sooner.
    /// </remarks>
    /// <returns>The event.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]> NextAsync() => _handle.UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_event_bus_next(handle, out IntPtr next));
        return Taken(next);
    });

    /// <summary>Waits a limited time for the next event on this endpoint.</summary>
    /// <remarks>A wait that runs out takes no event, so the next one is left for the next call.</remarks>
    /// <param name="timeout">How long to wait, rounded up to the next millisecond.</param>
    /// <returns>The event.</returns>
    /// <exception cref="ArgumentOutOfRangeException">The time limit is negative.</exception>
    /// <exception cref="TimeoutException">No event arrived in time.</exception>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<byte[]> NextAsync(TimeSpan timeout)
    {
        ulong limit = Milliseconds(timeout);
        return _handle.UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_event_bus_next_within(
                handle, limit, out IntPtr next, out bool timedOut));
            return timedOut
                ? throw new TimeoutException($"no event arrived within {limit} ms")
                : Taken(next);
        });
    }

    /// <summary>Waits for the next event on this endpoint, as text.</summary>
    /// <returns>The event, decoded as UTF-8.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public async Task<string> NextTextAsync() =>
        System.Text.Encoding.UTF8.GetString(await NextAsync().ConfigureAwait(false));

    /// <summary>Waits a limited time for the next event on this endpoint, as text.</summary>
    /// <param name="timeout">How long to wait, rounded up to the next millisecond.</param>
    /// <returns>The event, decoded as UTF-8.</returns>
    /// <exception cref="ArgumentOutOfRangeException">The time limit is negative.</exception>
    /// <exception cref="TimeoutException">No event arrived in time.</exception>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public async Task<string> NextTextAsync(TimeSpan timeout) =>
        System.Text.Encoding.UTF8.GetString(await NextAsync(timeout).ConfigureAwait(false));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Checks a capacity and converts it to the native width.</summary>
    internal static nuint Capacity(int capacity) => capacity < 0
        ? throw new ArgumentOutOfRangeException(
            nameof(capacity), capacity, "a capacity cannot be negative")
        : (nuint)capacity;

    /// <summary>Copies an event out of its native buffer.</summary>
    private static byte[] Taken(IntPtr next) => next == IntPtr.Zero
        ? throw new PamojaException("the event bus closed")
        : Pamoja.Codec.Codec.TakeBytes(next);

    /// <summary>Converts a time limit to the whole milliseconds a native call takes.</summary>
    private static ulong Milliseconds(TimeSpan timeout) => timeout < TimeSpan.Zero
        ? throw new ArgumentOutOfRangeException(
            nameof(timeout), timeout, "a time limit cannot be negative")
        : (ulong)Math.Ceiling(timeout.TotalMilliseconds);
}
