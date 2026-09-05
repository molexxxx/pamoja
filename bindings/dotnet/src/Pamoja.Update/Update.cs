using Pamoja.Codec;
using Pamoja.Security;
using Pamoja.Native.Interop;

namespace Pamoja.Update;

/// <summary>What a device believes about one slot.</summary>
public enum SlotState
{
    /// <summary>Nothing has been written here.</summary>
    Empty = 0,

    /// <summary>An image is arriving, and <c>Written</c> says how much of it has.</summary>
    Receiving = 1,

    /// <summary>A complete image that matched its manifest, not yet tried.</summary>
    Staged = 2,

    /// <summary>Being tried for the first time; it reverts unless it confirms.</summary>
    Pending = 3,

    /// <summary>Tried and confirmed working.</summary>
    Confirmed = 4,

    /// <summary>Tried and did not confirm, so it will not be tried again.</summary>
    Failed = 5,
}

/// <summary>What a bootloader should do with what it found.</summary>
public enum BootAction
{
    /// <summary>Nothing new to try; run the confirmed image.</summary>
    Confirmed = 0,

    /// <summary>A staged image is being tried for the first time.</summary>
    Trying = 1,

    /// <summary>A pending image never confirmed, so it was failed.</summary>
    Reverted = 2,
}

/// <summary>What a release says about itself, and what a device checks it against.</summary>
/// <param name="Sequence">
/// Rises with every release, which is what stops an older image being replayed at
/// a device.
/// </param>
/// <param name="VendorId">Who built the image, as 16 bytes.</param>
/// <param name="ClassId">Which kind of device it is for, as 16 bytes.</param>
/// <param name="Storage">Which slot the payload belongs in.</param>
/// <param name="Digest">The SHA-256 of the payload, as 32 bytes.</param>
/// <param name="Size">The payload length in bytes.</param>
/// <param name="Expires">
/// When the release stops being offered, in seconds since the Unix epoch, or 0 to
/// never expire.
/// </param>
/// <param name="Format">How the payload is encoded.</param>
/// <param name="StructureVersion">Which iteration of the manifest format this is.</param>
public sealed record Manifest(
    ulong Sequence,
    byte[] VendorId,
    byte[] ClassId,
    byte Storage,
    byte[] Digest,
    uint Size,
    ulong Expires = 0,
    byte Format = NativeMethods.UpdateFormatRaw,
    byte StructureVersion = NativeMethods.UpdateStructureVersion);

/// <summary>A statement, signed by the anchor, that a second key may sign releases.</summary>
/// <param name="Epoch">
/// Rises with every rotation, so a retired key cannot be reinstated by replaying
/// the statement that once authorised it.
/// </param>
/// <param name="ReleaseKey">The key that may sign manifests while this stands.</param>
/// <param name="Expires">
/// When the delegation stops being honoured, in seconds since the Unix epoch, or
/// 0 to never expire.
/// </param>
public sealed record Delegation(ulong Epoch, byte[] ReleaseKey, ulong Expires = 0);

/// <summary>The record a device keeps about one slot, durable across a reboot.</summary>
/// <param name="State">The state of the slot.</param>
/// <param name="Sequence">The sequence number of the image in the slot.</param>
/// <param name="Size">The length of the image in bytes.</param>
/// <param name="Digest">The digest of the image.</param>
/// <param name="Written">
/// How many bytes have been stored, which is where a resumed transfer picks up.
/// </param>
public sealed record SlotRecord(
    SlotState State,
    ulong Sequence,
    uint Size,
    byte[] Digest,
    uint Written);

/// <summary>The decision a device made at boot, recorded before it was returned.</summary>
/// <param name="Action">What the bootloader should do.</param>
/// <param name="Slot">
/// The image the decision is about, which for <see cref="BootAction.Reverted"/>
/// is the one that failed.
/// </param>
/// <param name="Fallback">
/// The slot to run. It is the same as <paramref name="Slot"/> for anything but
/// <see cref="BootAction.Reverted"/>.
/// </param>
public readonly record struct BootDecision(BootAction Action, byte Slot, byte Fallback);

/// <summary>How much of an image has arrived.</summary>
/// <param name="Written">The bytes stored so far.</param>
/// <param name="Total">The total the manifest declares.</param>
public readonly record struct Progress(uint Written, uint Total);

/// <summary>Hashes an image as it arrives and settles it against its manifest.</summary>
public sealed class ImageVerifier : IDisposable
{
    private IntPtr _handle;

