using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>The split between the time a node works and the time it sleeps.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDutyCycle
{
    /// <summary>How long the node stays awake each period, in microseconds.</summary>
    public ulong ActiveUs;

    /// <summary>How long it sleeps each period, in microseconds.</summary>
    public ulong SleepUs;
}

/// <summary>What a node should be doing at the current state of charge.</summary>
public enum PamojaPowerMode
{
    /// <summary>Full duty, because the charge is healthy.</summary>
    Active = 0,

    /// <summary>Reduced duty, to conserve charge.</summary>
    Saver = 1,

    /// <summary>Minimum duty, to stay alive as long as possible.</summary>
    Critical = 2,
}

/// <summary>The work intervals a node uses in each mode, and where they change.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPowerPlan
{
    /// <summary>The interval between work at a healthy charge, in microseconds.</summary>
    public ulong ActiveUs;

    /// <summary>The interval used to conserve charge, in microseconds.</summary>
    public ulong SaverUs;

    /// <summary>The interval used at a critically low charge, in microseconds.</summary>
    public ulong CriticalUs;

    /// <summary>Enter saver mode below this state of charge.</summary>
    public float SaverBelow;

    /// <summary>Enter critical mode below this state of charge.</summary>
    public float CriticalBelow;
}

/// <summary>How urgent a telemetry event is.</summary>
public enum PamojaTelemetryLevel
{
    /// <summary>Fine-grained detail, useful only when chasing a specific problem.</summary>
    Trace = 0,

    /// <summary>Diagnostic detail for development.</summary>
    Debug = 1,

    /// <summary>A normal, noteworthy event.</summary>
    Info = 2,

    /// <summary>Something unexpected that the node recovered from.</summary>
    Warn = 3,

    /// <summary>A failure that needs attention.</summary>
    Error = 4,
}

/// <summary>What the link back to the network currently costs.</summary>
public enum PamojaLinkCost
{
    /// <summary>Bytes are effectively free, such as on wired power and ethernet.</summary>
    Free = 0,

    /// <summary>Bytes are paid for, such as on a cellular plan.</summary>
    Metered = 1,

    /// <summary>Bytes are scarce, such as on a satellite or long-range radio link.</summary>
    Expensive = 2,

    /// <summary>Nothing can be shipped at all.</summary>
    Offline = 3,
}

/// <summary>A count of everything a reporter has seen.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaTelemetrySnapshot
{
    /// <summary>How many events were seen at each level, indexed by level.</summary>
    public PamojaLevelCounts ByLevel;

    /// <summary>How many events passed the filter and were shipped.</summary>
    public uint Emitted;

    /// <summary>How many events the filter dropped.</summary>
    public uint Dropped;
}

/// <summary>The five per-level counters a snapshot carries, inline.</summary>
[System.Runtime.CompilerServices.InlineArray(Length)]
public struct PamojaLevelCounts
{
    /// <summary>The number of severity levels.</summary>
    public const int Length = 5;

    private uint _element0;
}

/// <summary>Which side of a session a device is on.</summary>
public enum PamojaSessionRole
{
    /// <summary>The device that opens the session.</summary>
    Initiator = 0,

    /// <summary>The device that answers.</summary>
    Responder = 1,
}

/// <summary>The header that travels beside a sealed message.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSealed
{
    /// <summary>The counter naming this message within the session.</summary>
    public ulong Counter;

    /// <summary>The tag over the ciphertext and its associated data.</summary>
    public PamojaTag Tag;
}

/// <summary>What a release says about itself, and what a device checks it against.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaManifest
{
    /// <summary>Which iteration of the manifest format this is.</summary>
    public byte StructureVersion;

    /// <summary>Rises with every release, which is what stops a replay.</summary>
    public ulong Sequence;

    /// <summary>Who built the image.</summary>
    public PamojaId VendorId;

    /// <summary>Which kind of device it is for.</summary>
    public PamojaId ClassId;

    /// <summary>How the payload is encoded.</summary>
    public byte Format;

    /// <summary>Which slot the payload belongs in.</summary>
    public byte Storage;

    /// <summary>The SHA-256 of the payload.</summary>
    public PamojaDigest Digest;

    /// <summary>The payload length in bytes.</summary>
    public uint Size;

    /// <summary>When the release stops being offered, or 0 to never expire.</summary>
    public ulong Expires;
}

/// <summary>A statement, signed by the anchor, that a second key may sign releases.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDelegation
{
    /// <summary>Rises with every rotation.</summary>
    public ulong Epoch;

    /// <summary>The public key that may sign manifests while this stands.</summary>
    public PamojaDigest ReleaseKey;

    /// <summary>When the delegation stops being honoured, or 0 to never expire.</summary>
    public ulong Expires;
}

/// <summary>Who a device is, and whose signature it trusts.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaDevice
{
    /// <summary>Who built this firmware.</summary>
    public PamojaId VendorId;

    /// <summary>What kind of device this is.</summary>
    public PamojaId ClassId;

    /// <summary>The key this device anchors its trust in.</summary>
    public PamojaDigest Anchor;
}

/// <summary>What a device believes about one slot.</summary>
public enum PamojaSlotState
{
    /// <summary>Nothing has been written here.</summary>
    Empty = 0,

    /// <summary>An image is arriving.</summary>
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

/// <summary>The record a device keeps about one slot.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSlotRecord
{
    /// <summary>The state of the slot.</summary>
    public PamojaSlotState State;

    /// <summary>The sequence number of the image in the slot.</summary>
    public ulong Sequence;

    /// <summary>The length of the image in bytes.</summary>
    public uint Size;

    /// <summary>The digest of the image.</summary>
    public PamojaDigest Digest;

    /// <summary>How many bytes have been stored.</summary>
    public uint Written;
}

/// <summary>What a bootloader should do with what it found.</summary>
public enum PamojaBootAction
{
    /// <summary>Nothing new to try; run the confirmed image.</summary>
    Confirmed = 0,

    /// <summary>A staged image is being tried for the first time.</summary>
    Trying = 1,

    /// <summary>A pending image never confirmed, so it was failed.</summary>
    Reverted = 2,
}

/// <summary>The decision a device made at boot.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaBoot
{
    /// <summary>What the bootloader should do.</summary>
    public PamojaBootAction Action;

    /// <summary>The image the decision is about.</summary>
    public byte Slot;

    /// <summary>The slot to run.</summary>
    public byte Fallback;
}
