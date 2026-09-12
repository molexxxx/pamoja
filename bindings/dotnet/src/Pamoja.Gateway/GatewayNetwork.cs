using Pamoja.Lora;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Gateway;

/// <summary>What a forwarded packet turned out to be.</summary>
public enum GatewayNetworkOutcome : byte
{
    /// <summary>A device joined, and its accept is ready to transmit.</summary>
    Joined = 0,

    /// <summary>A session frame arrived, decrypted.</summary>
    Data = 1,

    /// <summary>The frame belongs to a device this site never granted.</summary>
    Foreign = 2,
}

/// <summary>The channels the first receive window answers on, which the region decides.</summary>
public enum GatewayRx1Channels : byte
{
    /// <summary>The window answers on the frequency the uplink arrived on.</summary>
    SameAsUplink = 0,

    /// <summary>
    /// The window answers on a run of downlink channels, chosen by the uplink channel number
    /// modulo how many the run holds.
    /// </summary>
    Downstream = 1,
}

/// <summary>When and where a network answers, and at what rate.</summary>
/// <remarks>
/// The defaults are the values RP002-1.0.5 recommends for every region: one second to the
/// first receive window, five to the first join window, and no offset between the uplink data
/// rate and the downlink one.
/// </remarks>
public sealed record GatewayNetworkWindows
{
    /// <summary>The delay before the first receive window, in microseconds.</summary>
    public uint ReceiveDelayUs { get; init; } = 1_000_000;

    /// <summary>The delay before the window a join accept is sent in, in microseconds.</summary>
    public uint JoinDelayUs { get; init; } = 5_000_000;

    /// <summary>The offset between the uplink data rate and the rate the first window answers at.</summary>
    public byte Rx1DataRateOffset { get; init; }

    /// <summary>Which channels the first window answers on.</summary>
    public GatewayRx1Channels Rx1Channels { get; init; } = GatewayRx1Channels.SameAsUplink;

    /// <summary>The first downlink channel, in hertz, when the channels are downstream.</summary>
    public uint DownstreamStartHz { get; init; }

    /// <summary>The spacing between those channels, in hertz.</summary>
    public uint DownstreamStepHz { get; init; }

    /// <summary>How many there are.</summary>
    public ushort DownstreamCount { get; init; }
}

/// <summary>Where and when a downlink answers an uplink, in the concentrator's own terms.</summary>
/// <param name="TimestampUs">The concentrator timestamp to transmit at, in microseconds.</param>
/// <param name="FrequencyHz">The frequency to transmit on, in hertz.</param>
/// <param name="Link">The settings to transmit with.</param>
public sealed record GatewaySlot(uint TimestampUs, uint FrequencyHz, LoraLink Link);

/// <summary>What a forwarded packet turned out to be, and where its answer goes.</summary>
/// <param name="Outcome">Which of the three this was.</param>
/// <param name="DevAddr">The address granted, or the address a frame claimed.</param>
public sealed record GatewayNetworkEvent(GatewayNetworkOutcome Outcome, uint DevAddr)
{
    /// <summary>The device that joined, for a join.</summary>
    public byte[]? DevEui { get; init; }

    /// <summary>The counter the frame carried, reconstructed to its full width, for data.</summary>
    public uint? Fcnt { get; init; }

    /// <summary>The port the frame was sent on, absent for a frame carrying only options.</summary>
    public byte? Fport { get; init; }

    /// <summary>What the device sent, decrypted.</summary>
    public byte[]? Payload { get; init; }

    /// <summary>Whether the device asked to be acknowledged.</summary>
    public bool Confirmed { get; init; }

    /// <summary>Where an answer goes, for a join or for data.</summary>
    public GatewaySlot? Slot { get; init; }

    /// <summary>The packet carrying the accept, for a join.</summary>
    public GatewayTxpk? Accept { get; init; }
}

/// <summary>The network side of one site: what a server does with what a gateway forwarded.</summary>
/// <remarks>
/// A gateway forwards packets without reading them, because it holds no keys. This is the
/// other half: it holds the devices it admits, the sessions it has granted and the counters it
/// has seen, so a packet handed to it comes back as one of three things. A device joined and
/// its accept is ready to transmit, a session frame arrived and was decrypted, or the frame
/// belongs to a network this site never granted, which a gateway hears all the time and is not
/// an error.
/// </remarks>
public sealed class GatewayNetwork : IDisposable
{
    /// <summary>The largest frame this reads or writes, which is above every regional maximum.</summary>
    private const int FrameCapacity = 256;

    private readonly NativeHandle _handle;