    /// <summary>Creates a verifier for the image a manifest describes.</summary>
    /// <param name="manifest">The manifest describing the image.</param>
    /// <exception cref="PamojaException">The native verifier could not be created.</exception>
    public ImageVerifier(Manifest manifest)
    {
        _handle = NativeMethods.pamoja_image_verifier_new(Pamoja.Update.Update.ToNative(manifest));
        if (_handle == IntPtr.Zero)
        {
            throw new PamojaException(
                Status.LastError() ?? "failed to create the image verifier");
        }
    }

    /// <summary>Takes the next piece of the image, in order.</summary>
    /// <param name="chunk">The next bytes of the image.</param>
    /// <exception cref="PamojaException">
    /// More bytes have arrived than the manifest declared, or the verifier has
    /// already been settled.
    /// </exception>
    public void Update(ReadOnlySpan<byte> chunk)
    {
        Status.ThrowIfError(NativeMethods.pamoja_image_verifier_update(
            Live(), chunk, (nuint)chunk.Length));
    }

    /// <summary>Settles the image against its manifest, spending this verifier.</summary>
    /// <returns>The digest of the image that was hashed.</returns>
    /// <exception cref="PamojaException">
    /// The image is not the one the manifest described, or the verifier has
    /// already been settled.
    /// </exception>
    public byte[] Finish()
    {
        IntPtr handle = Live();
        _handle = IntPtr.Zero;
        byte[] digest = new byte[PamojaDigest.Length];
        Status.ThrowIfError(NativeMethods.pamoja_image_verifier_finish(
            handle, out _, digest));
        return digest;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.pamoja_image_verifier_free(_handle);
            _handle = IntPtr.Zero;
        }
    }

    /// <summary>Returns the handle, refusing one that has already been spent.</summary>
    private IntPtr Live() => _handle != IntPtr.Zero
        ? _handle
        : throw new PamojaException("this verifier has already been settled");
}

