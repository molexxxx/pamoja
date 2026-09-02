using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>The direction a frame travelled, which its MIC and encryption fold in.</summary>
public enum LorawanDirection
{
    /// <summary>From an end device up to the network.</summary>
    Uplink = 0,

    /// <summary>From the network down to an end device.</summary>
    Downlink = 1,
}

/// <summary>The header flags and frame options a sender sets on a data frame.</summary>
/// <remarks>
/// Every member defaults off. <see cref="FPending"/> applies to a downlink only
/// and is ignored when encoding an uplink.
/// </remarks>
public sealed class LorawanOptions
{
    /// <summary>Ask the far end to acknowledge this frame.</summary>
    public bool Confirmed { get; init; }

    /// <summary>Mark the frame as taking part in adaptive data rate.</summary>
    public bool Adr { get; init; }

    /// <summary>Acknowledge the last confirmed frame from the far end.</summary>
    public bool Ack { get; init; }

    /// <summary>Tell the device more downlink data is waiting.</summary>
    public bool FPending { get; init; }

    /// <summary>MAC commands to carry in the header, at most 15 bytes.</summary>
    public byte[] Fopts { get; init; } = [];

    /// <summary>Renders these options as the flags the C ABI takes.</summary>
    /// <returns>The flag struct.</returns>
    internal PamojaLorawanFlags ToFlags() => new()
    {
        Confirmed = Confirmed ? (byte)1 : (byte)0,
        Adr = Adr ? (byte)1 : (byte)0,
        Ack = Ack ? (byte)1 : (byte)0,
        FPending = FPending ? (byte)1 : (byte)0,
    };
}

/// <summary>A decoded LoRaWAN data frame, with its payload decrypted.</summary>
public sealed class LorawanRxData
{
    /// <summary>Creates a decoded frame from the fields the native core reported.</summary>
    /// <param name="direction">The direction the frame travelled.</param>
    /// <param name="devAddr">The device address the frame carries.</param>
    /// <param name="fcnt">The low 16 bits of the frame counter.</param>
    /// <param name="confirmed">Whether the frame asks to be acknowledged.</param>
    /// <param name="adr">Whether the frame takes part in adaptive data rate.</param>
    /// <param name="ack">Whether the frame acknowledges the last confirmed one.</param>
    /// <param name="fpending">Whether more downlink data is waiting.</param>
    /// <param name="fport">The port the frame was sent on, or <c>null</c>.</param>
    /// <param name="fopts">The MAC commands the header carried.</param>
    /// <param name="payload">The decrypted application payload.</param>
    internal LorawanRxData(
        LorawanDirection direction,
        uint devAddr,
        ushort fcnt,
        bool confirmed,
        bool adr,
        bool ack,
        bool fpending,
        byte? fport,
        byte[] fopts,
        byte[] payload)
    {
        Direction = direction;
        DevAddr = devAddr;
        Fcnt = fcnt;
        Confirmed = confirmed;
        Adr = adr;
        Ack = ack;
        FPending = fpending;
        Fport = fport;
        Fopts = fopts;
        Payload = payload;
    }

    /// <summary>The direction the frame travelled.</summary>
    public LorawanDirection Direction { get; }

    /// <summary>The device address the frame carries.</summary>
    public uint DevAddr { get; }

    /// <summary>The low 16 bits of the frame counter.</summary>
    public ushort Fcnt { get; }

    /// <summary>Whether the frame asks to be acknowledged.</summary>
    public bool Confirmed { get; }

    /// <summary>Whether the frame takes part in adaptive data rate.</summary>
    public bool Adr { get; }

    /// <summary>Whether the frame acknowledges the last confirmed one.</summary>
    public bool Ack { get; }

    /// <summary>Whether the network has more downlink data waiting.</summary>
    public bool FPending { get; }

    /// <summary>The port the frame was sent on, or <c>null</c> when it carries only options.</summary>
    public byte? Fport { get; }

    /// <summary>The MAC commands the header carried.</summary>
    public byte[] Fopts { get; }

    /// <summary>The decrypted application payload.</summary>
    public byte[] Payload { get; }
}

