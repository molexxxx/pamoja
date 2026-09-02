using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The P/Invoke declarations for the operational capabilities of the pamoja C ABI
/// - audit logs, secured sessions, signed updates, power scheduling, and
/// telemetry - mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The length in bytes of an audit entry hash.</summary>
    public const int AuditDigestLen = 32;

    /// <summary>The length in bytes of an audit entry signature.</summary>
    public const int AuditSignatureLen = 64;

    /// <summary>The length in bytes of an agreement seed, key, or digest.</summary>
    public const int SessionKeyLen = 32;

    /// <summary>The length in bytes of the tag on a sealed message.</summary>
    public const int SessionTagLen = 16;

    /// <summary>The manifest structure version this build writes.</summary>
    public const byte UpdateStructureVersion = 1;

    /// <summary>The payload format meaning the payload is the image itself.</summary>
    public const byte UpdateFormatRaw = 1;

    /// <summary>Creates a log that signs with a device identity, starting empty.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_log_new(IntPtr identity);

    /// <summary>Creates a log that carries on from an existing final entry.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_log_resume(IntPtr identity, IntPtr last);

    /// <summary>Appends a payload, signing and chaining it.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_log_append(
        IntPtr log,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Releases a log handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_audit_log_free(IntPtr log);

    /// <summary>Reads an entry back from the bytes it was written as.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_entry_from_bytes(
        ReadOnlySpan<byte> bytes,
        nuint len);

    /// <summary>Encodes an entry for storage or transmission.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_entry_to_bytes(IntPtr entry);

    /// <summary>Returns the position of an entry in its chain.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_audit_entry_index(IntPtr entry);

    /// <summary>Copies out the hash of the entry before this one.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_audit_entry_previous(
        IntPtr entry,
        Span<byte> outPrevious);

    /// <summary>Copies out the hash of this entry.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_audit_entry_digest(
        IntPtr entry,
        Span<byte> outDigest);

    /// <summary>Copies out the signature over an entry.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_audit_entry_signature(
        IntPtr entry,
        Span<byte> outSignature);

    /// <summary>Copies out the record an entry carries.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_entry_payload(IntPtr entry);

    /// <summary>Releases an entry handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_audit_entry_free(IntPtr entry);

    /// <summary>Creates a verifier for a chain signed by one public key.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_audit_verifier_new(ReadOnlySpan<byte> publicKey);

    /// <summary>Checks the next entry of a chain.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_audit_verifier_check(
        IntPtr verifier,
        IntPtr entry);

    /// <summary>Releases a verifier handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_audit_verifier_free(IntPtr verifier);

    /// <summary>Checks a whole chain that has already arrived.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_audit_verify_chain(
        ReadOnlySpan<byte> publicKey,
        ReadOnlySpan<IntPtr> entries,
        nuint count);

    /// <summary>Creates a key-agreement secret from a 32-byte seed.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_agreement_key_from_seed(
        ReadOnlySpan<byte> seed,
        nuint seedLen);

    /// <summary>Copies out the public key to hand to a peer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_agreement_key_public(
        IntPtr key,
        Span<byte> outPublicKey);

    /// <summary>Releases an agreement key handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_agreement_key_free(IntPtr key);

    /// <summary>Establishes a session with a peer.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_session_establish(
        IntPtr local,
        ReadOnlySpan<byte> peerPublicKey,
        ReadOnlySpan<byte> salt,
        nuint saltLen,
        PamojaSessionRole role);

    /// <summary>Seals a message for the peer, encrypting it in place.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_session_seal(
        IntPtr session,
        Span<byte> buf,
        nuint len,
        ReadOnlySpan<byte> aad,
        nuint aadLen,
        out PamojaSealed outSealed);

    /// <summary>Opens a message from the peer, decrypting it in place.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_session_open(
        IntPtr session,
        PamojaSealed sealedHeader,
        Span<byte> buf,
        nuint len,
        ReadOnlySpan<byte> aad,
        nuint aadLen);

    /// <summary>Releases a session handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_session_free(IntPtr session);

    /// <summary>Computes a keyed hash over a message.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_session_hmac_sha256(
        ReadOnlySpan<byte> key,
        nuint keyLen,
        ReadOnlySpan<byte> message,
        nuint messageLen,
        Span<byte> outDigest);

    /// <summary>Expands input keying material into as many bytes as asked for.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_session_hkdf_sha256(
        ReadOnlySpan<byte> salt,
        nuint saltLen,
        ReadOnlySpan<byte> ikm,
        nuint ikmLen,
        ReadOnlySpan<byte> info,
        nuint infoLen,
        Span<byte> outBytes,
        nuint outLen);

    /// <summary>Encodes the body of a manifest.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_manifest_encode(PamojaManifest manifest);

    /// <summary>Reads a manifest body back from its bytes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_manifest_decode(
        ReadOnlySpan<byte> bytes,
        nuint len,
        out PamojaManifest outManifest);

    /// <summary>Signs a manifest into the envelope offered to a device.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_manifest_sign(PamojaManifest manifest, IntPtr author);

    /// <summary>Verifies an envelope and reads the manifest inside it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_envelope_verify(
        ReadOnlySpan<byte> bytes,
        nuint len,
        ReadOnlySpan<byte> publicKey,
        out PamojaManifest outManifest);

    /// <summary>Copies out the signed body of an envelope.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_envelope_body(ReadOnlySpan<byte> bytes, nuint len);

    /// <summary>Signs a delegation naming a release key.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_delegation_sign(
        PamojaDelegation delegation,
        IntPtr anchor);

    /// <summary>Opens a signed delegation against its anchor.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_delegation_open(
        ReadOnlySpan<byte> bytes,
        nuint len,
        ReadOnlySpan<byte> anchorPublicKey,
        out PamojaDelegation outDelegation);

    /// <summary>Creates a verifier that hashes an image against a manifest.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_image_verifier_new(PamojaManifest manifest);

    /// <summary>Takes the next piece of the image.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_image_verifier_update(
        IntPtr verifier,
        ReadOnlySpan<byte> chunk,
        nuint len);

    /// <summary>Settles an image against its manifest, consuming the verifier.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_image_verifier_finish(
        IntPtr verifier,
        out uint outSize,
        Span<byte> outDigest);

    /// <summary>Releases a verifier handle that will not be settled.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_image_verifier_free(IntPtr verifier);

    /// <summary>Creates an updater over a device's slots.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_updater_new(
        PamojaDevice device,
        byte slotCount,
        uint slotCapacity);

    /// <summary>Adopts a delegation naming a release key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_adopt(
        IntPtr updater,
        ReadOnlySpan<byte> bytes,
        nuint len,
        [MarshalAs(UnmanagedType.U1)] bool hasNow,
        ulong now,
        out PamojaDelegation outDelegation);

    /// <summary>Reads the delegation an updater currently honours.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_updater_delegation(
        IntPtr updater,
        out PamojaDelegation outDelegation);

    /// <summary>Reads the highest sequence number the device already holds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_installed_sequence(
        IntPtr updater,
        out ulong outSequence);

    /// <summary>Reads what a device believes about one slot.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_slot_record(
        IntPtr updater,
        byte slot,
        out PamojaSlotRecord outRecord);

    /// <summary>Returns how many slots a device has.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_updater_slot_count(IntPtr updater);

    /// <summary>Records that a slot holds a confirmed image at a sequence number.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_provision(
        IntPtr updater,
        byte slot,
        ulong sequence);

    /// <summary>Checks a manifest and stages an image held whole.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_stage(
        IntPtr updater,
        ReadOnlySpan<byte> envelope,
        nuint envelopeLen,
        ReadOnlySpan<byte> image,
        nuint imageLen,
        [MarshalAs(UnmanagedType.U1)] bool hasNow,
        ulong now,
        out byte outSlot);

    /// <summary>Checks a manifest and opens its slot for a transfer in pieces.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_begin(
        IntPtr updater,
        ReadOnlySpan<byte> envelope,
        nuint envelopeLen,
        [MarshalAs(UnmanagedType.U1)] bool hasNow,
        ulong now,
        out byte outSlot);

    /// <summary>Takes the next piece of an opened image.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_write(
        IntPtr updater,
        ReadOnlySpan<byte> chunk,
        nuint len);

    /// <summary>Reports how much of an opened image has arrived.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_progress(
        IntPtr updater,
        out uint outWritten,
        out uint outTotal);

    /// <summary>Finishes an opened image and marks the slot bootable if it matched.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_finish(IntPtr updater, out byte outSlot);

    /// <summary>Decides what to run, recording the decision before returning it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_on_boot(
        IntPtr updater,
        out PamojaBoot outBoot);

    /// <summary>Confirms the pending image.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_confirm(IntPtr updater, out byte outSlot);

    /// <summary>Fails the pending image and goes back to the confirmed one.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_updater_revert(IntPtr updater, out byte outSlot);

    /// <summary>Releases an updater handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_updater_free(IntPtr updater);

    /// <summary>Creates a duty cycle from the time awake and the time asleep.</summary>
    [LibraryImport(Library)]
    public static partial PamojaDutyCycle pamoja_duty_cycle_new(ulong activeUs, ulong sleepUs);

    /// <summary>Creates a duty cycle that spends a fraction of a period awake.</summary>
    [LibraryImport(Library)]
    public static partial PamojaDutyCycle pamoja_duty_cycle_from_fraction(
        ulong periodUs,
        float fraction);

    /// <summary>Returns the whole period of a duty cycle.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_duty_cycle_period_us(PamojaDutyCycle duty);

    /// <summary>Returns the share of a period a duty cycle spends awake.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_duty_cycle_fraction(PamojaDutyCycle duty);

    /// <summary>Creates a power plan with the default thresholds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPowerPlan pamoja_power_plan_new(
        ulong activeUs,
        ulong saverUs,
        ulong criticalUs);

    /// <summary>Returns a plan with the state-of-charge thresholds moved.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPowerPlan pamoja_power_plan_with_thresholds(
        PamojaPowerPlan plan,
        float saverBelow,
        float criticalBelow);

    /// <summary>Returns the mode a plan calls for at a state of charge.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPowerMode pamoja_power_plan_mode(PamojaPowerPlan plan, float soc);

    /// <summary>Returns the mode, eased one step toward full duty while charging.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPowerMode pamoja_power_plan_mode_while_charging(
        PamojaPowerPlan plan,
        float soc,
        byte charging);

    /// <summary>Returns the work interval a plan uses in a mode.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_power_plan_interval_for_us(
        PamojaPowerPlan plan,
        PamojaPowerMode mode);

    /// <summary>Returns the work interval a plan calls for at a state of charge.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_power_plan_interval_us(PamojaPowerPlan plan, float soc);

    /// <summary>Returns the level a link cost calls for.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTelemetryLevel pamoja_link_cost_threshold(PamojaLinkCost cost);

    /// <summary>Creates a reporter that ships events at or above a level.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_reporter_new(PamojaTelemetryLevel threshold);

    /// <summary>Returns the level a reporter is currently shipping from.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTelemetryLevel pamoja_reporter_threshold(IntPtr reporter);

    /// <summary>Moves the level a reporter ships from.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_reporter_set_threshold(
        IntPtr reporter,
        PamojaTelemetryLevel threshold);

    /// <summary>Moves the threshold to match what the link now costs.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_reporter_adapt_to(IntPtr reporter, PamojaLinkCost cost);

    /// <summary>Records an event and reports whether it is worth shipping.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_reporter_record(
        IntPtr reporter,
        PamojaTelemetryLevel level);

    /// <summary>Returns how many events a reporter has seen at a level.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_reporter_count(
        IntPtr reporter,
        PamojaTelemetryLevel level);

    /// <summary>Returns how many events a reporter has seen across every level.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_reporter_total(IntPtr reporter);

    /// <summary>Returns how many events passed the threshold and were shipped.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_reporter_emitted(IntPtr reporter);

    /// <summary>Returns how many events the threshold dropped.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_reporter_dropped(IntPtr reporter);

    /// <summary>Takes a snapshot of a reporter's counters.</summary>
    [LibraryImport(Library)]
    public static partial PamojaTelemetrySnapshot pamoja_reporter_snapshot(IntPtr reporter);

    /// <summary>Releases a reporter handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_reporter_free(IntPtr reporter);
}
