using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Wire formats and metered-link packing.</summary>
/// <remarks>
/// The core's codec trait is generic over the value it carries and cannot cross
/// the C ABI, so what is offered here is the concrete work a caller with an
/// untyped document needs: moving it between JSON and CBOR, and packing a batch
/// of samples small enough for a link that charges per byte.
/// </remarks>
public static class Codec
{
    /// <summary>Converts a JSON document into its CBOR encoding.</summary>
    /// <param name="json">The UTF-8 JSON document to convert.</param>
    /// <returns>The CBOR encoding, typically a good deal smaller.</returns>
    /// <exception cref="PamojaException">The document is not valid JSON.</exception>
    public static byte[] JsonToCbor(ReadOnlySpan<byte> json)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_codec_json_to_cbor(json, (nuint)json.Length, out IntPtr buffer));
        return TakeBytes(buffer);
    }

    /// <summary>Converts a CBOR document into its JSON encoding.</summary>
    /// <param name="cbor">The CBOR document to convert.</param>
    /// <returns>The UTF-8 JSON encoding.</returns>
    /// <exception cref="PamojaException">
    /// The document is malformed, or holds a construct with no JSON equivalent
    /// such as a non-string map key.
    /// </exception>
    public static byte[] CborToJson(ReadOnlySpan<byte> cbor)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_codec_cbor_to_json(cbor, (nuint)cbor.Length, out IntPtr buffer));
        return TakeBytes(buffer);
    }

    /// <summary>Delta-encodes a series of integer samples into a compact buffer.</summary>
    /// <param name="samples">The samples, in order.</param>
    /// <returns>The packed encoding.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] PackSamples(ReadOnlySpan<long> samples)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_codec_encode_deltas(
            samples, (nuint)samples.Length, out IntPtr buffer));
        return TakeBytes(buffer);
    }

    /// <summary>Unpacks a buffer produced by <see cref="PackSamples"/>.</summary>
    /// <param name="bytes">The packed encoding.</param>
    /// <returns>The samples, in order.</returns>
    /// <exception cref="PamojaException">The buffer is malformed.</exception>
    public static long[] UnpackSamples(ReadOnlySpan<byte> bytes)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_codec_decode_deltas(
            bytes, (nuint)bytes.Length, out IntPtr samples));
        try
        {
            int count = checked((int)NativeMethods.pamoja_samples_len(samples));
            long[] values = new long[count];
            if (count > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_samples_data(samples), values, 0, count);
            }

            return values;
        }
        finally
        {
            NativeMethods.pamoja_samples_free(samples);
        }
    }

    /// <summary>Copies a native byte buffer out and releases it.</summary>
    /// <param name="buffer">The buffer handle a native call produced.</param>
    /// <returns>The buffer's contents.</returns>
    internal static byte[] TakeBytes(IntPtr buffer)
    {
        try
        {
            int length = checked((int)NativeMethods.pamoja_buffer_len(buffer));
            byte[] bytes = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_buffer_data(buffer), bytes, 0, length);
            }

            return bytes;
        }
        finally
        {
            NativeMethods.pamoja_buffer_free(buffer);
        }
    }
}
