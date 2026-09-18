using Pamoja.Native;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>
/// One command of an application layer package, whichever package it belongs to.
/// </summary>
/// <remarks>
/// <para>
/// <see cref="Port"/> says which package: clock synchronization TS003-2.0.0, fragmented
/// data block transport TS004-2.0.0, remote multicast setup TS005-2.0.0 or firmware
/// management TS006-1.0.0. <see cref="Cid"/> names the command within it.
/// </para>
/// <para>
/// The same identifier means a different command in each direction, so
/// <see cref="Uplink"/> is always given and never guessed. Only the properties that
/// command carries are set; the rest read as zero.
/// </para>
/// </remarks>
public sealed class LorawanPackageCommand
{
    /// <summary>The longest command any of the four packages writes, in bytes.</summary>
    public const int MaxLength = 64;

    /// <summary>Which package this command belongs to, as its port.</summary>
    public required byte Port { get; init; }

    /// <summary>Which command within that package.</summary>
    public required byte Cid { get; init; }

    /// <summary>Which way it travels.</summary>
    public bool Uplink { get; init; }

    /// <summary>The package identifier a version answer carries.</summary>
    public byte Package { get; init; }

    /// <summary>The package version it implements.</summary>
    public byte Version { get; init; }

    /// <summary>A device's own clock, in seconds since the GPS epoch.</summary>
    public uint DeviceTime { get; init; }

    /// <summary>The seconds to add to a device's clock.</summary>
    public int TimeCorrection { get; init; }

    /// <summary>The token that pairs a clock answer with its request.</summary>
    public byte Token { get; init; }

    /// <summary>Whether a clock request must be answered.</summary>
    public bool AnsRequired { get; init; }

    /// <summary>The coded period between clock requests.</summary>
    public byte Period { get; init; }

    /// <summary>Whether a device manages its own clock periodicity.</summary>
    public bool NotSupported { get; init; }

    /// <summary>How many requests a resynchronization command asks for.</summary>
    public byte Transmissions { get; init; }

    /// <summary>The firmware a device reports running.</summary>
    public uint Firmware { get; init; }

    /// <summary>The hardware it runs on.</summary>
    public uint Hardware { get; init; }

    /// <summary>The moment or the delay a reboot is set for.</summary>
    public uint Reboot { get; init; }

    /// <summary>What a device makes of the upgrade image it holds.</summary>
    public byte ImageStatus { get; init; }

    /// <summary>The version it would run once that image is installed.</summary>
    public uint NextVersion { get; init; }

    /// <summary>Whether a version rides with the image status.</summary>
    public bool HasNextVersion { get; init; }

    /// <summary>The version a delete command names.</summary>
    public uint DeleteVersion { get; init; }

    /// <summary>Whether a device holds no valid image.</summary>
    public bool NoValidImage { get; init; }

    /// <summary>Whether the version named is not the one held.</summary>
    public bool InvalidVersion { get; init; }

    /// <summary>Which fragmentation session, 0 to 3.</summary>
    public byte FragIndex { get; init; }

    /// <summary>Which multicast groups may feed it, a bit for each.</summary>
    public byte McGroupBitMask { get; init; }

    /// <summary>How many uncoded fragments a block was cut into.</summary>
    public ushort NbFrag { get; init; }

    /// <summary>How many bytes each fragment carries.</summary>
    public byte FragSize { get; init; }

    /// <summary>Whether a device reports the block once it has it.</summary>
    public bool AckReception { get; init; }

    /// <summary>Which fragmentation algorithm to run.</summary>
    public byte FragAlgo { get; init; }

    /// <summary>The coded spread of the delay before a device answers.</summary>
    public byte BlockAckDelay { get; init; }

    /// <summary>How many bytes of padding the last fragment carries.</summary>
    public byte Padding { get; init; }

    /// <summary>The four bytes a server describes a block with.</summary>
    public byte[] Descriptor { get; init; } = [];

    /// <summary>The session counter, which must rise for each new block.</summary>
    public ushort SessionCnt { get; init; }

    /// <summary>The code over the block a device checks once it has it all.</summary>
    public byte[] Mic { get; init; } = [];

    /// <summary>How many fragments arrived, coded, uncoded and repeated.</summary>
    public ushort Received { get; init; }

    /// <summary>How many uncoded fragments are still missing.</summary>
    public byte Missing { get; init; }

    /// <summary>Whether the block's code did not check out.</summary>
    public bool MicError { get; init; }

    /// <summary>Whether a session ran out of memory to defragment with.</summary>
    public bool MemoryError { get; init; }

    /// <summary>Whether the session or group named does not exist on the device.</summary>
    public bool NoSession { get; init; }

    /// <summary>Whether the setup named an algorithm the device does not run.</summary>
    public bool UnsupportedAlgorithm { get; init; }

