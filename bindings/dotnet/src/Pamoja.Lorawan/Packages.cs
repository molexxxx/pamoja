using Pamoja.Native;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>What a device makes of the firmware upgrade image it holds, TS006-1.0.0 table 10.</summary>
public enum LorawanImageStatus : byte
{
    /// <summary>It is holding none.</summary>
    None = NativeMethods.LorawanImageNone,

    /// <summary>One is there, but it is corrupt or its signature does not verify.</summary>
    Corrupt = NativeMethods.LorawanImageCorrupt,

    /// <summary>One is there and authentic, but it is not for this hardware.</summary>
    WrongHardware = NativeMethods.LorawanImageWrongHardware,

    /// <summary>One is there, and it can be installed.</summary>
    Valid = NativeMethods.LorawanImageValid,
}

/// <summary>How a block is cut into fragments.</summary>
/// <param name="NbFrag">How many uncoded fragments the block takes.</param>
/// <param name="Padding">How many bytes of padding the last one carries.</param>
public readonly record struct LorawanFragSession(ushort NbFrag, byte Padding);

/// <summary>What a downlink on the clock port asked of a device.</summary>
/// <param name="Correction">The seconds to add to the clock, where an answer carried one.</param>
/// <param name="MoreCorrection">Whether the correction was the largest the field carries.</param>
/// <param name="Resync">How many requests a resynchronization command asked for.</param>
/// <param name="AnswerDue">Whether the device now owes an answer.</param>
public readonly record struct LorawanClockHeard(
    int? Correction,
    bool MoreCorrection,
    byte? Resync,
    bool AnswerDue);

/// <summary>The one reboot a device keeps.</summary>
/// <param name="AtS">The moment it is to reboot, where one was set as a time.</param>
/// <param name="InS">How long until it reboots, where one was set as a countdown.</param>
/// <param name="Now">Whether it was told to reboot at once.</param>
public readonly record struct LorawanReboot(uint? AtS, uint? InS, bool Now);

/// <summary>
/// The four LoRaWAN application layer packages: clock synchronization TS003-2.0.0, fragmented
/// data block transport TS004-2.0.0, remote multicast setup TS005-2.0.0 and firmware
/// management TS006-1.0.0.
/// </summary>
/// <remarks>
/// The key derivations and the parity matrix are functions of their arguments and live here.
/// What has to remember something is a class: <see cref="LorawanClockSync"/>,
/// <see cref="LorawanFirmware"/>, <see cref="LorawanDefragmenter"/> and
/// <see cref="LorawanBlockMic"/>. One command of any of the four is
/// <see cref="LorawanPackageCommand"/>.
/// </remarks>
public static class LorawanPackages
{
    /// <summary>The port clock synchronization is spoken on.</summary>
    public const byte ClockPort = NativeMethods.LorawanClockPort;

    /// <summary>The port fragmented data block transport is spoken on.</summary>
    public const byte FragmentPort = NativeMethods.LorawanFragmentPort;

    /// <summary>The port remote multicast setup is spoken on.</summary>
    public const byte MulticastPort = NativeMethods.LorawanMulticastPort;

    /// <summary>The port firmware management is spoken on.</summary>
    public const byte FirmwarePort = NativeMethods.LorawanFirmwarePort;

    /// <summary>The most fragments one session carries.</summary>
    public const ushort MaxFragments = NativeMethods.LorawanMaxFragments;