/// <summary>A device's slots, and the rules applied to what is offered for them.</summary>
/// <remarks>
/// A device that cannot be fixed in the field is a device that has to be visited,
/// and some of them are a day's travel away. A release carries a manifest naming
/// who it is for and what it hashes to, a device refuses anything not signed by
/// the key it trusts, and an image that fails to confirm itself is rolled back to
/// the one that worked.
/// </remarks>
public sealed class Updater : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an updater for a device with a number of slots.</summary>
    /// <param name="vendorId">Who built this firmware, as 16 bytes.</param>
    /// <param name="classId">What kind of device this is, as 16 bytes.</param>
    /// <param name="anchorPublicKey">
    /// The 32-byte key this device anchors its trust in, which is the root of
    /// every decision about who may update it.
    /// </param>
    /// <param name="slotCount">How many slots the device has.</param>
    /// <param name="slotCapacity">How many bytes each slot holds.</param>
    /// <exception cref="PamojaException">The native updater could not be created.</exception>
    public Updater(
        ReadOnlySpan<byte> vendorId,
        ReadOnlySpan<byte> classId,
        ReadOnlySpan<byte> anchorPublicKey,
        byte slotCount,
        uint slotCapacity)
    {
        PamojaDevice device = new()
        {
            VendorId = PamojaId.From(vendorId, nameof(vendorId)),
            ClassId = PamojaId.From(classId, nameof(classId)),
            Anchor = PamojaDigest.From(anchorPublicKey, nameof(anchorPublicKey)),
        };
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_updater_new(device, slotCount, slotCapacity),
            NativeMethods.pamoja_updater_free,
            "updater");
    }

    /// <summary>Gets how many slots this device has.</summary>
    public byte SlotCount => _handle.Use(NativeMethods.pamoja_updater_slot_count);

    /// <summary>Gets the highest sequence number the device already holds.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public ulong InstalledSequence
    {
        get
        {
            ulong sequence = 0;
            Status.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_updater_installed_sequence(handle, out sequence)));
            return sequence;
        }
    }

    /// <summary>
    /// Gets the delegation this updater honours, or <c>null</c> when releases must
    /// be signed by the anchor itself.
    /// </summary>
    public Delegation? CurrentDelegation
    {
        get
        {
            PamojaDelegation delegated = default;
            bool held = _handle.Use(handle =>
                NativeMethods.pamoja_updater_delegation(handle, out delegated));
            return held ? Update.FromNative(delegated) : null;
        }
    }

    /// <summary>Reads what the device believes about one slot.</summary>
    /// <param name="slot">The slot to read.</param>
    /// <returns>The record.</returns>
    /// <exception cref="PamojaException">The device has no such slot.</exception>
    public SlotRecord Record(byte slot)
    {
        PamojaSlotRecord record = default;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_slot_record(handle, slot, out record)));
        return new SlotRecord(
            (SlotState)record.State,
            record.Sequence,
            record.Size,
            record.Digest.ToArray(),
            record.Written);
    }

    /// <summary>Records that a slot holds a confirmed image at a sequence number.</summary>
    /// <remarks>
    /// This is how a device that shipped with firmware says what it is running, so
    /// the rollback rule has something to compare against.
    /// </remarks>
    /// <param name="slot">The slot holding the running image.</param>
    /// <param name="sequence">The sequence number of that image.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public void Provision(byte slot, ulong sequence) =>
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_provision(handle, slot, sequence)));

    /// <summary>Adopts a delegation, accepting releases signed by the key it names.</summary>
    /// <param name="envelope">The signed delegation envelope.</param>
    /// <param name="now">
    /// Seconds since the Unix epoch, or <c>null</c> on a device with no clock.
    /// </param>
    /// <returns>The adopted delegation.</returns>
    /// <exception cref="PamojaException">
    /// The delegation is not from the anchor, is not newer than the one held, or
    /// has expired.
    /// </exception>
    public Delegation Adopt(ReadOnlySpan<byte> envelope, ulong? now = null)
    {
        byte[] bytes = envelope.ToArray();
        PamojaDelegation adopted = default;
        Status.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_updater_adopt(
            handle,
            bytes,
            (nuint)bytes.Length,
            now.HasValue,
            now ?? 0,
            out adopted)));
        return Update.FromNative(adopted);
    }

    /// <summary>Checks a manifest and stages an image that is already held whole.</summary>
    /// <param name="envelope">The signed manifest offered to this device.</param>
    /// <param name="image">The whole image.</param>
    /// <param name="now">
    /// Seconds since the Unix epoch, or <c>null</c> on a device with no clock.
    /// </param>
    /// <returns>The slot the image was staged into.</returns>
    /// <exception cref="PamojaException">A rule refused the update.</exception>
    public byte Stage(ReadOnlySpan<byte> envelope, ReadOnlySpan<byte> image, ulong? now = null)
    {
        byte[] envelopeBytes = envelope.ToArray();
        byte[] imageBytes = image.ToArray();
        byte slot = 0;
        Status.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_updater_stage(
            handle,
            envelopeBytes,
            (nuint)envelopeBytes.Length,
            imageBytes,
            (nuint)imageBytes.Length,
            now.HasValue,
            now ?? 0,
            out slot)));
        return slot;
    }

    /// <summary>Checks a manifest and opens its slot for a transfer in pieces.</summary>
    /// <remarks>
    /// Every check that can be made without the image runs here, so a release that
    /// is not for this device, would roll it back, or does not fit is refused
    /// before a byte of it is accepted. The envelope is remembered until
    /// <see cref="Finish"/>, and each call after this one reopens the transfer
    /// from what the slot records, which is the same path a device takes after a
    /// reset.
    /// </remarks>
    /// <param name="envelope">The signed manifest offered to this device.</param>
    /// <param name="now">
    /// Seconds since the Unix epoch, or <c>null</c> on a device with no clock.
    /// </param>
    /// <returns>The slot the image will be written into.</returns>
    /// <exception cref="PamojaException">A rule refused the update.</exception>
    public byte Begin(ReadOnlySpan<byte> envelope, ulong? now = null)
    {
        byte[] bytes = envelope.ToArray();
        byte slot = 0;
        Status.ThrowIfError(_handle.Use(handle => NativeMethods.pamoja_updater_begin(
            handle,
            bytes,
            (nuint)bytes.Length,
            now.HasValue,
            now ?? 0,
            out slot)));
        return slot;
    }

    /// <summary>Takes the next piece of an image opened with <see cref="Begin"/>.</summary>
    /// <param name="chunk">The next bytes of the image, in order.</param>
    /// <exception cref="PamojaException">
    /// No transfer is open, or more bytes arrived than the manifest declared.
    /// </exception>
    public void Write(ReadOnlySpan<byte> chunk)
    {
        byte[] bytes = chunk.ToArray();
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_write(handle, bytes, (nuint)bytes.Length)));
    }

    /// <summary>Reports how much of an opened image has arrived.</summary>
    /// <returns>The bytes stored so far and the total the manifest declares.</returns>
    /// <exception cref="PamojaException">No transfer is open.</exception>
    public Progress CurrentProgress()
    {
        uint written = 0;
        uint total = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_progress(handle, out written, out total)));
        return new Progress(written, total);
    }

    /// <summary>Finishes an opened image, marking the slot bootable if it matched.</summary>
    /// <returns>The slot now holding a staged image.</returns>
    /// <exception cref="PamojaException">
    /// No transfer is open, or the image is not the one the manifest described,
    /// which leaves the slot unbootable.
    /// </exception>
    public byte Finish()
    {
        byte slot = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_finish(handle, out slot)));
        return slot;
    }

    /// <summary>Decides what to run, recording the decision before returning it.</summary>
    /// <remarks>
    /// Call this once per boot, before jumping to an image. A staged image becomes
    /// pending here, so a device that resets before confirming reverts on the next
    /// call rather than trying a broken image forever.
    /// </remarks>
    /// <returns>What the bootloader should do.</returns>
    /// <exception cref="PamojaException">There is nothing to fall back to.</exception>
    public BootDecision OnBoot()
    {
        PamojaBoot boot = default;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_on_boot(handle, out boot)));
        return new BootDecision((BootAction)boot.Action, boot.Slot, boot.Fallback);
    }

    /// <summary>Confirms the pending image, so it will be run from now on.</summary>
    /// <returns>The slot that is now confirmed.</returns>
    /// <exception cref="PamojaException">There is no pending image.</exception>
    public byte Confirm()
    {
        byte slot = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_confirm(handle, out slot)));
        return slot;
    }

    /// <summary>Fails the pending image and goes back to the confirmed one.</summary>
    /// <returns>The slot to fall back to.</returns>
    /// <exception cref="PamojaException">There is nothing to fall back to.</exception>
    public byte Revert()
    {
        byte slot = 0;
        Status.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_updater_revert(handle, out slot)));
        return slot;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Signing releases and checking the ones that arrive.</summary>