/// <summary>An activated LoRaWAN session: a device address and its two session keys.</summary>
/// <remarks>
/// A long-range public-band link is wide open, so LoRaWAN wraps every frame in two
/// guarantees: a message integrity code keyed to the network proves the frame is
/// authentic and intact, and the payload is encrypted to the application so only
/// its owner can read it. The keys are held natively and never come back out.
/// </remarks>
public sealed class LorawanSession : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a session from a device address and its two session keys.</summary>
    /// <param name="devAddr">The device address the network assigned.</param>
    /// <param name="nwkSKey">The 16-byte network session key, which authenticates frames.</param>
    /// <param name="appSKey">The 16-byte application session key, which encrypts payloads.</param>
    /// <exception cref="PamojaException">Either key is not 16 bytes.</exception>
    public LorawanSession(uint devAddr, ReadOnlySpan<byte> nwkSKey, ReadOnlySpan<byte> appSKey)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_session_new(
            devAddr,
            nwkSKey,
            (nuint)nwkSKey.Length,
            appSKey,
            (nuint)appSKey.Length,
            out IntPtr session));
        _handle = new NativeHandle(session, NativeMethods.pamoja_lorawan_session_free);
    }

    /// <summary>Wraps a session handle the native core produced.</summary>
    /// <param name="session">The handle, which this session takes ownership of.</param>
    internal LorawanSession(IntPtr session) =>
        _handle = new NativeHandle(session, NativeMethods.pamoja_lorawan_session_free);

    /// <summary>The device address this session is bound to.</summary>
    public uint DevAddr => _handle.Use(NativeMethods.pamoja_lorawan_session_dev_addr);

    /// <summary>Encodes an uplink, encrypting the payload and appending the MIC.</summary>
    /// <param name="fcnt">The frame counter for this uplink.</param>
    /// <param name="fport">The port; 0 for MAC commands, otherwise an application port.</param>
    /// <param name="payload">The application payload to carry.</param>
    /// <param name="options">The header flags and frame options to set.</param>
    /// <returns>The frame to transmit.</returns>
    /// <exception cref="PamojaException">
    /// The payload and options do not fit a single frame.
    /// </exception>
    public byte[] EncodeUplink(
        uint fcnt,
        byte fport,
        ReadOnlySpan<byte> payload,
        LorawanOptions? options = null)
    {
        options ??= new LorawanOptions();
        // The span cannot be captured by the lambda that holds the handle open,
        // so it is copied once here.
        byte[] body = payload.ToArray();
        byte[] fopts = options.Fopts;
        PamojaLorawanFlags flags = options.ToFlags();
        return _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_session_encode_uplink(
                handle,
                fcnt,
                fport,
                body,
                (nuint)body.Length,
                fopts,
                (nuint)fopts.Length,
                flags,
                out IntPtr frame));
            return Codec.TakeBytes(frame);
        });
    }

    /// <summary>Encodes a downlink, encrypting the payload and appending the MIC.</summary>
    /// <param name="fcnt">The frame counter for this downlink.</param>
    /// <param name="fport">The port; 0 for MAC commands, otherwise an application port.</param>
    /// <param name="payload">The application payload to carry.</param>
    /// <param name="options">The header flags and frame options to set.</param>
    /// <returns>The frame to transmit.</returns>
    /// <exception cref="PamojaException">
    /// The payload and options do not fit a single frame.
    /// </exception>
    public byte[] EncodeDownlink(
        uint fcnt,
        byte fport,
        ReadOnlySpan<byte> payload,
        LorawanOptions? options = null)
    {
        options ??= new LorawanOptions();
        byte[] body = payload.ToArray();
        byte[] fopts = options.Fopts;
        PamojaLorawanFlags flags = options.ToFlags();
        return _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_session_encode_downlink(
                handle,
                fcnt,
                fport,
                body,
                (nuint)body.Length,
                fopts,
                (nuint)fopts.Length,
                flags,
                out IntPtr frame));
            return Codec.TakeBytes(frame);
        });
    }

    /// <summary>Verifies a received frame, then decrypts it.</summary>
    /// <param name="bytes">The frame exactly as it came off the radio.</param>
    /// <param name="fcnt">
    /// The full 32-bit counter expected for this frame; its low 16 bits must match
    /// the counter the frame carries.
    /// </param>
    /// <returns>The decoded frame.</returns>
    /// <exception cref="PamojaException">
    /// The MIC does not verify, the counter does not match, or the frame is not a
    /// data frame.
    /// </exception>
    public LorawanRxData Decode(ReadOnlySpan<byte> bytes, uint fcnt)
    {
        byte[] frame = bytes.ToArray();
        return _handle.Use(handle => Decoded(handle, frame, fcnt));
    }

    /// <summary>Decodes a frame against an open session handle.</summary>
    /// <param name="session">The live session handle.</param>
    /// <param name="bytes">The frame exactly as it came off the radio.</param>
    /// <param name="fcnt">The full 32-bit counter expected for this frame.</param>
    /// <returns>The decoded frame.</returns>
    /// <exception cref="PamojaException">The frame did not verify or decode.</exception>
    private static LorawanRxData Decoded(IntPtr session, byte[] bytes, uint fcnt)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_session_decode(
            session, bytes, (nuint)bytes.Length, fcnt, out IntPtr rx));
        try
        {
            byte? fport = NativeMethods.pamoja_lorawan_rx_fport(rx, out byte port) ? port : null;
            return new LorawanRxData(
                (LorawanDirection)NativeMethods.pamoja_lorawan_rx_direction(rx),
                NativeMethods.pamoja_lorawan_rx_dev_addr(rx),
                NativeMethods.pamoja_lorawan_rx_fcnt(rx),
                NativeMethods.pamoja_lorawan_rx_confirmed(rx),
                NativeMethods.pamoja_lorawan_rx_adr(rx),
                NativeMethods.pamoja_lorawan_rx_ack(rx),
                NativeMethods.pamoja_lorawan_rx_fpending(rx),
                fport,
                Copy(
                    NativeMethods.pamoja_lorawan_rx_fopts(rx),
                    NativeMethods.pamoja_lorawan_rx_fopts_len(rx)),
                Copy(
                    NativeMethods.pamoja_lorawan_rx_payload(rx),
                    NativeMethods.pamoja_lorawan_rx_payload_len(rx)));
        }
        finally
        {
            NativeMethods.pamoja_lorawan_rx_free(rx);
        }
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

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

