using Pamoja.Lora;
using Pamoja.Native;
using Pamoja.Native.Interop;

namespace Pamoja.Lorawan;

/// <summary>Where a frame goes and how fast.</summary>
/// <param name="FrequencyHz">The frequency in hertz.</param>
/// <param name="DataRate">The data rate.</param>
public readonly record struct LorawanCarrier(uint FrequencyHz, byte DataRate);

/// <summary>The integrity and encryption keys of one end device's wake-on-radio frames.</summary>
/// <param name="Integrity"><c>WorSIntKey</c>, 16 bytes.</param>
/// <param name="Encryption"><c>WorSEncKey</c>, 16 bytes.</param>
public sealed record LorawanWorKeys(byte[] Integrity, byte[] Encryption)
{
    /// <summary>Compares the key bytes.</summary>
    /// <param name="other">The other keys.</param>
    /// <returns>Whether both keys hold the same bytes.</returns>
    public bool Equals(LorawanWorKeys? other) =>
        other is not null
        && Integrity.AsSpan().SequenceEqual(other.Integrity)
        && Encryption.AsSpan().SequenceEqual(other.Encryption);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(Integrity.Length, Encryption.Length);

    /// <inheritdoc/>
    public override string ToString() => "LorawanWorKeys { .. }";
}

/// <summary>Which WOR frame a relay heard.</summary>
public enum LorawanWorKind
{
    /// <summary>Ahead of a join request, which nothing protects.</summary>
    JoinRequest = 0,

    /// <summary>Ahead of a Class A uplink, sealed with the device's WOR keys.</summary>
    Uplink = 1,
}

/// <summary>A wake-on-radio frame, as a relay reads it.</summary>
/// <param name="Kind">Which frame it is.</param>
/// <param name="Uplink">Where and how fast a join request follows; null for an uplink, whose carrier is sealed.</param>
/// <param name="DevAddr">The address an uplink WOR names.</param>
/// <param name="Wfcnt">The low sixteen bits of an uplink WOR's counter.</param>
public sealed record LorawanWor(LorawanWorKind Kind, LorawanCarrier? Uplink, uint? DevAddr, ushort? Wfcnt);

/// <summary>How often a relay scans a channel for a WOR preamble, TS011-1.0.1 table 18.</summary>
public enum LorawanCadPeriodicity
{
    /// <summary>Once a second, the default.</summary>
    Ms1000 = 0,

    /// <summary>Every 500 milliseconds.</summary>
    Ms500 = 1,

    /// <summary>Every 250 milliseconds.</summary>
    Ms250 = 2,

    /// <summary>Every 100 milliseconds.</summary>
    Ms100 = 3,

    /// <summary>Every 50 milliseconds.</summary>
    Ms50 = 4,

    /// <summary>Every 20 milliseconds.</summary>
    Ms20 = 5,
}

/// <summary>How many symbols a relay takes from detecting activity to receiving, table 15.</summary>
public enum LorawanCadToRx
{
    /// <summary>Two symbols.</summary>
    Symbols2 = 0,

    /// <summary>Four symbols.</summary>
    Symbols4 = 1,

    /// <summary>Six symbols.</summary>
    Symbols6 = 2,

    /// <summary>Eight symbols, which a device assumes before it has heard from a relay.</summary>
    Symbols8 = 3,
}

/// <summary>How accurate a relay's crystal is, table 17.</summary>
public enum LorawanXtalAccuracy
{
    /// <summary>Better than 10 parts per million.</summary>
    Ppm10 = 0,

    /// <summary>Better than 20.</summary>
    Ppm20 = 1,

    /// <summary>Better than 30.</summary>
    Ppm30 = 2,

    /// <summary>Better than 40, which a device assumes before it has heard from a relay.</summary>
    Ppm40 = 3,
}

/// <summary>Whether a relay will forward the uplink after a WOR frame, table 16.</summary>
public enum LorawanRelayForward
{
    /// <summary>It has room to.</summary>
    Available = 0,

    /// <summary>A forwarding limit is reached; try again in 30 minutes.</summary>
    RetryIn30Minutes = 1,

    /// <summary>A forwarding limit is reached; try again in 60 minutes.</summary>
    RetryIn60Minutes = 2,

