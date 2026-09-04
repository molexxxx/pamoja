using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Session;

/// <summary>Which side of a session a device is on.</summary>
/// <remarks>
/// The two devices must choose opposite roles: the role decides the order the
/// public keys are mixed in and which direction each side tags its messages with,
/// so a session where both sides claim the same role opens nothing.
/// </remarks>
public enum SessionRole
{
    /// <summary>The device that opens the session.</summary>
    Initiator = 0,

    /// <summary>The device that answers.</summary>
    Responder = 1,
}

/// <summary>A message that has been sealed, with the header beside it.</summary>
/// <param name="Counter">The counter naming this message within the session.</param>
/// <param name="Tag">The tag over the ciphertext and its associated data.</param>
/// <param name="Ciphertext">The encrypted message.</param>
public sealed record SealedMessage(ulong Counter, byte[] Tag, byte[] Ciphertext);

/// <summary>A key-agreement secret, and the public key to hand to a peer.</summary>
public sealed class AgreementKey : IDisposable
{
    /// <summary>The length in bytes of a seed and of a public key.</summary>
    public const int KeyLength = NativeMethods.SessionKeyLen;

    private readonly NativeHandle _handle;

    /// <summary>Creates a key-agreement secret from a provisioned 32-byte seed.</summary>
    /// <param name="seed">The device's secret, held on the device only.</param>
    /// <exception cref="PamojaException">The native key could not be created.</exception>
    public AgreementKey(ReadOnlySpan<byte> seed)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_agreement_key_from_seed(seed, (nuint)seed.Length),
            NativeMethods.pamoja_agreement_key_free,
            "agreement key");
    }

    /// <summary>Gets the public key to hand to a peer.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] PublicKey
    {
        get
        {
            byte[] key = new byte[KeyLength];
            PamojaCore.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_agreement_key_public(handle, key)));
            return key;
        }
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Runs a native call that needs this key handle.</summary>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    internal TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);
}

/// <summary>A confidential, tamper-evident, replay-protected channel with one peer.</summary>
/// <remarks>
/// Two devices that already know each other's public keys can agree on a session
/// key without ever sending it, and then exchange messages that cannot be read,
/// altered undetected, or replayed. That is the whole of what a small device
/// usually needs from transport security, at a fraction of what a TLS stack costs
/// it.
/// </remarks>
public sealed class Session : IDisposable
{
    /// <summary>The length in bytes of the tag on a sealed message.</summary>
    public const int TagLength = NativeMethods.SessionTagLen;

    private readonly NativeHandle _handle;

    /// <summary>Establishes a session with a peer.</summary>
    /// <remarks>
    /// Both devices call this with the same salt and opposite roles, and arrive at
    /// the same key without either sending it.
    /// </remarks>
    /// <param name="local">This device's key-agreement secret.</param>
    /// <param name="peerPublicKey">
    /// The peer's 32-byte public key, already authenticated by pinning or by a
    /// signature.
    /// </param>
    /// <param name="salt">
    /// A fresh per-session salt both sides share, exchanged in the clear. Reusing
    /// one with the same pair of keys reuses the session key, so it must change
    /// each session.
    /// </param>
    /// <param name="role">Whether this device opens the session or answers.</param>
    /// <exception cref="PamojaException">The native session could not be created.</exception>
    public Session(
        AgreementKey local,
        ReadOnlySpan<byte> peerPublicKey,
        ReadOnlySpan<byte> salt,
        SessionRole role)
    {
        ArgumentNullException.ThrowIfNull(local);

        byte[] peer = peerPublicKey.ToArray();
        byte[] saltBytes = salt.ToArray();
        _handle = NativeHandle.Create(
            local.Use(key => NativeMethods.pamoja_session_establish(
                key,
                peer,
                saltBytes,
                (nuint)saltBytes.Length,
                (PamojaSessionRole)role)),
            NativeMethods.pamoja_session_free,
            "session");
    }

    /// <summary>Seals a message for the peer.</summary>
    /// <param name="plaintext">The message to protect.</param>
    /// <param name="aad">
    /// Data authenticated but not encrypted, so it stays readable on the wire yet
    /// cannot be altered: a device identifier or a routing header belongs here.
    /// </param>
    /// <returns>The ciphertext, with the counter and tag to send beside it.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public SealedMessage Seal(ReadOnlySpan<byte> plaintext, ReadOnlySpan<byte> aad = default)
    {
        byte[] message = plaintext.ToArray();
        byte[] associated = aad.ToArray();
        PamojaSealed header = default;
        PamojaCore.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_session_seal(
            handle,
            message,
            (nuint)message.Length,
            associated,
            (nuint)associated.Length,
            out header)));
        return new SealedMessage(header.Counter, header.Tag.ToArray(), message);
    }

    /// <summary>Opens a message from the peer.</summary>
    /// <param name="message">
    /// The ciphertext with the counter and tag that arrived with it.
    /// </param>
    /// <param name="aad">The same associated data the sender authenticated.</param>
    /// <returns>The plaintext.</returns>
    /// <exception cref="PamojaException">
    /// The counter repeats or is older than the replay window still tracks, or the
    /// tag does not authenticate. Nothing readable is ever returned from a message
    /// that failed either check.
    /// </exception>
    public byte[] Open(SealedMessage message, ReadOnlySpan<byte> aad = default)
    {
        ArgumentNullException.ThrowIfNull(message);

        byte[] buffer = (byte[])message.Ciphertext.Clone();
        byte[] associated = aad.ToArray();
        PamojaSealed header = new()
        {
            Counter = message.Counter,
            Tag = PamojaTag.From(message.Tag, nameof(message)),
        };
        PamojaCore.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_session_open(
            handle,
            header,
            buffer,
            (nuint)buffer.Length,
            associated,
            (nuint)associated.Length)));
        return buffer;
    }

    /// <summary>Computes a keyed hash over a message.</summary>
    /// <remarks>
    /// This is the primitive a host uses to authenticate a pairing exchange or a
    /// single command, where a whole session would be more than the job needs.
    /// </remarks>
    /// <param name="key">The secret key.</param>
    /// <param name="message">The message to authenticate.</param>
    /// <returns>The 32-byte digest.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] HmacSha256(ReadOnlySpan<byte> key, ReadOnlySpan<byte> message)
    {
        byte[] digest = new byte[NativeMethods.SessionKeyLen];
        PamojaCore.ThrowIfError(NativeMethods.pamoja_session_hmac_sha256(
            key, (nuint)key.Length, message, (nuint)message.Length, digest));
        return digest;
    }

    /// <summary>Expands input keying material into bytes bound to a purpose.</summary>
    /// <param name="salt">The salt, which may be empty.</param>
    /// <param name="ikm">The input keying material.</param>
    /// <param name="info">Context binding the output to its purpose.</param>
    /// <param name="length">How many bytes to derive.</param>
    /// <returns>The derived bytes.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] HkdfSha256(
        ReadOnlySpan<byte> salt,
        ReadOnlySpan<byte> ikm,
        ReadOnlySpan<byte> info,
        int length)
    {
        byte[] derived = new byte[length];
        PamojaCore.ThrowIfError(NativeMethods.pamoja_session_hkdf_sha256(
            salt,
            (nuint)salt.Length,
            ikm,
            (nuint)ikm.Length,
            info,
            (nuint)info.Length,
            derived,
            (nuint)derived.Length));
        return derived;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