public static class Update
{
    /// <summary>The manifest structure version this build writes.</summary>
    public const byte StructureVersion = NativeMethods.UpdateStructureVersion;

    /// <summary>The payload format meaning the payload is the image itself.</summary>
    public const byte FormatRaw = NativeMethods.UpdateFormatRaw;

    /// <summary>Encodes the body of a manifest, which is what a signature covers.</summary>
    /// <param name="manifest">The manifest to encode.</param>
    /// <returns>The encoded body.</returns>
    /// <exception cref="PamojaException">The manifest could not be encoded.</exception>
    public static byte[] EncodeManifest(Manifest manifest) =>
        TakeOrThrow(NativeMethods.pamoja_manifest_encode(ToNative(manifest)));

    /// <summary>Reads a manifest body back from its bytes.</summary>
    /// <remarks>
    /// This reads what a manifest claims; it proves nothing about who wrote it.
    /// Use <see cref="VerifyEnvelope"/> to read one whose signature was checked.
    /// </remarks>
    /// <param name="bytes">The encoded manifest body.</param>
    /// <returns>The manifest.</returns>
    /// <exception cref="PamojaException">The bytes are not a well-formed manifest.</exception>
    public static Manifest DecodeManifest(ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(NativeMethods.pamoja_manifest_decode(
            bytes, (nuint)bytes.Length, out PamojaManifest manifest));
        return FromNative(manifest);
    }

    /// <summary>Hashes a complete image, for a publisher filling in a manifest.</summary>
    /// <remarks>
    /// The manifest commits to a SHA-256 over the image, and this is that hash, so a
    /// publisher does not need a hashing library of its own just to name the image it is
    /// releasing.
    /// </remarks>
    /// <param name="image">The complete image the release carries.</param>
    /// <returns>The 32-byte digest to put in a <see cref="Manifest"/>.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] ImageDigest(ReadOnlySpan<byte> image)
    {
        byte[] digest = new byte[32];
        Status.ThrowIfError(NativeMethods.pamoja_image_digest(image, (nuint)image.Length, digest));
        return digest;
    }

    /// <summary>Signs a manifest into the envelope offered to a device.</summary>
    /// <param name="manifest">What the release says about itself.</param>
    /// <param name="author">The identity signing the release.</param>
    /// <returns>The signed envelope.</returns>
    /// <exception cref="PamojaException">The manifest could not be signed.</exception>
    public static byte[] SignManifest(Manifest manifest, DeviceIdentity author)
    {
        ArgumentNullException.ThrowIfNull(author);
        PamojaManifest native = ToNative(manifest);
        return TakeOrThrow(author.Use(handle =>
            NativeMethods.pamoja_manifest_sign(native, handle)));
    }

    /// <summary>Verifies an envelope and reads the manifest inside it.</summary>
    /// <param name="envelope">The signed envelope.</param>
    /// <param name="publicKey">The key expected to have signed it.</param>
    /// <returns>The verified manifest.</returns>
    /// <exception cref="PamojaException">The signature is not from that key.</exception>
    public static Manifest VerifyEnvelope(
        ReadOnlySpan<byte> envelope,
        ReadOnlySpan<byte> publicKey)
    {
        Status.ThrowIfError(NativeMethods.pamoja_envelope_verify(
            envelope, (nuint)envelope.Length, publicKey, out PamojaManifest manifest));
        return FromNative(manifest);
    }

