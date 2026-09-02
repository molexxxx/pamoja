using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>One signed record, chained onto the one before it.</summary>
public sealed class AuditEntry : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a native entry handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    internal AuditEntry(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_audit_entry_free, "audit entry");
    }

    /// <summary>Gets the position of this record in its chain.</summary>
    public ulong Index => _handle.Use(NativeMethods.pamoja_audit_entry_index);

    /// <summary>Gets the hash of the record before this one, zeroes for the first.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Previous
    {
        get
        {
            byte[] digest = new byte[NativeMethods.AuditDigestLen];
            PamojaCore.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_audit_entry_previous(handle, digest)));
            return digest;
        }
    }

    /// <summary>Gets the hash of this record, which the next one chains onto.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Digest
    {
        get
        {
            byte[] digest = new byte[NativeMethods.AuditDigestLen];
            PamojaCore.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_audit_entry_digest(handle, digest)));
            return digest;
        }
    }

    /// <summary>Gets the record this entry carries.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Payload => Codec.TakeBytes(
        _handle.Use(NativeMethods.pamoja_audit_entry_payload));

    /// <summary>Gets the signature over this record.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] Signature
    {
        get
        {
            byte[] signature = new byte[NativeMethods.AuditSignatureLen];
            PamojaCore.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_audit_entry_signature(handle, signature)));
            return signature;
        }
    }

    /// <summary>Reads a record back from the bytes it was written as.</summary>
    /// <param name="bytes">The encoded entry.</param>
    /// <returns>The entry.</returns>
    /// <exception cref="PamojaException">The bytes are not a well-formed entry.</exception>
    public static AuditEntry FromBytes(ReadOnlySpan<byte> bytes) =>
        new(NativeMethods.pamoja_audit_entry_from_bytes(bytes, (nuint)bytes.Length));

    /// <summary>Encodes this record for storage or transmission.</summary>
    /// <returns>The encoded entry.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] ToBytes() =>
        Codec.TakeBytes(_handle.Use(NativeMethods.pamoja_audit_entry_to_bytes));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Runs a native call that needs this entry handle.</summary>
    /// <typeparam name="TResult">What the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    internal TResult Use<TResult>(Func<IntPtr, TResult> call) => _handle.Use(call);
}

/// <summary>A log that signs what it is given and chains it onto what came before.</summary>
/// <remarks>
/// A log that can be edited after the fact proves nothing. Each record here is
/// signed and carries the hash of the one before it, so altering a record,
/// dropping one, or reordering two breaks the chain at that point and at every
/// point after it. The index is part of what is signed, which is what makes a
/// record removed from the end detectable too.
/// </remarks>
public sealed class AuditLog : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a log that signs with a device identity, starting empty.</summary>
    /// <param name="identity">The identity whose signature each record carries.</param>
    /// <exception cref="PamojaException">The native log could not be created.</exception>
    public AuditLog(DeviceIdentity identity)
    {
        ArgumentNullException.ThrowIfNull(identity);
        _handle = NativeHandle.Create(
            identity.Use(NativeMethods.pamoja_audit_log_new),
            NativeMethods.pamoja_audit_log_free,
            "audit log");
    }

    /// <summary>Creates a log carrying on from the last record of an existing one.</summary>
    /// <remarks>
    /// This is what a device does after a restart: the chain continues at the next
    /// index and hashes onto the record it left off at, so a reboot leaves no gap
    /// for a record to be removed through.
    /// </remarks>
    /// <param name="identity">The identity whose signature each record carries.</param>
    /// <param name="last">The final record of the existing log.</param>
    /// <exception cref="PamojaException">The native log could not be created.</exception>
    private AuditLog(DeviceIdentity identity, AuditEntry last)
    {
        _handle = NativeHandle.Create(
            identity.Use(signer => last.Use(entry =>
                NativeMethods.pamoja_audit_log_resume(signer, entry))),
            NativeMethods.pamoja_audit_log_free,
            "audit log");
    }

    /// <summary>Creates a log carrying on from the last record of an existing one.</summary>
    /// <param name="identity">The identity whose signature each record carries.</param>
    /// <param name="last">The final record of the existing log.</param>
    /// <returns>The log, positioned after <paramref name="last"/>.</returns>
    /// <exception cref="PamojaException">The native log could not be created.</exception>
    public static AuditLog Resume(DeviceIdentity identity, AuditEntry last)
    {
        ArgumentNullException.ThrowIfNull(identity);
        ArgumentNullException.ThrowIfNull(last);
        return new AuditLog(identity, last);
    }

    /// <summary>Appends a payload, signing it and chaining it onto the last record.</summary>
    /// <param name="payload">The record to store.</param>
    /// <returns>The new entry.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public AuditEntry Append(ReadOnlySpan<byte> payload)
    {
        byte[] copy = payload.ToArray();
        return new AuditEntry(_handle.Use(handle =>
            NativeMethods.pamoja_audit_log_append(handle, copy, (nuint)copy.Length)));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Checks a chain one record at a time, in the order they were written.</summary>
public sealed class AuditVerifier : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a verifier for a chain signed by one public key.</summary>
    /// <param name="publicKey">The 32-byte key the records were signed with.</param>
    /// <exception cref="PamojaException">The key is not a valid public key.</exception>
    public AuditVerifier(ReadOnlySpan<byte> publicKey)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_audit_verifier_new(publicKey),
            NativeMethods.pamoja_audit_verifier_free,
            "audit verifier");
    }

    /// <summary>Checks the next record, in the order the records were written.</summary>
    /// <remarks>
    /// Feeding records out of order, skipping one, or repeating one is refused
    /// just as an altered payload is.
    /// </remarks>
    /// <param name="entry">The next record to check.</param>
    /// <returns><c>true</c> when the record belongs where it was offered.</returns>
    public bool Check(AuditEntry entry)
    {
        ArgumentNullException.ThrowIfNull(entry);
        return _handle.Use(verifier => entry.Use(handle =>
            NativeMethods.pamoja_audit_verifier_check(verifier, handle))) == PamojaStatus.Ok;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Checking a chain of audit records that has already arrived.</summary>
public static class Audit
{
    /// <summary>Checks a whole chain that has already arrived.</summary>
    /// <param name="publicKey">The 32-byte key the records were signed with.</param>
    /// <param name="entries">The records, in the order they were written.</param>
    /// <returns>
    /// <c>true</c> when every record follows the one before it and carries a
    /// signature that holds.
    /// </returns>
    public static bool VerifyChain(ReadOnlySpan<byte> publicKey, IReadOnlyList<AuditEntry> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);

        IntPtr[] handles = new IntPtr[entries.Count];
        for (int at = 0; at < entries.Count; at++)
        {
            handles[at] = entries[at].Use(handle => handle);
        }

        bool holds = NativeMethods.pamoja_audit_verify_chain(
            publicKey, handles, (nuint)handles.Length) == PamojaStatus.Ok;
        GC.KeepAlive(entries);
        return holds;
    }
}