    /// <summary>Derives a device's multicast root key, TS005-2.0.0 section 4.3.</summary>
    /// <param name="rootKey">The device's <c>GenAppKey</c> on 1.0.x, or its <c>AppKey</c> on 1.1.</param>
    /// <param name="lorawan11">Whether to use the 1.1 scheme, which starts from another constant.</param>
    /// <returns>The sixteen-byte <c>McRootKey</c>.</returns>
    /// <exception cref="PamojaException">The key was not sixteen bytes.</exception>
    public static byte[] McRootKey(ReadOnlySpan<byte> rootKey, bool lorawan11 = false)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mc_root_key(
            Sixteen(rootKey, nameof(rootKey)),
            lorawan11 ? (byte)1 : (byte)0,
            key));
        return key;
    }

    /// <summary>Derives the key a multicast group's key travels under.</summary>
    /// <param name="mcRootKey">What <see cref="McRootKey"/> derived.</param>
    /// <returns>The sixteen-byte <c>McKEKey</c>.</returns>
    /// <exception cref="PamojaException">The key was not sixteen bytes.</exception>
    public static byte[] McKeKey(ReadOnlySpan<byte> mcRootKey)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mc_ke_key(
            Sixteen(mcRootKey, nameof(mcRootKey)),
            key));
        return key;
    }

    /// <summary>Unwraps the group key a setup command carried.</summary>
    /// <param name="mcKeKey">What <see cref="McKeKey"/> derived.</param>
    /// <param name="wrapped">The wrapped key the command carried.</param>
    /// <returns>The group key.</returns>
    /// <exception cref="PamojaException">Either value was not sixteen bytes.</exception>
    public static byte[] McKey(ReadOnlySpan<byte> mcKeKey, ReadOnlySpan<byte> wrapped)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mc_key(
            Sixteen(mcKeKey, nameof(mcKeKey)),
            Sixteen(wrapped, nameof(wrapped)),
            key));
        return key;
    }

    /// <summary>Wraps a group key for a device, which is what a server does before sending it.</summary>
    /// <param name="mcKeKey">The device's key encryption key.</param>
    /// <param name="mcKey">The group key to wrap.</param>
    /// <returns>The wrapped key a setup command carries.</returns>
    /// <exception cref="PamojaException">Either key was not sixteen bytes.</exception>
    public static byte[] WrapMcKey(ReadOnlySpan<byte> mcKeKey, ReadOnlySpan<byte> mcKey)
    {
        byte[] wrapped = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_wrap_mc_key(
            Sixteen(mcKeKey, nameof(mcKeKey)),
            Sixteen(mcKey, nameof(mcKey)),
            wrapped));
        return wrapped;
    }

    /// <summary>Derives the key that reads a multicast group's payloads.</summary>
    /// <param name="mcKey">The group key.</param>
    /// <param name="mcAddr">The address the group answers to.</param>
    /// <returns>The group's <c>McAppSKey</c>.</returns>
    /// <exception cref="PamojaException">The key was not sixteen bytes.</exception>
    public static byte[] McAppSKey(ReadOnlySpan<byte> mcKey, uint mcAddr)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mc_app_s_key(
            Sixteen(mcKey, nameof(mcKey)),
            mcAddr,
            key));
        return key;
    }

    /// <summary>Derives the key that verifies a multicast group's frames.</summary>
    /// <param name="mcKey">The group key.</param>
    /// <param name="mcAddr">The address the group answers to.</param>
    /// <returns>The group's <c>McNwkSKey</c>.</returns>
    /// <exception cref="PamojaException">The key was not sixteen bytes.</exception>
    public static byte[] McNwkSKey(ReadOnlySpan<byte> mcKey, uint mcAddr)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_mc_nwk_s_key(
            Sixteen(mcKey, nameof(mcKey)),
            mcAddr,
            key));
        return key;
    }

    /// <summary>Derives the key that signs a data block, TS004-2.0.0 section 3.3.</summary>
    /// <param name="rootKey">The device's root key.</param>
    /// <returns>The sixteen-byte <c>DataBlockIntKey</c>.</returns>
    /// <exception cref="PamojaException">The key was not sixteen bytes.</exception>
    public static byte[] DataBlockIntKey(ReadOnlySpan<byte> rootKey)
    {
        byte[] key = new byte[16];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_data_block_int_key(
            Sixteen(rootKey, nameof(rootKey)),
            key));
        return key;
    }

    /// <summary>Steps the pseudo-random sequence the parity matrix is drawn from.</summary>
    /// <param name="x">The current state.</param>
    /// <returns>The next one.</returns>
    public static uint FragPrbs23(uint x) => NativeMethods.pamoja_lorawan_frag_prbs23(x);

    /// <summary>Lists the uncoded fragments one coded fragment is made of.</summary>
    /// <param name="coded">Which coded fragment, counting from one past the uncoded ones.</param>
    /// <param name="nbFrag">How many uncoded fragments the session carries.</param>
    /// <returns>The fragments it is the exclusive-or of, counting from zero.</returns>
    /// <exception cref="PamojaException">The session is not one this build runs.</exception>
    public static ushort[] FragParityLine(ushort coded, ushort nbFrag)
    {
        byte[] line = new byte[(nbFrag + 7) / 8];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_frag_parity_line(
            coded,
            nbFrag,
            line,
            (nuint)line.Length,
            out nuint ones));

        ushort[] fragments = new ushort[(int)ones];
        int at = 0;
        for (ushort n = 0; n < nbFrag; n++)
        {
            if ((line[n / 8] & (1 << (n % 8))) != 0)
            {
                fragments[at++] = n;
            }
        }
        return fragments;
    }

    /// <summary>Says how many fragments a block takes, and how much padding the last one needs.</summary>
    /// <param name="blockLen">The block's length in bytes.</param>
    /// <param name="fragSize">How many bytes each fragment carries.</param>
    /// <returns>The shape of the session.</returns>
    /// <exception cref="PamojaException">The block does not fit a session this build runs.</exception>
    public static LorawanFragSession FragSession(int blockLen, byte fragSize)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_frag_session(
            (nuint)blockLen,
            fragSize,
            out ushort nbFrag,
            out byte padding));
        return new LorawanFragSession(nbFrag, padding);
    }

    /// <summary>Builds one fragment of a session out of a block held whole.</summary>
    /// <remarks>
    /// Up to the session's fragment count the fragment is a piece of the block; past that it
    /// is a coded fragment, the exclusive-or of a pseudo-random half of the pieces.
    /// </remarks>
    /// <param name="block">The block, padding and all.</param>
    /// <param name="fragSize">How many bytes each fragment carries.</param>
    /// <param name="n">Which fragment, counting from one.</param>
    /// <returns>Its bytes.</returns>
    /// <exception cref="PamojaException">The fragment is not one of this session.</exception>
    public static byte[] FragFragment(ReadOnlySpan<byte> block, byte fragSize, ushort n)
    {
        byte[] fragment = new byte[fragSize];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_frag_fragment(
            block,
            (nuint)block.Length,
            fragSize,
            n,
            fragment,
            (nuint)fragment.Length));
        return fragment;
    }

    internal static ReadOnlySpan<byte> Sixteen(ReadOnlySpan<byte> bytes, string name) =>
        bytes.Length == 16
            ? bytes
            : throw new ArgumentException($"{name} must be exactly 16 bytes", name);
}

