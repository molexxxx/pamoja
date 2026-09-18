using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// One command of a LoRaWAN application layer package, mirroring
/// <c>PamojaLorawanPackageCommand</c> in <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// <see cref="Port"/> says which package and <see cref="Cid"/> names the command within
/// it; together with <see cref="Uplink"/> they decide which of the other fields carry
/// anything. The rest read as zero.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaLorawanPackageCommand
{
    /// <summary>Which package this command belongs to, as its port.</summary>
    public byte Port;

    /// <summary>Which command within that package.</summary>
    public byte Cid;

    /// <summary>Which way it travels, <c>1</c> for yes.</summary>
    public byte Uplink;

    /// <summary>The package identifier a version answer carries.</summary>
    public byte Package;

    /// <summary>The package version it implements.</summary>
    public byte Version;

    /// <summary>A device's own clock, in seconds since the GPS epoch.</summary>
    public uint DeviceTime;

    /// <summary>The seconds to add to a device's clock.</summary>
    public int TimeCorrection;

    /// <summary>The token that pairs a clock answer with its request.</summary>
    public byte Token;

    /// <summary>Whether a clock request must be answered, <c>1</c> for yes.</summary>
    public byte AnsRequired;

    /// <summary>The coded period between clock requests.</summary>
    public byte Period;

    /// <summary>Whether a device manages its own clock periodicity, <c>1</c> for yes.</summary>
    public byte NotSupported;

    /// <summary>How many requests a resynchronization command asks for.</summary>
    public byte Transmissions;

    /// <summary>The firmware a device reports running.</summary>
    public uint Firmware;

    /// <summary>The hardware it runs on.</summary>
    public uint Hardware;

    /// <summary>The moment or the delay a reboot is set for.</summary>
    public uint Reboot;

    /// <summary>What a device makes of the upgrade image it holds.</summary>
    public byte ImageStatus;

    /// <summary>The version it would run once that image is installed.</summary>
    public uint NextVersion;

    /// <summary>Whether a version rides with the image status, <c>1</c> for yes.</summary>
    public byte HasNextVersion;

    /// <summary>The version a delete command names.</summary>
    public uint DeleteVersion;

    /// <summary>Whether a device holds no valid image, <c>1</c> for yes.</summary>
    public byte NoValidImage;

    /// <summary>Whether the version named is not the one held, <c>1</c> for yes.</summary>
    public byte InvalidVersion;

    /// <summary>Which fragmentation session, 0 to 3.</summary>
    public byte FragIndex;

    /// <summary>Which multicast groups may feed it, a bit for each.</summary>
    public byte McGroupBitMask;

    /// <summary>How many uncoded fragments a block was cut into.</summary>
    public ushort NbFrag;

    /// <summary>How many bytes each fragment carries.</summary>
    public byte FragSize;

    /// <summary>Whether a device reports the block once it has it, <c>1</c> for yes.</summary>
    public byte AckReception;

    /// <summary>Which fragmentation algorithm to run.</summary>
    public byte FragAlgo;

    /// <summary>The coded spread of the delay before a device answers.</summary>
    public byte BlockAckDelay;

    /// <summary>How many bytes of padding the last fragment carries.</summary>
    public byte Padding;

    /// <summary>The four bytes a server describes a block with.</summary>
    public PamojaFourBytes Descriptor;

    /// <summary>The session counter, which must rise for each new block.</summary>
    public ushort SessionCnt;

    /// <summary>The code over the block a device checks once it has it all.</summary>
    public PamojaFourBytes Mic;

    /// <summary>How many fragments arrived, coded, uncoded and repeated.</summary>
    public ushort Received;

    /// <summary>How many uncoded fragments are still missing.</summary>
    public byte Missing;

    /// <summary>Whether the block's code did not check out, <c>1</c> for yes.</summary>
    public byte MicError;

    /// <summary>Whether a session ran out of memory to defragment with, <c>1</c> for yes.</summary>
    public byte MemoryError;

    /// <summary>Whether the session or group named does not exist on the device, <c>1</c> for yes.</summary>
    public byte NoSession;

    /// <summary>Whether the setup named an algorithm the device does not run, <c>1</c> for yes.</summary>
    public byte UnsupportedAlgorithm;

    /// <summary>Whether the setup named a session index the device does not keep, <c>1</c> for yes.</summary>
    public byte UnsupportedIndex;

    /// <summary>Whether the descriptor is not one the device accepts, <c>1</c> for yes.</summary>
    public byte WrongDescriptor;

    /// <summary>Whether the session counter repeats one already used, <c>1</c> for yes.</summary>
    public byte SessionReplay;

    /// <summary>Whether every device answers a status request, <c>1</c> for yes.</summary>
    public byte AllParticipants;

    /// <summary>Whether this is one group record of a multicast status answer.</summary>
    public byte StatusItem;

    /// <summary>Which fragment of a session a data fragment carries, counting from one.</summary>
    public ushort FragmentN;

    /// <summary>Which multicast group, 0 to 3.</summary>
    public byte McGroupId;

    /// <summary>The address a group answers to.</summary>
    public uint McAddr;

    /// <summary>A group's key, wrapped under the device's key encryption key.</summary>
    public PamojaId McKeyEncrypted;

    /// <summary>The first frame counter a device accepts from a group.</summary>
    public uint MinMcFcnt;

    /// <summary>The last one, which ends the group's life.</summary>
    public uint MaxMcFcnt;

    /// <summary>Which groups a status request or answer covers, a bit for each.</summary>
    public byte GroupMask;

    /// <summary>How many groups a device holds in all.</summary>
    public byte NbTotalGroups;

    /// <summary>Whether a device holds no group by the identifier named, <c>1</c> for yes.</summary>
    public byte IdError;

    /// <summary>When a multicast window opens, in seconds since the GPS epoch.</summary>
    public uint SessionTime;

    /// <summary>How long it lasts at most, coded.</summary>
    public byte TimeOut;

    /// <summary>How often a device opens a ping slot inside a Class B window.</summary>
    public byte Periodicity;

    /// <summary>Where a group listens, in hertz.</summary>
    public uint DlFrequencyHz;

    /// <summary>The data rate it listens at.</summary>
    public byte DataRate;

    /// <summary>How many seconds until a window opens.</summary>
    public uint TimeToStart;

    /// <summary>Whether a delay rides with the answer, <c>1</c> for yes.</summary>
    public byte HasTimeToStart;

    /// <summary>Whether the data rate named is not one the device has, <c>1</c> for yes.</summary>
    public byte DrError;

    /// <summary>Whether the frequency named is not one it can use, <c>1</c> for yes.</summary>
    public byte FreqError;

    /// <summary>Whether the window was to start at a time already past, <c>1</c> for yes.</summary>
    public byte StartMissed;
}