/// <summary>The root credentials over-the-air activation is built on.</summary>
public sealed class LorawanDevice : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a device from its two EUIs and its application key.</summary>
    /// <param name="devEui">The 8-byte device EUI.</param>
    /// <param name="appEui">The 8-byte application (join) EUI.</param>
    /// <param name="appKey">The 16-byte application key the join is secured with.</param>
    /// <exception cref="PamojaException">A credential is the wrong length.</exception>
    public LorawanDevice(
        ReadOnlySpan<byte> devEui,
        ReadOnlySpan<byte> appEui,
        ReadOnlySpan<byte> appKey)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_device_new(
            devEui,
            (nuint)devEui.Length,
            appEui,
            (nuint)appEui.Length,
            appKey,
            (nuint)appKey.Length,
            out IntPtr device));
        _handle = new NativeHandle(device, NativeMethods.pamoja_lorawan_device_free);
    }

    /// <summary>Builds the join request this device broadcasts to activate.</summary>
    /// <param name="devNonce">
    /// A nonce that must never repeat for this device, since the network rejects a
    /// replayed one.
    /// </param>
    /// <returns>The join request to transmit.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[] JoinRequest(ushort devNonce)
    {
        return _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_device_join_request(
                handle, devNonce, out IntPtr frame));
            return Codec.TakeBytes(frame);
        });
    }

    /// <summary>Turns the join accept a network sent into the settings it grants.</summary>
    /// <param name="bytes">The join accept exactly as it arrived.</param>
    /// <param name="devNonce">The nonce the matching join request carried.</param>
    /// <returns>The accepted join, which grants a session.</returns>
    /// <exception cref="PamojaException">
    /// The MIC does not verify, or the frame is not a join accept.
    /// </exception>
    public LorawanJoinAccept AcceptJoin(ReadOnlySpan<byte> bytes, ushort devNonce)
    {
        byte[] frame = bytes.ToArray();
        return _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_device_accept_join(
                handle, frame, (nuint)frame.Length, devNonce, out IntPtr accept));
            return new LorawanJoinAccept(accept);
        });
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>An accepted join: the network settings, and the session it grants.</summary>
public sealed class LorawanJoinAccept : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps an accepted join the native core produced.</summary>
    /// <param name="accept">The handle, which this object takes ownership of.</param>
    internal LorawanJoinAccept(IntPtr accept) =>
        _handle = new NativeHandle(accept, NativeMethods.pamoja_lorawan_join_accept_free);

    /// <summary>The device address the network assigned.</summary>
    public uint DevAddr => _handle.Use(NativeMethods.pamoja_lorawan_join_accept_dev_addr);

    /// <summary>The identifier of the network that accepted the join.</summary>
    public uint NetId => _handle.Use(NativeMethods.pamoja_lorawan_join_accept_net_id);

    /// <summary>
    /// The downlink settings byte, carrying the second receive window data rate and
    /// the first window offset.
    /// </summary>
    public byte DlSettings => _handle.Use(NativeMethods.pamoja_lorawan_join_accept_dl_settings);

    /// <summary>The delay before the first receive window, in seconds.</summary>
    public byte RxDelay => _handle.Use(NativeMethods.pamoja_lorawan_join_accept_rx_delay);

    /// <summary>Takes the activated session this join grants.</summary>
    /// <returns>The session, with its keys already derived.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public LorawanSession Session()
    {
        return _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_lorawan_join_accept_session(
                handle, out IntPtr session));
            return new LorawanSession(session);
        });
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