/// <summary>
/// A fragmentation session being put back together, TS004-2.0.0 appendix A.2.
/// </summary>
/// <remarks>
/// The uncoded fragments go straight into the block. A coded fragment is reduced against
/// everything already known and kept only if it says something new, so the working memory is
/// sized by the losses rather than by the block.
/// </remarks>
public sealed class LorawanDefragmenter : IDisposable
{
    private readonly NativeHandle _handle;
    private readonly int _blockLen;

    /// <summary>Opens a session for a block of <paramref name="nbFrag"/> fragments.</summary>
    /// <param name="nbFrag">How many uncoded fragments the block was cut into.</param>
    /// <param name="fragSize">How many bytes each fragment carries.</param>
    /// <param name="maxLost">
    /// The most uncoded fragments to be able to solve for, which decides how much working
    /// memory the session takes.
    /// </param>
    /// <exception cref="PamojaException">The session is not one this build runs.</exception>
    public LorawanDefragmenter(ushort nbFrag, byte fragSize, ushort maxLost)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_defrag_new(
            nbFrag,
            fragSize,
            maxLost,
            out IntPtr session));
        _handle = NativeHandle.Create(
            session,
            NativeMethods.pamoja_lorawan_defrag_free,
            "fragmentation session");
        _blockLen = nbFrag * fragSize;
    }

    /// <summary>The working memory a session of this shape takes, in bytes.</summary>
    /// <param name="nbFrag">How many uncoded fragments the block was cut into.</param>
    /// <param name="maxLost">How many of them the session can solve for.</param>
    /// <returns>The byte count the session allocates for itself.</returns>
    public static int MatrixLength(ushort nbFrag, ushort maxLost) =>
        (int)NativeMethods.pamoja_lorawan_defrag_matrix_len(nbFrag, maxLost);

    /// <summary>Takes one fragment of the session, counting from one.</summary>
    /// <param name="n">Which fragment.</param>
    /// <param name="data">Its bytes.</param>
    /// <returns><c>true</c> once the block is whole.</returns>
    /// <exception cref="PamojaException">
    /// The fragment is not one of this session, or the session ran out of memory to solve
    /// with.
    /// </exception>
    public bool Fragment(ushort n, ReadOnlySpan<byte> data)
    {
        byte[] bytes = data.ToArray();
        byte done = _handle.Use(session =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_defrag_fragment(
                session,
                n,
                bytes,
                (nuint)bytes.Length,
                out byte finished));
            return finished;
        });
        return done != 0;
    }

    /// <summary>Whether the whole block is there.</summary>
    public bool Done => Progress().Done;

    /// <summary>How many fragments arrived, coded, uncoded and repeated.</summary>
    public ushort Received => Progress().Received;

    /// <summary>How many uncoded fragments are still missing.</summary>
    public ushort Missing => Progress().Missing;

    /// <summary>The block, as far as it has been put back together, padding and all.</summary>
    /// <returns>The block.</returns>
    public byte[] Block()
    {
        byte[] block = new byte[_blockLen];
        _handle.Use(session =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_defrag_block(
                session,
                block,
                (nuint)block.Length,
                out nuint _));
        });
        return block;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    private (ushort Received, ushort Missing, bool Done) Progress() =>
        _handle.Use(session =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_defrag_status(
                session,
                out ushort received,
                out ushort missing,
                out byte done));
            return (received, missing, done != 0);
        });
}