    /// <summary>Forwarding is off.</summary>
    Disabled = 3,
}

/// <summary>Which of a relay's channels a WOR frame arrived on, table 28.</summary>
public enum LorawanWorChannel
{
    /// <summary>The default channel.</summary>
    Default = 0,

    /// <summary>The second channel a network configured.</summary>
    Second = 1,
}

/// <summary>What a relay tells an end device about itself in a WOR ACK, table 14.</summary>
/// <param name="CadToRx">How long it takes to start receiving.</param>
/// <param name="Forward">Whether it forwards.</param>
/// <param name="RelayDataRate">The data rate it forwards at.</param>
/// <param name="XtalAccuracy">How accurate its crystal is.</param>
/// <param name="CadPeriodicity">How often it scans.</param>
/// <param name="TOffsetMs">Milliseconds from the start of the scan to the end of the WOR preamble.</param>
public readonly record struct LorawanStateSync(
    LorawanCadToRx CadToRx,
    LorawanRelayForward Forward,
    byte RelayDataRate,
    LorawanXtalAccuracy XtalAccuracy,
    LorawanCadPeriodicity CadPeriodicity,
    ushort TOffsetMs);

/// <summary>What a relay heard of an uplink it forwards.</summary>
/// <param name="WorChannel">The channel the WOR frame came in on.</param>
/// <param name="RssiDbm">The uplink's signal strength in dBm, carried from -142 to -15.</param>
/// <param name="SnrDb">Its signal-to-noise ratio in dB, carried from -20 to 11.</param>
/// <param name="DataRate">The data rate it arrived at.</param>
public readonly record struct LorawanUplinkMetadata(
    LorawanWorChannel WorChannel,
    short RssiDbm,
    sbyte SnrDb,
    byte DataRate);

/// <summary>An end device's uplink as a relay forwards it on port 226.</summary>
/// <param name="Metadata">What the relay heard of it.</param>
/// <param name="FrequencyHz">The frequency it arrived on, in hertz.</param>
/// <param name="PhyPayload">The end device's frame.</param>
public sealed record LorawanForwardedUplink(LorawanUplinkMetadata Metadata, uint FrequencyHz, byte[] PhyPayload);

/// <summary>What an end device knows of a relay's scans once a WOR ACK has arrived.</summary>
/// <param name="ReferenceMicros">When the relay scanned, in the device's microseconds.</param>
/// <param name="CadPeriodicity">How often it scans.</param>
/// <param name="RelayXtal">How accurate its crystal is.</param>
/// <param name="CadToRx">How long it takes to start receiving.</param>
public readonly record struct LorawanSynchronization(
    ulong ReferenceMicros,
    LorawanCadPeriodicity CadPeriodicity,
    LorawanXtalAccuracy RelayXtal,
    LorawanCadToRx CadToRx);

/// <summary>When a synchronized end device's next WOR frame goes out.</summary>
/// <param name="StartMicros">When to start sending, in microseconds.</param>
/// <param name="PreambleSymbols">The preamble length in symbols.</param>
public readonly record struct LorawanWorSlot(ulong StartMicros, ushort PreambleSymbols);

/// <summary>A LoRaWAN relay, TS011-1.0.1: what an end device and a relay say to each other.</summary>
/// <remarks>
/// A relay sleeps, waking every scan period to look for radio activity. An end device out of a
/// gateway's reach first sends a wake-on-radio (WOR) frame whose preamble spans that sleep and
/// which says where its uplink follows. The relay may acknowledge with its own timing, so the
/// next preamble can be short, then forwards the uplink on port <see cref="FPort"/>.
/// </remarks>
public static class LorawanRelay
{
    /// <summary>The port every message between a relay and its network uses.</summary>
    public const byte FPort = 226;

    /// <summary>How many end devices a relay verifies wake-on-radio frames for.</summary>
    public const int TrustedEndDevices = 16;

    /// <summary>How many WOR frames go without an acknowledgment before the uplink goes anyway.</summary>
    public const byte WorAttemptsWithoutAck = 8;

    /// <summary>The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it.</summary>
    public const uint WorDataDelayMicros = 50_000;