    /// <summary>Copies out the signed body of an envelope, unchecked.</summary>
    /// <remarks>This is what a gateway relays onward unchanged.</remarks>
    /// <param name="envelope">The signed envelope.</param>
    /// <returns>The signed body.</returns>
    /// <exception cref="PamojaException">The envelope is malformed.</exception>
    public static byte[] EnvelopeBody(ReadOnlySpan<byte> envelope) =>
        TakeOrThrow(NativeMethods.pamoja_envelope_body(envelope, (nuint)envelope.Length));

    /// <summary>Signs a delegation, naming a release key the anchor stands behind.</summary>
    /// <remarks>
    /// Keeping the anchor offline and rotating a release key under it is the
    /// arrangement to prefer, because the key that signs day to day is the one
    /// most likely to be stolen.
    /// </remarks>
    /// <param name="delegation">The statement to sign.</param>
    /// <param name="anchor">The anchor identity, the root of the trust.</param>
    /// <returns>The signed delegation envelope.</returns>
    /// <exception cref="PamojaException">The delegation could not be signed.</exception>
    public static byte[] SignDelegation(Delegation delegation, DeviceIdentity anchor)
    {
        ArgumentNullException.ThrowIfNull(delegation);
        ArgumentNullException.ThrowIfNull(anchor);
        PamojaDelegation native = new()
        {
            Epoch = delegation.Epoch,
            ReleaseKey = PamojaDigest.From(delegation.ReleaseKey, nameof(delegation)),
            Expires = delegation.Expires,
        };
        return TakeOrThrow(anchor.Use(handle =>
            NativeMethods.pamoja_delegation_sign(native, handle)));
    }

    /// <summary>Opens a signed delegation against the anchor that signed it.</summary>
    /// <param name="envelope">The signed delegation envelope.</param>
    /// <param name="anchorPublicKey">The anchor key.</param>
    /// <returns>The verified delegation.</returns>
    /// <exception cref="PamojaException">The delegation is not from the anchor.</exception>
    public static Delegation OpenDelegation(
        ReadOnlySpan<byte> envelope,
        ReadOnlySpan<byte> anchorPublicKey)
    {
        Status.ThrowIfError(NativeMethods.pamoja_delegation_open(
            envelope,
            (nuint)envelope.Length,
            anchorPublicKey,
            out PamojaDelegation delegation));
        return FromNative(delegation);
    }

    /// <summary>Converts a manifest into the blittable form the C ABI takes.</summary>
    /// <param name="manifest">The manifest to convert.</param>
    /// <returns>The blittable manifest.</returns>
    /// <exception cref="ArgumentException">A fixed-width field is the wrong length.</exception>
    internal static PamojaManifest ToNative(Manifest manifest)
    {
        ArgumentNullException.ThrowIfNull(manifest);
        return new PamojaManifest
        {
            StructureVersion = manifest.StructureVersion,
            Sequence = manifest.Sequence,
            VendorId = PamojaId.From(manifest.VendorId, nameof(manifest)),
            ClassId = PamojaId.From(manifest.ClassId, nameof(manifest)),
            Format = manifest.Format,
            Storage = manifest.Storage,
            Digest = PamojaDigest.From(manifest.Digest, nameof(manifest)),
            Size = manifest.Size,
            Expires = manifest.Expires,
        };
    }

    /// <summary>Converts a blittable manifest into the managed record.</summary>
    private static Manifest FromNative(PamojaManifest manifest) => new(
        manifest.Sequence,
        manifest.VendorId.ToArray(),
        manifest.ClassId.ToArray(),
        manifest.Storage,
        manifest.Digest.ToArray(),
        manifest.Size,
        manifest.Expires,
        manifest.Format,
        manifest.StructureVersion);

    /// <summary>Converts a blittable delegation into the managed record.</summary>
    internal static Delegation FromNative(PamojaDelegation delegation) => new(
        delegation.Epoch,
        delegation.ReleaseKey.ToArray(),
        delegation.Expires);

    /// <summary>Reads a produced buffer, or throws when the call returned null.</summary>
    private static byte[] TakeOrThrow(IntPtr buffer) => buffer != IntPtr.Zero
        ? Pamoja.Codec.Codec.TakeBytes(buffer)
        : throw new PamojaException(Status.LastError() ?? "the update call failed");
}
