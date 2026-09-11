using Pamoja.Lora;
using Pamoja.Native.Interop;

namespace Pamoja.Radios;

/// <summary>Holds a radio silent for the off time a duty-cycle limit owes after each frame.</summary>
/// <remarks>
/// A regional plan limits the share of time a transmitter may hold a sub-band, so every
/// frame buys a stretch of silence in proportion to its airtime. The guard keeps its
/// times on the caller's clock, in microseconds, so it runs the same on a gateway and in
/// a test that steps time by hand.
/// </remarks>
public sealed class RadioDutyCycle : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a guard, ready to transmit at once.</summary>
    /// <param name="permille">
    /// The limit in parts per thousand, so <c>10</c> is 1%; <c>0</c> forbids transmitting
    /// and <c>1000</c> or more imposes no silence.
    /// </param>
    /// <exception cref="PamojaException">The native guard could not be created.</exception>
    public RadioDutyCycle(uint permille)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_radio_duty_cycle_new(permille),
            NativeMethods.pamoja_radio_duty_cycle_free,
            "duty-cycle guard");
        Permille = permille;
    }

    /// <summary>The limit in parts per thousand.</summary>
    public uint Permille { get; }

    /// <summary>
    /// The earliest time the next transmission may start, in microseconds on the caller's
    /// clock, or <c>null</c> when the limit forbids transmitting.
    /// </summary>
    public ulong? EarliestMicros =>
        Allowed(_handle.Use(NativeMethods.pamoja_radio_duty_cycle_earliest_us));

    /// <summary>Records a transmission and the silence it owes.</summary>
    /// <param name="startedMicros">When the transmission started, on the caller's clock.</param>
    /// <param name="link">The settings the frame was sent with.</param>
    /// <param name="payloadLength">The payload length in bytes.</param>
    /// <returns>The frame's time on air, in microseconds.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="payloadLength"/> is negative.</exception>
    public ulong Transmitted(ulong startedMicros, LoraLink link, int payloadLength)
    {
        ArgumentNullException.ThrowIfNull(link);
        ArgumentOutOfRangeException.ThrowIfNegative(payloadLength);

        PamojaLoraLink settings = NativeLora.Link(link);
        return _handle.Use(handle => NativeMethods.pamoja_radio_duty_cycle_transmitted(
            handle, startedMicros, settings, (nuint)payloadLength));
    }

    /// <summary>Returns how long the radio must still stay silent.</summary>
    /// <param name="nowMicros">The current time on the caller's clock.</param>
    /// <returns>
    /// The remaining silence in microseconds, zero when a transmission may start, or
    /// <c>null</c> when the limit forbids transmitting.
    /// </returns>
    public ulong? WaitMicros(ulong nowMicros) =>
        Allowed(_handle.Use(handle => NativeMethods.pamoja_radio_duty_cycle_wait_us(handle, nowMicros)));

    /// <summary>Reports whether a transmission may start now.</summary>
    /// <param name="nowMicros">The current time on the caller's clock.</param>
    /// <returns><c>true</c> once the silence the last transmission owed has passed.</returns>
    public bool Ready(ulong nowMicros) =>
        _handle.Use(handle => NativeMethods.pamoja_radio_duty_cycle_ready(handle, nowMicros));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads the C ABI's never-allowed marker as <c>null</c>.</summary>
    /// <param name="micros">A time or duration in microseconds.</param>
    /// <returns>The value, or <c>null</c> for the marker.</returns>
    private static ulong? Allowed(ulong micros) => micros == ulong.MaxValue ? null : micros;
}