    /// <summary>The gap between a WOR frame and its acknowledgment.</summary>
    public const uint WorAckDelayMicros = 50_000;

    /// <summary>The gap between a relay hearing an uplink and forwarding it.</summary>
    public const uint RelayForwardDelayMicros = 50_000;

    /// <summary>How long after an uplink an end device's RXR window opens at the latest.</summary>
    public const uint RxrDelayMicros = 18_000_000;

    /// <summary>The bytes a forwarded uplink adds in front of the end device's frame.</summary>
    public const int ForwardOverhead = 6;

    /// <summary>The shortest WOR preamble, in symbols.</summary>
    public const ushort MinWorPreambleSymbols = 8;

    /// <summary>Derives an end device's root relay session key from its network session key, section 4.4.</summary>
    /// <param name="networkKey">The 16-byte network session key.</param>
    /// <returns>The 16-byte root key.</returns>
    /// <exception cref="PamojaException">The key is not 16 bytes.</exception>
    public static byte[] RootWorSKey(ReadOnlySpan<byte> networkKey)
    {
        byte[] root = new byte[16];
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_relay_root_wor_s_key(networkKey, (nuint)networkKey.Length, root));
        return root;
    }

    /// <summary>Derives an end device's WOR keys from its root relay session key, section 4.5.</summary>
    /// <param name="rootKey">The 16-byte root key.</param>
    /// <param name="devAddr">The device's address.</param>
    /// <returns>The integrity and encryption keys.</returns>
    /// <exception cref="PamojaException">The key is not 16 bytes.</exception>
    public static LorawanWorKeys WorKeys(ReadOnlySpan<byte> rootKey, uint devAddr)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lorawan_relay_wor_keys(rootKey, (nuint)rootKey.Length, devAddr, out PamojaLorawanWorKeys keys));
        return KeysOut(keys);
    }

    /// <summary>Builds the WOR frame ahead of a join request, section 5.3.1.</summary>
    /// <param name="uplink">Where and how fast the join request follows.</param>
    /// <returns>The five-byte frame.</returns>
    /// <exception cref="PamojaException">A data rate past 15 or a frequency the field cannot carry.</exception>
    public static byte[] WorJoinRequest(LorawanCarrier uplink)
    {
        byte[] frame = new byte[5];
        PamojaLorawanCarrier native = CarrierIn(uplink);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_join_request(in native, frame));
        return frame;
    }

    /// <summary>Builds the WOR frame ahead of a Class A uplink, section 5.3.2.</summary>
    /// <param name="keys">The device's WOR keys.</param>
    /// <param name="devAddr">Its address.</param>
    /// <param name="wfcnt">The WOR frame counter, raised for every WOR frame.</param>
    /// <param name="uplink">Where and how fast the uplink follows.</param>
    /// <param name="wor">The carrier this WOR frame goes out on.</param>
    /// <returns>The fifteen-byte frame.</returns>
    /// <exception cref="PamojaException">A carrier the fields cannot carry.</exception>
    public static byte[] WorUplink(LorawanWorKeys keys, uint devAddr, uint wfcnt, LorawanCarrier uplink, LorawanCarrier wor)
    {
        byte[] frame = new byte[15];
        PamojaLorawanWorKeys nativeKeys = KeysIn(keys);
        PamojaLorawanCarrier nativeUplink = CarrierIn(uplink);
        PamojaLorawanCarrier nativeWor = CarrierIn(wor);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_uplink(
            in nativeKeys, devAddr, wfcnt, in nativeUplink, in nativeWor, frame));
        return frame;
    }

    /// <summary>Reads a WOR frame, leaving an uplink's carrier sealed.</summary>
    /// <param name="frame">The bytes a relay received.</param>
    /// <returns>The frame.</returns>
    /// <exception cref="PamojaException">A reserved or proprietary type, or a length that is not the type's.</exception>
    public static LorawanWor ParseWor(ReadOnlySpan<byte> frame)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_parse(frame, (nuint)frame.Length, out PamojaLorawanWor wor));
        return wor.Kind == NativeMethods.LorawanWorJoinRequest
            ? new LorawanWor(LorawanWorKind.JoinRequest, CarrierOut(wor.Uplink), null, null)
            : new LorawanWor(LorawanWorKind.Uplink, null, wor.DevAddr, wor.Wfcnt);
    }

    /// <summary>Checks a WOR frame ahead of a Class A uplink and reads where the uplink follows.</summary>
    /// <param name="frame">The frame a relay received.</param>
    /// <param name="keys">The keys of the device it names.</param>
    /// <param name="wfcnt">The full 32-bit counter the relay takes it to carry.</param>
    /// <param name="wor">The carrier it arrived on.</param>
    /// <returns>Where and how fast the uplink follows.</returns>
    /// <exception cref="PamojaException">The frame is not an uplink WOR, or its integrity code does not verify.</exception>
    public static LorawanCarrier OpenWor(ReadOnlySpan<byte> frame, LorawanWorKeys keys, uint wfcnt, LorawanCarrier wor)
    {
        PamojaLorawanWorKeys nativeKeys = KeysIn(keys);
        PamojaLorawanCarrier nativeWor = CarrierIn(wor);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_open(
            frame, (nuint)frame.Length, in nativeKeys, wfcnt, in nativeWor, out PamojaLorawanCarrier uplink));
        return CarrierOut(uplink);
    }

    /// <summary>Builds a relay's WOR ACK, section 6.2.</summary>
    /// <param name="keys">The end device's WOR keys.</param>
    /// <param name="devAddr">Its address.</param>
    /// <param name="wfcnt">The counter of the acknowledged WOR frame.</param>
    /// <param name="ack">The carrier the acknowledgment goes out on.</param>
    /// <param name="uplink">The carrier the WOR frame named for the uplink.</param>
    /// <param name="state">What the relay tells the device.</param>
    /// <returns>The seven-byte acknowledgment.</returns>
    /// <exception cref="PamojaException">A state or carrier the fields cannot carry.</exception>
    public static byte[] WorAck(
        LorawanWorKeys keys,
        uint devAddr,
        uint wfcnt,
        LorawanCarrier ack,
        LorawanCarrier uplink,
        LorawanStateSync state)
    {
        byte[] frame = new byte[7];
        PamojaLorawanWorKeys nativeKeys = KeysIn(keys);
        PamojaLorawanCarrier nativeAck = CarrierIn(ack);
        PamojaLorawanCarrier nativeUplink = CarrierIn(uplink);
        PamojaLorawanStateSync nativeState = StateIn(state);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_ack(
            in nativeKeys, devAddr, wfcnt, in nativeAck, in nativeUplink, in nativeState, frame));
        return frame;
    }

    /// <summary>Checks and reads a WOR ACK, section 6.2.</summary>
    /// <param name="frame">The acknowledgment an end device received.</param>
    /// <param name="keys">The device's WOR keys.</param>
    /// <param name="devAddr">Its address.</param>
    /// <param name="wfcnt">The counter of the WOR frame it sent.</param>
    /// <param name="ack">The carrier the acknowledgment arrived on.</param>
    /// <param name="uplink">The carrier the WOR frame named.</param>
    /// <returns>What the relay said about itself.</returns>
    /// <exception cref="PamojaException">The integrity code does not verify, or the periodicity is reserved.</exception>
    public static LorawanStateSync OpenWorAck(
        ReadOnlySpan<byte> frame,
        LorawanWorKeys keys,
        uint devAddr,
        uint wfcnt,
        LorawanCarrier ack,
        LorawanCarrier uplink)
    {
        PamojaLorawanWorKeys nativeKeys = KeysIn(keys);
        PamojaLorawanCarrier nativeAck = CarrierIn(ack);
        PamojaLorawanCarrier nativeUplink = CarrierIn(uplink);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_wor_ack_open(
            frame, (nuint)frame.Length, in nativeKeys, devAddr, wfcnt, in nativeAck, in nativeUplink, out PamojaLorawanStateSync state));
        return StateOut(state);
    }

    /// <summary>Writes an uplink a relay forwards on port 226, section 9.1.</summary>
    /// <param name="forwarded">The metadata, the frequency, and the end device's frame.</param>
    /// <returns>The relay uplink's payload.</returns>
    /// <exception cref="PamojaException">A data rate or frequency the fields cannot carry.</exception>
    /// <remarks>A strength or ratio past what its field carries goes out as the closest value.</remarks>
    public static byte[] EncodeForward(LorawanForwardedUplink forwarded)
    {
        PamojaLorawanUplinkMetadata metadata = new()
        {
            WorChannel = (byte)forwarded.Metadata.WorChannel,
            RssiDbm = forwarded.Metadata.RssiDbm,
            SnrDb = forwarded.Metadata.SnrDb,
            DataRate = forwarded.Metadata.DataRate,
        };
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_forward_encode(
            in metadata,
            forwarded.FrequencyHz,
            forwarded.PhyPayload,
            (nuint)forwarded.PhyPayload.Length,
            out IntPtr payload));
        return Pamoja.Codec.Codec.TakeBytes(payload);
    }

    /// <summary>Reads an uplink a relay forwarded, section 9.1.</summary>
    /// <param name="payload">The relay uplink's payload on port 226.</param>
    /// <returns>The metadata, the frequency, and the end device's frame.</returns>
    /// <exception cref="PamojaException">A payload too short or a reserved WOR channel.</exception>
    public static LorawanForwardedUplink ParseForward(ReadOnlySpan<byte> payload)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_forward_parse(
            payload,
            (nuint)payload.Length,
            out PamojaLorawanUplinkMetadata metadata,
            out uint frequencyHz,
            out IntPtr phyPayload));
        return new LorawanForwardedUplink(
            new LorawanUplinkMetadata(
                (LorawanWorChannel)metadata.WorChannel,
                metadata.RssiDbm,
                metadata.SnrDb,
                metadata.DataRate),
            frequencyHz,
            Pamoja.Codec.Codec.TakeBytes(phyPayload));
    }

    /// <summary>The WOR preamble of an end device that does not know when the relay scans, section 5.2.</summary>
    /// <param name="cadPeriodicity">How often the relay scans.</param>
    /// <param name="symbolMicros">The symbol time of the WOR frame's data rate, in microseconds.</param>
    /// <param name="cadToRx">The relay's time to start receiving.</param>
    /// <returns>The preamble length in symbols.</returns>
    public static ushort UnsynchronizedPreamble(LorawanCadPeriodicity cadPeriodicity, ulong symbolMicros, LorawanCadToRx cadToRx)
    {
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_unsynchronized_preamble(
            (byte)cadPeriodicity, symbolMicros, (byte)cadToRx, out ushort symbols));
        return symbols;
    }

    /// <summary>The offset a relay reports in a WOR ACK, appendix 1.</summary>
    /// <param name="scanStartMicros">When the scan that detected the frame started.</param>
    /// <param name="preambleEndMicros">
    /// When the frame's preamble ended: when it finished arriving, less the airtime of its sync word and payload.
    /// </param>
    /// <returns>
    /// The offset in milliseconds, or null when the preamble ended before the scan or more than eleven bits of
    /// milliseconds after it.
    /// </returns>
    public static ushort? TOffsetMs(ulong scanStartMicros, ulong preambleEndMicros) =>
        NativeMethods.pamoja_lorawan_relay_t_offset_ms(
            scanStartMicros, preambleEndMicros, out ushort offset) == PamojaStatus.Ok
            ? offset
            : null;

    /// <summary>Works out when a relay scanned from the WOR ACK that answered a frame, appendix 1.</summary>
    /// <param name="worStartMicros">When the acknowledged WOR frame started going out.</param>
    /// <param name="preambleSymbols">Its preamble length.</param>
    /// <param name="symbolMicros">Its symbol time.</param>
    /// <param name="state">What the acknowledgment said.</param>
    /// <returns>The synchronization.</returns>
    public static LorawanSynchronization Synchronization(
        ulong worStartMicros,
        ushort preambleSymbols,
        ulong symbolMicros,
        LorawanStateSync state)
    {
        PamojaLorawanStateSync nativeState = StateIn(state);
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_synchronization(
            worStartMicros, preambleSymbols, symbolMicros, in nativeState, out PamojaLorawanSynchronization sync));
        return new LorawanSynchronization(
            sync.ReferenceUs,
            (LorawanCadPeriodicity)sync.CadPeriodicity,
            (LorawanXtalAccuracy)sync.RelayXtal,
            (LorawanCadToRx)sync.CadToRx);
    }

    /// <summary>Picks the relay scan a synchronized end device aims its next WOR frame at, appendix 1.</summary>
    /// <param name="synchronization">What the device knows of the relay.</param>
    /// <param name="nowMicros">The time.</param>
    /// <param name="deviceXtalPpm">The device's crystal accuracy.</param>
    /// <param name="symbolMicros">The symbol time of the WOR frame's data rate.</param>
    /// <param name="otherChannel">Whether the frame goes out on the relay's other channel.</param>
    /// <returns>When to send and how long a preamble, or null once the drift exceeds a period.</returns>
    public static LorawanWorSlot? NextWor(
        LorawanSynchronization synchronization,
        ulong nowMicros,
        uint deviceXtalPpm,
        ulong symbolMicros,
        bool otherChannel = false)
    {
        PamojaLorawanSynchronization native = new()
        {
            ReferenceUs = synchronization.ReferenceMicros,
            CadPeriodicity = (byte)synchronization.CadPeriodicity,
            RelayXtal = (byte)synchronization.RelayXtal,
            CadToRx = (byte)synchronization.CadToRx,
        };
        Status.ThrowIfError(NativeMethods.pamoja_lorawan_relay_next_wor(
            in native,
            nowMicros,
            deviceXtalPpm,
            symbolMicros,
            otherChannel ? (byte)1 : (byte)0,
            out PamojaLorawanWorSlot slot,
            out byte synchronized));
        return synchronized != 0 ? new LorawanWorSlot(slot.StartUs, slot.PreambleSymbols) : null;
    }

    /// <summary>Reads the second channel a relay or end device configuration describes.</summary>
    /// <param name="secondChannelIndex">The coded index, 1 for a second channel.</param>
    /// <param name="dataRate">Its data rate.</param>
    /// <param name="ackOffset">The coded acknowledgment offset, table 35.</param>
    /// <param name="frequencyHz">Its frequency.</param>
    /// <returns>The channel with its acknowledgment frequency, or null when none is named.</returns>
    public static LoraRelayChannel? SecondChannel(byte secondChannelIndex, byte dataRate, byte ackOffset, uint frequencyHz) =>
        NativeMethods.pamoja_lorawan_relay_second_channel(
            secondChannelIndex, dataRate, ackOffset, frequencyHz, out PamojaLoraRelayChannel channel) == PamojaStatus.Ok
            ? new LoraRelayChannel(channel.WorFrequencyHz, channel.AckFrequencyHz, channel.DataRate)
            : null;

    internal static LorawanWorKeys KeysOut(PamojaLorawanWorKeys keys) =>
        new(keys.Integrity.ToArray(), keys.Encryption.ToArray());

    private static PamojaLorawanWorKeys KeysIn(LorawanWorKeys keys) => new()
    {
        Integrity = PamojaId.From(keys.Integrity, nameof(keys.Integrity)),
        Encryption = PamojaId.From(keys.Encryption, nameof(keys.Encryption)),
    };

    private static PamojaLorawanCarrier CarrierIn(LorawanCarrier carrier) => new()
    {
        FrequencyHz = carrier.FrequencyHz,
        DataRate = carrier.DataRate,
    };

    private static LorawanCarrier CarrierOut(PamojaLorawanCarrier carrier) => new(carrier.FrequencyHz, carrier.DataRate);

    private static PamojaLorawanStateSync StateIn(LorawanStateSync state) => new()
    {
        CadToRx = (byte)state.CadToRx,
        Forward = (byte)state.Forward,
        RelayDataRate = state.RelayDataRate,
        XtalAccuracy = (byte)state.XtalAccuracy,
        CadPeriodicity = (byte)state.CadPeriodicity,
        TOffsetMs = state.TOffsetMs,
    };

    private static LorawanStateSync StateOut(PamojaLorawanStateSync state) => new(
        (LorawanCadToRx)state.CadToRx,
        (LorawanRelayForward)state.Forward,
        state.RelayDataRate,
        (LorawanXtalAccuracy)state.XtalAccuracy,
        (LorawanCadPeriodicity)state.CadPeriodicity,
        state.TOffsetMs);
}
