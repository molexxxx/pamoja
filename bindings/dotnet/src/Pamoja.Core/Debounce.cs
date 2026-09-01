using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Stops a flickering input from acting until it has settled.</summary>
/// <remarks>A float switch or a contact bounces as it changes; requiring several agreeing readings keeps one bounce from starting a pump.</remarks>
public sealed class Debounce : IDisposable
{
    private readonly NativeHandle _handle;

    private Debounce(IntPtr handle)
    {
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_debounce_free, "debouncer");
    }

    /// <summary>Creates a debouncer at the given settling length.</summary>
    /// <param name="samples">How many agreeing readings are needed to change state.</param>
    /// <param name="initial">The state to start in.</param>
    public Debounce(ushort samples, bool initial)
        : this(NativeMethods.pamoja_debounce_new(samples, initial))
    {
    }

    /// <summary>Feeds a raw reading in and returns the settled state.</summary>
    /// <param name="raw">The latest unfiltered reading.</param>
    /// <returns>The debounced state.</returns>
    public bool Update(bool raw) =>
        _handle.Use(handle => NativeMethods.pamoja_debounce_update(handle, raw));

    /// <summary>Gets the settled state.</summary>
    public bool State =>
        _handle.Use(NativeMethods.pamoja_debounce_state);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