/// <summary>The code taken over a data block as it arrives, TS004-2.0.0 section 3.3.</summary>
public sealed class LorawanBlockMic : IDisposable
{
    private readonly NativeHandle _handle;
    private bool _finished;

    /// <summary>Starts a code over one session's block.</summary>
    /// <param name="dataBlockIntKey">What <see cref="LorawanPackages.DataBlockIntKey"/> derived.</param>
    /// <param name="sessionCnt">The session counter the setup carried.</param>
    /// <param name="fragIndex">Which session.</param>
    /// <param name="descriptor">The four bytes the server described the block with.</param>
    /// <param name="blockLen">The block's length in bytes, padding excluded.</param>
    /// <exception cref="PamojaException">The key or the descriptor was the wrong length.</exception>
    public LorawanBlockMic(
        ReadOnlySpan<byte> dataBlockIntKey,
        ushort sessionCnt,
        byte fragIndex,
        ReadOnlySpan<byte> descriptor,
        uint blockLen)
    {
        if (descriptor.Length != 4)
        {
            throw new ArgumentException("descriptor must be exactly 4 bytes", nameof(descriptor));
        }

        Status.ThrowIfError(NativeMethods.pamoja_lorawan_block_mic_start(
            LorawanPackages.Sixteen(dataBlockIntKey, nameof(dataBlockIntKey)),
            sessionCnt,
            fragIndex,
            descriptor,
            blockLen,
            out IntPtr mic));

        _handle = NativeHandle.Create(mic, NativeMethods.pamoja_lorawan_block_mic_free, "block code");
    }

