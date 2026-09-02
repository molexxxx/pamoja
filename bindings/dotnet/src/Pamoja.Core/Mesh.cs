using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Mesh packet framing, for radios that give you no addressing of their own.</summary>
/// <remarks>
/// When the fixed infrastructure is gone or was never there, devices carry each
/// other's traffic: every node relays what it hears, so a message crosses an area
/// no single node can reach. This is the packet half of that; driving a radio is
/// the caller's job.
/// </remarks>
public static class Mesh
{
    /// <summary>The destination address that means every node.</summary>
    public const uint Broadcast = NativeMethods.MeshBroadcast;

    /// <summary>The hop limit a frame starts with unless one is given.</summary>
    public const byte DefaultHopLimit = NativeMethods.MeshDefaultHopLimit;

    /// <summary>The largest payload a single frame can carry, in bytes.</summary>
    public const int MaxPayload = NativeMethods.MeshPayloadMax;

    /// <summary>The largest frame, in bytes, including its header and checksum.</summary>
    public const int MaxFrame = NativeMethods.MeshFrameMax;

    /// <summary>Builds a frame addressed to one node.</summary>
    /// <param name="src">The address of this node.</param>
    /// <param name="dst">The address the frame is for, or <see cref="Broadcast"/>.</param>
    /// <param name="id">The sequence number identifying this packet from this source.</param>
    /// <param name="payload">The bytes to carry.</param>
    /// <param name="hopLimit">
    /// How many relays the frame may take, defaulting to <see cref="DefaultHopLimit"/>.
    /// </param>
    /// <returns>The frame, with the bytes to transmit on its <c>Bytes</c> property.</returns>
    /// <exception cref="PamojaException">
    /// The payload is larger than <see cref="MaxPayload"/>.
    /// </exception>
    public static MeshFrame Frame(
        uint src,
        uint dst,
        ushort id,
        ReadOnlySpan<byte> payload,
        byte? hopLimit = null)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_mesh_frame_new(
            src, dst, id, payload, (nuint)payload.Length, out IntPtr frame));
        return Describe(frame, hopLimit);
    }

    /// <summary>Builds a frame addressed to every node.</summary>
    /// <param name="src">The address of this node.</param>
    /// <param name="id">The sequence number identifying this packet from this source.</param>
    /// <param name="payload">The bytes to carry.</param>
    /// <param name="hopLimit">
    /// How many relays the frame may take, defaulting to <see cref="DefaultHopLimit"/>.
    /// </param>
    /// <returns>The frame, with the bytes to transmit on its <c>Bytes</c> property.</returns>
    /// <exception cref="PamojaException">
    /// The payload is larger than <see cref="MaxPayload"/>.
    /// </exception>
    public static MeshFrame BroadcastFrame(
        uint src,
        ushort id,
        ReadOnlySpan<byte> payload,
        byte? hopLimit = null)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_mesh_frame_broadcast(
            src, id, payload, (nuint)payload.Length, out IntPtr frame));
        return Describe(frame, hopLimit);
    }

    /// <summary>Parses a frame received off a radio.</summary>
    /// <param name="bytes">The frame exactly as it arrived.</param>
    /// <returns>The parsed frame.</returns>
    /// <exception cref="PamojaException">
    /// The frame is truncated, of an unknown version, or fails its checksum, which
    /// is what a noisy radio produces.
    /// </exception>
    public static MeshFrame Parse(ReadOnlySpan<byte> bytes)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_mesh_frame_parse(
            bytes, (nuint)bytes.Length, out IntPtr frame));
        return Describe(frame, null);
    }

    /// <summary>Returns the same frame with one hop spent, ready to forward.</summary>
    /// <param name="bytes">The frame exactly as it arrived.</param>
    /// <returns>
    /// The frame to forward, or <c>null</c> once its hops have run out, which is
    /// what stops a flood from circulating forever.
    /// </returns>
    /// <exception cref="PamojaException">The frame cannot be parsed.</exception>
    public static MeshFrame? Relayed(ReadOnlySpan<byte> bytes)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_mesh_frame_parse(
            bytes, (nuint)bytes.Length, out IntPtr frame));
        try
        {
            if (!NativeMethods.pamoja_mesh_frame_relayed(frame, out IntPtr forwarded))
            {
                return null;
            }

            return Describe(forwarded, null);
        }
        finally
        {
            NativeMethods.pamoja_mesh_frame_free(frame);
        }
    }

    /// <summary>Computes the CRC-16 a frame carries.</summary>
    /// <param name="data">The bytes the checksum covers.</param>
    /// <returns>The checksum.</returns>
    public static ushort Crc16(ReadOnlySpan<byte> data) =>
        NativeMethods.pamoja_mesh_crc16(data, (nuint)data.Length);

    /// <summary>Reads every field off a native frame handle and releases it.</summary>
    /// <param name="frame">The handle a native constructor produced.</param>
    /// <param name="hopLimit">A hop limit to apply first, or <c>null</c> to keep the default.</param>
    /// <returns>The frame as a value, so callers never hold a native resource.</returns>
    private static MeshFrame Describe(IntPtr frame, byte? hopLimit)
    {
        try
        {
            if (hopLimit is not null)
            {
                NativeMethods.pamoja_mesh_frame_set_hop_limit(frame, hopLimit.Value);
            }

            return new MeshFrame(
                NativeMethods.pamoja_mesh_frame_version(frame),
                NativeMethods.pamoja_mesh_frame_src(frame),
                NativeMethods.pamoja_mesh_frame_dst(frame),
                NativeMethods.pamoja_mesh_frame_id(frame),
                NativeMethods.pamoja_mesh_frame_hop_limit(frame),
                NativeMethods.pamoja_mesh_frame_is_broadcast(frame),
                Copy(
                    NativeMethods.pamoja_mesh_frame_payload(frame),
                    NativeMethods.pamoja_mesh_frame_payload_len(frame)),
                Copy(
                    NativeMethods.pamoja_mesh_frame_bytes(frame),
                    NativeMethods.pamoja_mesh_frame_bytes_len(frame)));
        }
        finally
        {
            NativeMethods.pamoja_mesh_frame_free(frame);
        }
    }

    /// <summary>Copies a borrowed native buffer into a managed array.</summary>
    /// <param name="data">The pointer the native call reported, which may be null.</param>
    /// <param name="length">Its length.</param>
    /// <returns>The bytes, empty when there are none.</returns>
    private static byte[] Copy(IntPtr data, nuint length)
    {
        int count = checked((int)length);
        byte[] bytes = new byte[count];
        if (count > 0)
        {
            Marshal.Copy(data, bytes, 0, count);
        }

        return bytes;
    }
}

