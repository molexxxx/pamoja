using System.Text;

using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Security;

/// <summary>A device's private signing identity.</summary>
/// <remarks>
/// A reading that drives a health or billing decision has to be provably from the
/// device that claims to have sent it, and provably unaltered on the way. Sign it
/// here, and any holder of <see cref="PublicKey"/> can check it with
/// <see cref="Verify(ReadOnlySpan{byte}, ReadOnlySpan{byte}, ReadOnlySpan{byte})"/>.
/// </remarks>
/// <example>
/// <code>
/// using var device = new DeviceIdentity(seed);
/// byte[] signature = device.Sign("21.5");
/// DeviceIdentity.Verify(device.PublicKey, "21.5", signature); // true
/// </code>
/// </example>
public sealed class DeviceIdentity : IDisposable
{
    /// <summary>The length in bytes of an identity seed and of a public key.</summary>
    public const int KeyLength = 32;

    /// <summary>The length in bytes of a signature.</summary>
    public const int SignatureLength = 64;

    /// <summary>The length in characters of a hex fingerprint.</summary>
    private const int FingerprintLength = 16;

    private readonly NativeHandle _handle;

    /// <summary>Creates an identity from a provisioned 32-byte secret seed.</summary>
    /// <param name="seed">The device's secret, held on the device only.</param>
    /// <exception cref="ArgumentException"><paramref name="seed"/> is not 32 bytes.</exception>
    /// <exception cref="PamojaException">The native identity could not be created.</exception>
    public DeviceIdentity(ReadOnlySpan<byte> seed)
    {
        if (seed.Length != KeyLength)
        {
            throw new ArgumentException(
                $"seed must be exactly {KeyLength} bytes", nameof(seed));
        }

        _handle = NativeHandle.Create(
            NativeMethods.pamoja_device_identity_new(seed, (nuint)seed.Length),
            NativeMethods.pamoja_device_identity_free,
            "device identity");
    }

    /// <summary>Runs a native call that needs this identity handle.</summary>
    /// <remarks>
    /// The audit and update capabilities sign with an identity this class holds,
    /// and the native calls take its handle. This is how the two meet without a
    /// caller ever seeing it.
    /// </remarks>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);

    /// <summary>Gets the public key matching this identity, which is safe to share.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] PublicKey
    {
        get
        {
            byte[] key = new byte[KeyLength];
            PamojaCore.ThrowIfError(
                _handle.Use(handle =>
                    NativeMethods.pamoja_device_identity_public_key(handle, key)));
            return key;
        }
    }

    /// <summary>Gets the short hex fingerprint of this identity, for logs and displays.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public string Fingerprint => FingerprintOf(PublicKey);

    /// <summary>Verifies that a signature covers a payload and was made by a key.</summary>
    /// <param name="publicKey">The 32-byte public key of the claimed signer.</param>
    /// <param name="payload">The bytes the signature should cover.</param>
    /// <param name="signature">The 64-byte detached signature.</param>
    /// <returns>
    /// <c>true</c> if the signature is authentic, and <c>false</c> if the payload
    /// was altered or was signed by a different device.
    /// </returns>
    /// <exception cref="PamojaException">An argument was not the expected length.</exception>
    public static bool Verify(
        ReadOnlySpan<byte> publicKey,
        ReadOnlySpan<byte> payload,
        ReadOnlySpan<byte> signature)
    {
        PamojaStatus status = NativeMethods.pamoja_public_identity_verify(
            publicKey, payload, (nuint)payload.Length, signature);

        if (status == PamojaStatus.Auth)
        {
            return false;
        }

        PamojaCore.ThrowIfError(status);
        return true;
    }

    /// <summary>Verifies a signature over text, encoded as UTF-8.</summary>
    /// <param name="publicKey">The 32-byte public key of the claimed signer.</param>
    /// <param name="payload">The text the signature should cover.</param>
    /// <param name="signature">The 64-byte detached signature.</param>
    /// <returns><c>true</c> if the signature is authentic.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="payload"/> is null.</exception>
    /// <exception cref="PamojaException">An argument was not the expected length.</exception>
    public static bool Verify(
        ReadOnlySpan<byte> publicKey,
        string payload,
        ReadOnlySpan<byte> signature)
    {
        ArgumentNullException.ThrowIfNull(payload);
        return Verify(publicKey, Encoding.UTF8.GetBytes(payload), signature);
    }

    /// <summary>Returns the short hex fingerprint of a public key.</summary>
    /// <param name="publicKey">The 32-byte public key to label.</param>
    /// <returns>A 16-character lowercase hex label.</returns>
    /// <exception cref="PamojaException">The key is not a valid public key.</exception>
    public static string FingerprintOf(ReadOnlySpan<byte> publicKey)
    {
        byte[] hex = new byte[FingerprintLength];
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_public_identity_fingerprint(publicKey, hex));
        return Encoding.ASCII.GetString(hex);
    }

    /// <summary>Signs a payload.</summary>
    /// <param name="payload">The bytes to cover.</param>
    /// <returns>The 64-byte detached signature.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Sign(ReadOnlySpan<byte> payload)
    {
        byte[] signature = new byte[SignatureLength];
        int length = payload.Length;

        // The payload cannot be captured by a lambda because it is a span, so the
        // ref-counted call is written out here rather than going through Use.
        bool added = false;
        try
        {
            _handle.DangerousAddRef(ref added);
            PamojaCore.ThrowIfError(NativeMethods.pamoja_device_identity_sign(
                _handle.DangerousGetHandle(), payload, (nuint)length, signature));
        }
        finally
        {
            if (added)
            {
                _handle.DangerousRelease();
            }
        }

        return signature;
    }

    /// <summary>Signs text, encoded as UTF-8.</summary>
    /// <param name="payload">The text to cover.</param>
    /// <returns>The 64-byte detached signature.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="payload"/> is null.</exception>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Sign(string payload)
    {
        ArgumentNullException.ThrowIfNull(payload);
        return Sign(Encoding.UTF8.GetBytes(payload));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