    /// <summary>Adds a piece of the block, in order, without any padding.</summary>
    /// <param name="data">The piece.</param>
    /// <exception cref="PamojaException">The code was already finished.</exception>
    public void Update(ReadOnlySpan<byte> data)
    {
        if (_finished)
        {
            throw new PamojaException("this code has already been finished");
        }

        byte[] bytes = data.ToArray();
        _handle.Use(mic =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_block_mic_update(
                mic,
                bytes,
                (nuint)bytes.Length));
        });
    }

    /// <summary>Finishes the code, returning the four bytes a session setup carries.</summary>
    /// <returns>The code.</returns>
    /// <exception cref="PamojaException">The code was already finished.</exception>
    public byte[] Finish()
    {
        if (_finished)
        {
            throw new PamojaException("this code has already been finished");
        }

        byte[] code = new byte[4];
        _handle.Use(mic =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_block_mic_finish(mic, code));
        });
        // The native call consumed the handle, so releasing it again would be a double free.
        _finished = true;
        _handle.SetHandleAsInvalid();
        return code;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>The clock synchronization package running on a device, TS003-2.0.0.</summary>
/// <remarks>
/// A device that has no clock of its own still needs the time: a multicast window and a
/// scheduled reboot both depend on it. The device says what time it believes it is, the server
/// answers with the difference, and a four-bit token keeps a late answer from pulling a
/// corrected clock back.
/// </remarks>
public sealed class LorawanClockSync : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Starts the package.</summary>
    /// <param name="selfManaged">
    /// Whether the device keeps its own periodicity and answers a server that tries to set one
    /// with the not-supported bit.
    /// </param>
    public LorawanClockSync(bool selfManaged = false)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_clock_sync_new(
            selfManaged ? (byte)1 : (byte)0,
            out IntPtr sync));
        _handle = NativeHandle.Create(
            sync,
            NativeMethods.pamoja_lorawan_clock_sync_free,
            "clock synchronization package");
    }

    /// <summary>Builds the request that asks a server for a correction.</summary>
    /// <param name="deviceTime">What the device believes the time is.</param>
    /// <param name="ansRequired">Whether the server must answer even when the clock is right.</param>
    /// <returns>The payload to send on <see cref="LorawanPackages.ClockPort"/>.</returns>
    public byte[] Request(uint deviceTime, bool ansRequired = false)
    {
        byte[] buffer = new byte[LorawanPackageCommand.MaxLength];
        nuint written = _handle.Use(sync =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_clock_sync_request(
                sync,
                deviceTime,
                ansRequired ? (byte)1 : (byte)0,
                buffer,
                (nuint)buffer.Length,
                out nuint len));
            return len;
        });
        return buffer[..(int)written];
    }

    /// <summary>Reads a downlink on the clock port and acts on it.</summary>
    /// <param name="payload">What arrived.</param>
    /// <returns>What it asked of the device.</returns>
    /// <exception cref="PamojaException">The payload is not a clock synchronization command.</exception>
    public LorawanClockHeard Heard(ReadOnlySpan<byte> payload)
    {
        int correction = 0;
        byte hasCorrection = 0;
        byte more = 0;
        byte resync = 0;
        byte answerDue = 0;
        byte[] bytes = payload.ToArray();
        _handle.Use(sync =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_clock_sync_heard(
                sync,
                bytes,
                (nuint)bytes.Length,
                out correction,
                out hasCorrection,
                out more,
                out resync,
                out answerDue));
        });
        return new LorawanClockHeard(
            hasCorrection != 0 ? correction : null,
            more != 0,
            resync != 0 ? resync : null,
            answerDue != 0);
    }

    /// <summary>Writes the answer the device owes, or nothing when it owes none.</summary>
    /// <param name="deviceTime">What the device believes the time is.</param>
    /// <returns>The payload to send, which is empty when nothing is owed.</returns>
    public byte[] Answer(uint deviceTime)
    {
        byte[] buffer = new byte[LorawanPackageCommand.MaxLength];
        nuint written = _handle.Use(sync =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_clock_sync_answer(
                sync,
                deviceTime,
                buffer,
                (nuint)buffer.Length,
                out nuint len));
            return len;
        });
        return buffer[..(int)written];
    }

    /// <summary>The token the next request will carry.</summary>
    public byte Token => Read().Token;

    /// <summary>The seconds between requests, as the server last set them.</summary>
    public uint PeriodS => Read().PeriodS;

    /// <summary>Whether the device owes its server an answer.</summary>
    public bool AnswerDue => Read().AnswerDue;

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    private (byte Token, uint PeriodS, bool AnswerDue) Read() =>
        _handle.Use(sync =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_clock_sync_status(
                sync,
                out byte token,
                out uint periodS,
                out byte answerDue));
            return (token, periodS, answerDue != 0);
        });
}

