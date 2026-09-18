using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the four LoRaWAN application layer packages, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The port clock synchronization is spoken on, TS003-2.0.0.</summary>
    public const byte LorawanClockPort = 202;

    /// <summary>The port fragmented data block transport is spoken on, TS004-2.0.0.</summary>
    public const byte LorawanFragmentPort = 201;

    /// <summary>The port remote multicast setup is spoken on, TS005-2.0.0.</summary>
    public const byte LorawanMulticastPort = 200;

    /// <summary>The port firmware management is spoken on, TS006-1.0.0.</summary>
    public const byte LorawanFirmwarePort = 203;

    /// <summary>The most fragments one session carries.</summary>
    public const ushort LorawanMaxFragments = 16383;

    /// <summary>The device holds no firmware upgrade image.</summary>
    public const byte LorawanImageNone = 0;

    /// <summary>One is there, but it is corrupt or its signature does not verify.</summary>
    public const byte LorawanImageCorrupt = 1;

    /// <summary>One is there and authentic, but it is not for this hardware.</summary>
    public const byte LorawanImageWrongHardware = 2;

    /// <summary>One is there, and it can be installed.</summary>
    public const byte LorawanImageValid = 3;

    /// <summary>Derives a device's multicast root key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_mc_root_key(
        ReadOnlySpan<byte> rootKey,
        byte lorawan11,
        Span<byte> outKey);

    /// <summary>Derives the key a multicast group's key travels under.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_mc_ke_key(
        ReadOnlySpan<byte> mcRootKey,
        Span<byte> outKey);

    /// <summary>Unwraps the group key a setup command carried.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_mc_key(
        ReadOnlySpan<byte> mcKeKey,
        ReadOnlySpan<byte> wrapped,
        Span<byte> outKey);

    /// <summary>Wraps a group key for a device, which is what a server does.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_wrap_mc_key(
        ReadOnlySpan<byte> mcKeKey,
        ReadOnlySpan<byte> mcKey,
        Span<byte> outWrapped);

    /// <summary>Derives the key that reads a multicast group's payloads.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_mc_app_s_key(
        ReadOnlySpan<byte> mcKey,
        uint mcAddr,
        Span<byte> outKey);

    /// <summary>Derives the key that verifies a multicast group's frames.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_mc_nwk_s_key(
        ReadOnlySpan<byte> mcKey,
        uint mcAddr,
        Span<byte> outKey);

    /// <summary>Derives the key that signs a data block.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_data_block_int_key(
        ReadOnlySpan<byte> rootKey,
        Span<byte> outKey);

    /// <summary>Steps the pseudo-random sequence the parity matrix is drawn from.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_lorawan_frag_prbs23(uint x);

    /// <summary>Writes the bitmap of the fragments one coded fragment is made of.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_frag_parity_line(
        ushort coded,
        ushort nbFrag,
        Span<byte> outLine,
        nuint lineLen,
        out nuint outOnes);

    /// <summary>Says how many fragments a block takes and how much padding it needs.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_frag_session(
        nuint blockLen,
        byte fragSize,
        out ushort outNbFrag,
        out byte outPadding);

    /// <summary>Builds one fragment of a session out of a block held whole.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_frag_fragment(
        ReadOnlySpan<byte> block,
        nuint blockLen,
        byte fragSize,
        ushort n,
        Span<byte> outFragment,
        nuint outLen);

    /// <summary>The working memory a session of this shape takes, in bytes.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_lorawan_defrag_matrix_len(ushort nbFrag, ushort maxLost);

    /// <summary>Opens a session for a block being put back together.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_defrag_new(
        ushort nbFrag,
        byte fragSize,
        ushort maxLost,
        out IntPtr outSession);

    /// <summary>Releases a session.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_defrag_free(IntPtr session);

    /// <summary>Takes one fragment of a session, counting from one.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_defrag_fragment(
        IntPtr session,
        ushort n,
        ReadOnlySpan<byte> fragment,
        nuint fragmentLen,
        out byte outDone);

    /// <summary>Says how far along a session is.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_defrag_status(
        IntPtr session,
        out ushort outReceived,
        out ushort outMissing,
        out byte outDone);

    /// <summary>Copies out the block as far as it has been put back together.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_defrag_block(
        IntPtr session,
        Span<byte> outBlock,
        nuint capacity,
        out nuint outLen);

    /// <summary>Starts a code over one session's block.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_block_mic_start(
        ReadOnlySpan<byte> dataBlockIntKey,
        ushort sessionCnt,
        byte fragIndex,
        ReadOnlySpan<byte> descriptor,
        uint blockLen,
        out IntPtr outMic);

    /// <summary>Adds a piece of the block, in order, without any padding.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_block_mic_update(
        IntPtr mic,
        ReadOnlySpan<byte> data,
        nuint dataLen);

    /// <summary>Finishes the code, writing the four bytes a session setup carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_block_mic_finish(
        IntPtr mic,
        Span<byte> outMic);

    /// <summary>Releases a code that was never finished.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_block_mic_free(IntPtr mic);

    /// <summary>Starts the clock synchronization package on a device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_clock_sync_new(
        byte selfManaged,
        out IntPtr outSync);

    /// <summary>Releases a clock synchronization package.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_clock_sync_free(IntPtr sync);

    /// <summary>Builds the request that asks a server for a correction.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_clock_sync_request(
        IntPtr sync,
        uint deviceTime,
        byte ansRequired,
        Span<byte> outCommand,
        nuint capacity,
        out nuint outLen);

    /// <summary>Reads a downlink on the clock port and acts on it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_clock_sync_heard(
        IntPtr sync,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out int outCorrection,
        out byte outHasCorrection,
        out byte outMore,
        out byte outResync,
        out byte outAnswerDue);

    /// <summary>Writes the answer the device owes, if it owes one.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_clock_sync_answer(
        IntPtr sync,
        uint deviceTime,
        Span<byte> outCommand,
        nuint capacity,
        out nuint outLen);

    /// <summary>Reports the token, the period, and whether an answer is owed.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_clock_sync_status(
        IntPtr sync,
        out byte outToken,
        out uint outPeriodS,
        out byte outAnswerDue);

    /// <summary>Starts the firmware management package on a device.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_new(
        uint firmware,
        uint hardware,
        out IntPtr outManager);

    /// <summary>Releases a firmware manager.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lorawan_firmware_free(IntPtr manager);

    /// <summary>Says what firmware upgrade image the device is holding.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_set_image(
        IntPtr manager,
        byte status,
        uint version);

    /// <summary>Reads back what the device is holding.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_image(
        IntPtr manager,
        out byte outStatus,
        out uint outNextVersion,
        out byte outHasNextVersion);

    /// <summary>Reads a downlink on the firmware port and writes the answers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_heard(
        IntPtr manager,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        uint nowS,
        byte hasNow,
        Span<byte> outAnswers,
        nuint capacity,
        out nuint outLen);

    /// <summary>Reports the one reboot the device keeps.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_reboot(
        IntPtr manager,
        out uint outAtS,
        out byte outHasAt,
        out uint outInS,
        out byte outHasIn,
        out byte outNow);

    /// <summary>Forgets the programmed reboot, for a device that carried it out.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_firmware_rebooted(IntPtr manager);

    /// <summary>Reads one command of an application layer package.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_package_parse(
        byte port,
        byte uplink,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out PamojaLorawanPackageCommand outCommand,
        out nuint outTaken);

    /// <summary>Writes one command of an application layer package.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_package_encode(
        in PamojaLorawanPackageCommand command,
        ReadOnlySpan<byte> data,
        nuint dataLen,
        Span<byte> outPayload,
        nuint capacity,
        out nuint outLen);

    /// <summary>Reads one group record of a multicast status answer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lorawan_package_status_item(
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out PamojaLorawanPackageCommand outCommand);
}