    /// <summary>Whether the setup named a session index the device does not keep.</summary>
    public bool UnsupportedIndex { get; init; }

    /// <summary>Whether the descriptor is not one the device accepts.</summary>
    public bool WrongDescriptor { get; init; }

    /// <summary>Whether the session counter repeats one already used.</summary>
    public bool SessionReplay { get; init; }

    /// <summary>Whether every device answers a status request.</summary>
    public bool AllParticipants { get; init; }

    /// <summary>Whether this is one group record of a multicast status answer.</summary>
    public byte IsStatusItem { get; init; }

    /// <summary>Which fragment of a session a data fragment carries, counting from one.</summary>
    public ushort FragmentN { get; init; }

    /// <summary>Which multicast group, 0 to 3.</summary>
    public byte McGroupId { get; init; }

    /// <summary>The address a group answers to.</summary>
    public uint McAddr { get; init; }

    /// <summary>A group's key, wrapped under the device's key encryption key.</summary>
    public byte[] McKeyEncrypted { get; init; } = [];

    /// <summary>The first frame counter a device accepts from a group.</summary>
    public uint MinMcFcnt { get; init; }

    /// <summary>The last one, which ends the group's life.</summary>
    public uint MaxMcFcnt { get; init; }

    /// <summary>Which groups a status request or answer covers, a bit for each.</summary>
    public byte GroupMask { get; init; }

    /// <summary>How many groups a device holds in all.</summary>
    public byte NbTotalGroups { get; init; }

    /// <summary>Whether a device holds no group by the identifier named.</summary>
    public bool IdError { get; init; }

    /// <summary>When a multicast window opens, in seconds since the GPS epoch.</summary>
    public uint SessionTime { get; init; }

    /// <summary>How long it lasts at most, coded.</summary>
    public byte TimeOut { get; init; }

    /// <summary>How often a device opens a ping slot inside a Class B window.</summary>
    public byte Periodicity { get; init; }

    /// <summary>Where a group listens, in hertz.</summary>
    public uint DlFrequencyHz { get; init; }

    /// <summary>The data rate it listens at.</summary>
    public byte DataRate { get; init; }

    /// <summary>How many seconds until a window opens.</summary>
    public uint TimeToStart { get; init; }

    /// <summary>Whether a delay rides with the answer.</summary>
    public bool HasTimeToStart { get; init; }

    /// <summary>Whether the data rate named is not one the device has.</summary>
    public bool DrError { get; init; }

    /// <summary>Whether the frequency named is not one it can use.</summary>
    public bool FreqError { get; init; }

    /// <summary>Whether the window was to start at a time already past.</summary>
    public bool StartMissed { get; init; }

    /// <summary>The bytes a data fragment carries.</summary>
    public byte[] Data { get; init; } = [];

    /// <summary>Reads a command out of the record the C ABI hands back.</summary>
    /// <param name="flat">The record.</param>
    /// <param name="data">The bytes a data fragment carried.</param>
    /// <returns>The command.</returns>
    public static LorawanPackageCommand From(PamojaLorawanPackageCommand flat, ReadOnlySpan<byte> data = default) => new()
    {
        Port = flat.Port,
        Cid = flat.Cid,
        Uplink = flat.Uplink != 0,
        Package = flat.Package,
        Version = flat.Version,
        DeviceTime = flat.DeviceTime,
        TimeCorrection = flat.TimeCorrection,
        Token = flat.Token,
        AnsRequired = flat.AnsRequired != 0,
        Period = flat.Period,
        NotSupported = flat.NotSupported != 0,
        Transmissions = flat.Transmissions,
        Firmware = flat.Firmware,
        Hardware = flat.Hardware,
        Reboot = flat.Reboot,
        ImageStatus = flat.ImageStatus,
        NextVersion = flat.NextVersion,
        HasNextVersion = flat.HasNextVersion != 0,
        DeleteVersion = flat.DeleteVersion,
        NoValidImage = flat.NoValidImage != 0,
        InvalidVersion = flat.InvalidVersion != 0,
        FragIndex = flat.FragIndex,
        McGroupBitMask = flat.McGroupBitMask,
        NbFrag = flat.NbFrag,
        FragSize = flat.FragSize,
        AckReception = flat.AckReception != 0,
        FragAlgo = flat.FragAlgo,
        BlockAckDelay = flat.BlockAckDelay,
        Padding = flat.Padding,
        Descriptor = flat.Descriptor.ToArray(),
        SessionCnt = flat.SessionCnt,
        Mic = flat.Mic.ToArray(),
        Received = flat.Received,
        Missing = flat.Missing,
        MicError = flat.MicError != 0,
        MemoryError = flat.MemoryError != 0,
        NoSession = flat.NoSession != 0,
        UnsupportedAlgorithm = flat.UnsupportedAlgorithm != 0,
        UnsupportedIndex = flat.UnsupportedIndex != 0,
        WrongDescriptor = flat.WrongDescriptor != 0,
        SessionReplay = flat.SessionReplay != 0,
        AllParticipants = flat.AllParticipants != 0,
        IsStatusItem = flat.StatusItem,
        FragmentN = flat.FragmentN,
        McGroupId = flat.McGroupId,
        McAddr = flat.McAddr,
        McKeyEncrypted = flat.McKeyEncrypted.ToArray(),
        MinMcFcnt = flat.MinMcFcnt,
        MaxMcFcnt = flat.MaxMcFcnt,
        GroupMask = flat.GroupMask,
        NbTotalGroups = flat.NbTotalGroups,
        IdError = flat.IdError != 0,
        SessionTime = flat.SessionTime,
        TimeOut = flat.TimeOut,
        Periodicity = flat.Periodicity,
        DlFrequencyHz = flat.DlFrequencyHz,
        DataRate = flat.DataRate,
        TimeToStart = flat.TimeToStart,
        HasTimeToStart = flat.HasTimeToStart != 0,
        DrError = flat.DrError != 0,
        FreqError = flat.FreqError != 0,
        StartMissed = flat.StartMissed != 0,
        Data = data.ToArray(),
    };