/// <summary>The firmware management package running on a device, TS006-1.0.0.</summary>
/// <remarks>
/// What the device is running, what upgrade image it holds and whether that image can be
/// installed, and the single reboot it keeps, set either as a moment in time or as a countdown.
/// </remarks>
public sealed class LorawanFirmware : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Starts the package, reporting the versions the device was built with.</summary>
    /// <param name="firmware">The firmware it is running.</param>
    /// <param name="hardware">The hardware it runs on.</param>
    public LorawanFirmware(uint firmware, uint hardware)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_new(
            firmware,
            hardware,
            out IntPtr manager));
        _handle = NativeHandle.Create(
            manager,
            NativeMethods.pamoja_lorawan_firmware_free,
            "firmware manager");
    }

    /// <summary>Says what firmware upgrade image the device is holding.</summary>
    /// <param name="status">What the device makes of it.</param>
    /// <param name="version">What it would boot into, for an image it can install.</param>
    public void SetImage(LorawanImageStatus status, uint version = 0) =>
        _handle.Use(manager =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_set_image(
                manager,
                (byte)status,
                version));
        });

    /// <summary>Reads a downlink on the firmware port and writes the answers it calls for.</summary>
    /// <param name="payload">What arrived.</param>
    /// <param name="nowS">
    /// What the device believes the time is, in seconds since the GPS epoch. A device that does
    /// not know refuses a reboot set for a moment in time.
    /// </param>
    /// <returns>The answers to send back, which may be empty.</returns>
    /// <exception cref="PamojaException">The payload is not a firmware management command.</exception>
    public byte[] Heard(ReadOnlySpan<byte> payload, uint? nowS = null)
    {
        byte[] bytes = payload.ToArray();
        byte[] buffer = new byte[LorawanPackageCommand.MaxLength];
        nuint written = _handle.Use(manager =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_heard(
                manager,
                bytes,
                (nuint)bytes.Length,
                nowS ?? 0,
                nowS.HasValue ? (byte)1 : (byte)0,
                buffer,
                (nuint)buffer.Length,
                out nuint len));
            return len;
        });
        return buffer[..(int)written];
    }

    /// <summary>What the device makes of the image it holds.</summary>
    public LorawanImageStatus ImageStatus => Image().Status;

    /// <summary>What the device would boot into, for an image it can install.</summary>
    public uint? NextVersion => Image().NextVersion;

    /// <summary>The one reboot the device keeps.</summary>
    /// <returns>When it reboots, and whether it was told to do so at once.</returns>
    public LorawanReboot Reboot() =>
        _handle.Use(manager =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_reboot(
                manager,
                out uint atS,
                out byte hasAt,
                out uint inS,
                out byte hasIn,
                out byte now));
            return new LorawanReboot(
                hasAt != 0 ? atS : null,
                hasIn != 0 ? inS : null,
                now != 0);
        });

    /// <summary>Forgets the programmed reboot, for a device that has carried it out.</summary>
    public void Rebooted() =>
        _handle.Use(manager =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_rebooted(manager));
        });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    private (LorawanImageStatus Status, uint? NextVersion) Image() =>
        _handle.Use(manager =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_lorawan_firmware_image(
                manager,
                out byte status,
                out uint nextVersion,
                out byte hasNextVersion));
            return ((LorawanImageStatus)status, hasNextVersion != 0 ? nextVersion : (uint?)null);
        });
}