/// <summary>A mesh packet: its addressing, its payload, and the bytes to transmit.</summary>
public sealed class MeshFrame
{
    /// <summary>Creates a frame from the fields the native core reported.</summary>
    /// <param name="version">The protocol version the frame declares.</param>
    /// <param name="src">The address of the node the frame came from.</param>
    /// <param name="dst">The address the frame is addressed to.</param>
    /// <param name="id">The sequence number identifying this packet from this source.</param>
    /// <param name="hopLimit">How many further relays the frame may take.</param>
    /// <param name="broadcast">Whether the frame is addressed to every node.</param>
    /// <param name="payload">The payload the frame carries.</param>
    /// <param name="bytes">The whole frame as it goes on the air.</param>
    internal MeshFrame(
        byte version,
        uint src,
        uint dst,
        ushort id,
        byte hopLimit,
        bool broadcast,
        byte[] payload,
        byte[] bytes)
    {
        Version = version;
        Src = src;
        Dst = dst;
        Id = id;
        HopLimit = hopLimit;
        Broadcast = broadcast;
        Payload = payload;
        Bytes = bytes;
    }

    /// <summary>The protocol version the frame declares.</summary>
    public byte Version { get; }

    /// <summary>The address of the node the frame came from.</summary>
    public uint Src { get; }

    /// <summary>The address the frame is addressed to.</summary>
    public uint Dst { get; }

    /// <summary>The sequence number identifying this packet from this source.</summary>
    public ushort Id { get; }

    /// <summary>How many further relays the frame may take.</summary>
    public byte HopLimit { get; }

    /// <summary>Whether the frame is addressed to every node.</summary>
    public bool Broadcast { get; }

    /// <summary>The payload the frame carries.</summary>
    public byte[] Payload { get; }

    /// <summary>The whole frame as it goes on the air.</summary>
    public byte[] Bytes { get; }
}

/// <summary>A memory of recently seen packets, so a node relays each one only once.</summary>
/// <remarks>
/// Without one, a flood multiplies without bound. The core cache is generic over
/// its size, which cannot cross the C ABI, so this one is sized when it is built.
/// </remarks>
public sealed class SeenPackets : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty cache.</summary>
    /// <param name="capacity">
    /// How many recently seen packets to remember. A capacity of 0 remembers
    /// nothing, so every copy of a packet is relayed.
    /// </param>
    /// <exception cref="PamojaException">The native cache could not be created.</exception>
    public SeenPackets(int capacity = NativeMethods.MeshSeenDefaultCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mesh_seen_new((nuint)capacity),
            NativeMethods.pamoja_mesh_seen_free,
            "duplicate cache");
    }

    /// <summary>How many packets this cache remembers.</summary>
    public int Capacity =>
        checked((int)_handle.Use(NativeMethods.pamoja_mesh_seen_capacity));

    /// <summary>Reports whether a packet is remembered, without recording it.</summary>
    /// <param name="src">The address the packet came from.</param>
    /// <param name="id">The sequence number the packet carries.</param>
    /// <returns>Whether the packet has been seen recently.</returns>
    public bool Contains(uint src, ushort id) =>
        _handle.Use(handle => NativeMethods.pamoja_mesh_seen_contains(handle, src, id));

    /// <summary>Records a packet and reports whether it was new.</summary>
    /// <param name="src">The address the packet came from.</param>
    /// <param name="id">The sequence number the packet carries.</param>
    /// <returns>
    /// <c>true</c> when the packet had not been seen, which is when a node should
    /// act on it and relay it, and <c>false</c> for a duplicate.
    /// </returns>
    public bool Record(uint src, ushort id) =>
        _handle.Use(handle => NativeMethods.pamoja_mesh_seen_record(handle, src, id));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