    /// <summary>Renders this command as the record the C ABI takes.</summary>
    /// <returns>The record.</returns>
    public PamojaLorawanPackageCommand ToNative() => new()
    {
        Port = Port,
        Cid = Cid,
        Uplink = Uplink ? (byte)1 : (byte)0,
        Package = Package,
        Version = Version,
        DeviceTime = DeviceTime,
        TimeCorrection = TimeCorrection,
        Token = Token,
        AnsRequired = AnsRequired ? (byte)1 : (byte)0,
        Period = Period,
        NotSupported = NotSupported ? (byte)1 : (byte)0,
        Transmissions = Transmissions,
        Firmware = Firmware,
        Hardware = Hardware,
        Reboot = Reboot,
        ImageStatus = ImageStatus,
        NextVersion = NextVersion,
        HasNextVersion = HasNextVersion ? (byte)1 : (byte)0,
        DeleteVersion = DeleteVersion,
        NoValidImage = NoValidImage ? (byte)1 : (byte)0,
        InvalidVersion = InvalidVersion ? (byte)1 : (byte)0,
        FragIndex = FragIndex,
        McGroupBitMask = McGroupBitMask,
        NbFrag = NbFrag,
        FragSize = FragSize,
        AckReception = AckReception ? (byte)1 : (byte)0,
        FragAlgo = FragAlgo,
        BlockAckDelay = BlockAckDelay,
        Padding = Padding,
        Descriptor = Four(Descriptor, nameof(Descriptor)),
        SessionCnt = SessionCnt,
        Mic = Four(Mic, nameof(Mic)),
        Received = Received,
        Missing = Missing,
        MicError = MicError ? (byte)1 : (byte)0,
        MemoryError = MemoryError ? (byte)1 : (byte)0,
        NoSession = NoSession ? (byte)1 : (byte)0,
        UnsupportedAlgorithm = UnsupportedAlgorithm ? (byte)1 : (byte)0,
        UnsupportedIndex = UnsupportedIndex ? (byte)1 : (byte)0,
        WrongDescriptor = WrongDescriptor ? (byte)1 : (byte)0,
        SessionReplay = SessionReplay ? (byte)1 : (byte)0,
        AllParticipants = AllParticipants ? (byte)1 : (byte)0,
        StatusItem = IsStatusItem,
        FragmentN = FragmentN,
        McGroupId = McGroupId,
        McAddr = McAddr,
        McKeyEncrypted = Sixteen(McKeyEncrypted, nameof(McKeyEncrypted)),
        MinMcFcnt = MinMcFcnt,
        MaxMcFcnt = MaxMcFcnt,
        GroupMask = GroupMask,
        NbTotalGroups = NbTotalGroups,
        IdError = IdError ? (byte)1 : (byte)0,
        SessionTime = SessionTime,
        TimeOut = TimeOut,
        Periodicity = Periodicity,
        DlFrequencyHz = DlFrequencyHz,
        DataRate = DataRate,
        TimeToStart = TimeToStart,
        HasTimeToStart = HasTimeToStart ? (byte)1 : (byte)0,
        DrError = DrError ? (byte)1 : (byte)0,
        FreqError = FreqError ? (byte)1 : (byte)0,
        StartMissed = StartMissed ? (byte)1 : (byte)0,
    };

    private static PamojaFourBytes Four(byte[] bytes, string name) =>
        bytes.Length == 0 ? default : PamojaFourBytes.From(bytes, name);

