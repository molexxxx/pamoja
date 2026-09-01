using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Packs float readings to a fixed precision, for a link charging per byte.</summary>
/// <remarks>
/// Full-precision floats are wasteful when a reading is only meaningful to two
/// decimal places. Rounding to that precision and delta-encoding the batch turns a
/// day of samples into something a metered link can afford.
/// </remarks>
/// <example>
/// <code>
/// var quantizer = new Quantizer(100); // keep two decimal places
/// byte[] packed = quantizer.Encode(new[] { 20.0f, 20.1f, 20.2f });
/// float[] readings = quantizer.Decode(packed); // to within 0.01
/// </code>
/// </example>
public sealed class Quantizer
{
    private readonly float _scale;

    /// <summary>Creates a quantizer at the given precision.</summary>
    /// <param name="scale">
    /// The multiplier applied before rounding; <c>100</c> keeps two decimal
    /// places. Must be positive and finite.
    /// </param>
    /// <exception cref="ArgumentOutOfRangeException">
    /// <paramref name="scale"/> is not positive and finite.
    /// </exception>
    public Quantizer(float scale)
    {
        if (!float.IsFinite(scale) || scale <= 0.0f)
        {
            throw new ArgumentOutOfRangeException(
                nameof(scale), scale, "scale must be positive and finite");
        }

        _scale = scale;
    }

    /// <summary>Quantizes and packs a batch of readings.</summary>
    /// <param name="readings">The readings, in order.</param>
    /// <returns>The packed encoding.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Encode(ReadOnlySpan<float> readings)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_codec_quantizer_encode(
            _scale, readings, (nuint)readings.Length, out IntPtr buffer));
        return Codec.TakeBytes(buffer);
    }

    /// <summary>Unpacks a batch, to within this quantizer's precision.</summary>
    /// <param name="bytes">The encoding produced by <see cref="Encode"/> at the same scale.</param>
    /// <returns>The readings, in order.</returns>
    /// <exception cref="PamojaException">The buffer is malformed.</exception>
    public float[] Decode(ReadOnlySpan<byte> bytes)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_codec_quantizer_decode(
            _scale, bytes, (nuint)bytes.Length, out IntPtr readings));
        try
        {
            int count = checked((int)NativeMethods.pamoja_readings_len(readings));
            float[] values = new float[count];
            if (count > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_readings_data(readings), values, 0, count);
            }

            return values;
        }
        finally
        {
            NativeMethods.pamoja_readings_free(readings);
        }
    }
}