    /// <summary>Opens the network side of a site on a channel plan.</summary>
    /// <param name="plan">The band this site operates in, which is copied into the network.</param>
    /// <param name="netId">The network identifier granted addresses carry.</param>
    /// <param name="windows">When and where to answer, or null for the recommended values.</param>
    /// <param name="firstDevAddr">The first address to grant.</param>
    /// <exception cref="PamojaException">The native core could not open a network.</exception>
    public GatewayNetwork(
        LoraChannelPlan plan,
        uint netId,
        GatewayNetworkWindows? windows = null,
        uint firstDevAddr = 1)
    {
        ArgumentNullException.ThrowIfNull(plan);

        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_network_open(
            plan.DangerousGetHandle(),
            netId,
            Native(windows ?? new GatewayNetworkWindows()),
            firstDevAddr,
            out IntPtr network));
        _handle = NativeHandle.Create(
            network, NativeMethods.pamoja_gateway_network_free, "gateway network");
    }

    /// <summary>Admits a device, so a join request signed with its key is accepted.</summary>
    /// <param name="devEui">The device identifier, eight bytes.</param>
    /// <param name="appEui">The application identifier, eight bytes.</param>
    /// <param name="appKey">The root key, sixteen bytes.</param>
    /// <exception cref="PamojaException">An identifier or the key is the wrong length.</exception>
    public void Register(ReadOnlySpan<byte> devEui, ReadOnlySpan<byte> appEui, ReadOnlySpan<byte> appKey)
    {
        IntPtr network = _handle.DangerousGetHandle();
        NativeStatus.ThrowIfError(
            NativeMethods.pamoja_gateway_network_register(network, devEui, appEui, appKey));
    }

    /// <summary>Reads a packet the gateway forwarded.</summary>
    /// <param name="heard">The packet as the gateway reported it.</param>
    /// <returns>What it turned out to be, and where its answer goes.</returns>
    /// <exception cref="PamojaException">The frame was refused, with the reason.</exception>
    public GatewayNetworkEvent Uplink(GatewayRxpk heard)
    {
        ArgumentNullException.ThrowIfNull(heard);

        byte[] buffer = new byte[FrameCapacity];
        PamojaGatewayRxpk packet = Gateway.Native(heard);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_network_uplink(
            _handle.DangerousGetHandle(),
            packet,
            heard.Payload,
            (nuint)heard.Payload.Length,
            buffer,
            (nuint)buffer.Length,
            out PamojaGatewayNetworkEvent read));

        return Managed(read, buffer);
    }

    /// <summary>Builds a downlink for a device, encrypted with its session.</summary>
    /// <param name="devAddr">The device to answer.</param>
    /// <param name="slot">Where and when to transmit, from the event that reported the uplink.</param>
    /// <param name="fport">The port to answer on.</param>
    /// <param name="payload">What to send.</param>
    /// <returns>The packet to put in a PULL_RESP.</returns>
    /// <exception cref="PamojaException">No session is held for the address.</exception>
    public GatewayTxpk Answer(uint devAddr, GatewaySlot slot, byte fport, ReadOnlySpan<byte> payload)
    {
        ArgumentNullException.ThrowIfNull(slot);

        byte[] buffer = new byte[FrameCapacity];
        NativeStatus.ThrowIfError(NativeMethods.pamoja_gateway_network_answer(
            _handle.DangerousGetHandle(),
            devAddr,
            new PamojaGatewayNetworkSlot
            {
                TimestampUs = slot.TimestampUs,
                FrequencyHz = slot.FrequencyHz,
                Link = Gateway.NativeLink(slot.Link),
            },
            fport,
            payload,
            (nuint)payload.Length,
            buffer,
            (nuint)buffer.Length,
            out PamojaGatewayTxpk downlink,
            out nuint written));

        return Gateway.Managed(downlink, buffer.AsSpan(0, (int)written).ToArray());
    }

    /// <summary>Releases the network.</summary>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads the windows into the shape the native core takes.</summary>
    private static PamojaGatewayNetworkWindows Native(GatewayNetworkWindows windows) =>
        new()
        {
            ReceiveDelayUs = windows.ReceiveDelayUs,
            JoinDelayUs = windows.JoinDelayUs,
            Rx1DataRateOffset = windows.Rx1DataRateOffset,
            Rx1Channels = (byte)windows.Rx1Channels,
            DownstreamStartHz = windows.DownstreamStartHz,
            DownstreamStepHz = windows.DownstreamStepHz,
            DownstreamCount = windows.DownstreamCount,
        };

    /// <summary>Reads an event the native core filled in.</summary>
    private static GatewayNetworkEvent Managed(PamojaGatewayNetworkEvent read, byte[] buffer)
    {
        byte[] carried = read.Truncated != 0
            ? []
            : buffer.AsSpan(0, (int)read.Len).ToArray();
        GatewayNetworkOutcome outcome = (GatewayNetworkOutcome)read.Outcome;

        return outcome switch
        {
            GatewayNetworkOutcome.Joined => new GatewayNetworkEvent(outcome, read.DevAddr)
            {
                DevEui = read.DevEui.ToArray(),
                Accept = Gateway.Managed(read.Accept, carried),
                Slot = Slot(read.Slot),
            },
            GatewayNetworkOutcome.Data => new GatewayNetworkEvent(outcome, read.DevAddr)
            {
                Fcnt = read.Fcnt,
                Fport = read.HasFport != 0 ? read.Fport : null,
                Payload = carried,
                Confirmed = read.Confirmed != 0,
                Slot = Slot(read.Slot),
            },
            _ => new GatewayNetworkEvent(outcome, read.DevAddr),
        };
    }

    /// <summary>Reads a window the native core filled in.</summary>
    private static GatewaySlot Slot(PamojaGatewayNetworkSlot slot) =>
        new(slot.TimestampUs, slot.FrequencyHz, Gateway.ManagedLink(slot.Link));
}