    private static PamojaId Sixteen(byte[] bytes, string name) =>
        bytes.Length == 0 ? default : PamojaId.From(bytes, name);

    /// <summary>Reads one command of an application layer package.</summary>
    /// <remarks>
    /// A data fragment takes the whole message, as TS004-2.0.0 asks, and its bytes come back
    /// on <see cref="Data"/>.
    /// </remarks>
    /// <param name="port">
    /// Which package: one of <see cref="LorawanPackages.ClockPort"/>,
    /// <see cref="LorawanPackages.FragmentPort"/>,
    /// <see cref="LorawanPackages.MulticastPort"/> or
    /// <see cref="LorawanPackages.FirmwarePort"/>.
    /// </param>
    /// <param name="uplink">Whether the frame carrying it traveled up.</param>
    /// <param name="bytes">The message, from this command's identifier on.</param>
    /// <returns>The command that was read.</returns>
    /// <exception cref="PamojaException">
    /// The port names no package, the message ends inside the command, or the identifier is
    /// not one the package defines.
    /// </exception>
    public static LorawanPackageCommand Parse(byte port, bool uplink, ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_package_parse(
            port,
            uplink ? (byte)1 : (byte)0,
            bytes,
            (nuint)bytes.Length,
            out PamojaLorawanPackageCommand flat,
            out nuint taken));
        return From(flat, Carried(port, flat, bytes[..(int)taken]));
    }

    /// <summary>Reads every command in one message.</summary>
    /// <remarks>
    /// A command does not carry its own length, so one this build does not know cannot be
    /// stepped over. Reading stops there and returns what came before it.
    /// </remarks>
    /// <param name="port">Which package.</param>
    /// <param name="uplink">Whether the frame carrying them traveled up.</param>
    /// <param name="bytes">The message.</param>
    /// <returns>The commands that were readable, in order.</returns>
    /// <exception cref="PamojaException">The port names no package.</exception>
    public static IReadOnlyList<LorawanPackageCommand> ParseAll(
        byte port,
        bool uplink,
        ReadOnlySpan<byte> bytes)
    {
        List<LorawanPackageCommand> read = [];
        int at = 0;
        while (at < bytes.Length)
        {
            PamojaStatus status = NativeMethods.pamoja_lorawan_package_parse(
                port,
                uplink ? (byte)1 : (byte)0,
                bytes[at..],
                (nuint)(bytes.Length - at),
                out PamojaLorawanPackageCommand flat,
                out nuint taken);
            // A port that names no package is refused; a command that cannot be read is
            // where reading stops, because nothing says how long an unknown one is.
            if (status == PamojaStatus.InvalidArgument)
            {
                Status.ThrowIfError(status);
            }
            if (status != PamojaStatus.Ok)
            {
                break;
            }

            read.Add(From(flat, Carried(port, flat, bytes.Slice(at, (int)taken))));
            at += (int)taken;
        }
        return read;
    }

    /// <summary>Reads one group record of a multicast status answer.</summary>
    /// <remarks>
    /// The answer says how many groups follow; each is five bytes and carries no identifier of
    /// its own, so they are read one at a time rather than by <see cref="ParseAll"/>.
    /// </remarks>
    /// <param name="bytes">The message, from the record on.</param>
    /// <returns>The record.</returns>
    /// <exception cref="PamojaException">The message ends inside the record.</exception>
    public static LorawanPackageCommand StatusItem(ReadOnlySpan<byte> bytes)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_package_status_item(
            bytes,
            (nuint)bytes.Length,
            out PamojaLorawanPackageCommand flat));
        return From(flat);
    }

    /// <summary>Writes this command out.</summary>
    /// <returns>The bytes it goes out as.</returns>
    /// <exception cref="PamojaException">
    /// The port names no package, the identifier and direction name no command, or a field
    /// will not fit what carries it.
    /// </exception>
    public byte[] Encode()
    {
        PamojaLorawanPackageCommand flat = ToNative();
        byte[] buffer = new byte[MaxLength + Data.Length];
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_package_encode(
            in flat,
            Data,
            (nuint)Data.Length,
            buffer,
            (nuint)buffer.Length,
            out nuint written));
        return buffer[..(int)written];
    }

    /// <summary>The bytes a data fragment carried, which sit past its three-byte head.</summary>
    private static ReadOnlySpan<byte> Carried(
        byte port,
        PamojaLorawanPackageCommand flat,
        ReadOnlySpan<byte> command) =>
        port == LorawanPackages.FragmentPort && flat.Cid == DataFragmentCid && command.Length > 3
            ? command[3..]
            : default;

    /// <summary>The identifier a data fragment travels under, TS004-2.0.0 section 3.6.</summary>
    private const byte DataFragmentCid = 0x08;
}
