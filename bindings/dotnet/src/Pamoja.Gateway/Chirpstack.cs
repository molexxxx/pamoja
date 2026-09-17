using System.Text;

using Pamoja.Native.Interop;

namespace Pamoja.Gateway;

/// <summary>One gateway that heard an uplink.</summary>
/// <param name="Gateway">The gateway's EUI, as lowercase hex.</param>
/// <param name="RssiDbm">The received signal strength, in dBm.</param>
/// <param name="SnrDb">The signal-to-noise ratio, in dB.</param>
public sealed record ChirpstackReception(string Gateway, int RssiDbm, double SnrDb);

/// <summary>An uplink as a ChirpStack network server reports it.</summary>
/// <remarks>
/// ChirpStack publishes every uplink it deduplicates to an MQTT topic, as the JSON form of its
/// <c>UplinkEvent</c> message. Fields protobuf's JSON mapping leaves out when they hold their
/// default read as that default: a frame counter of zero, ADR off, unconfirmed.
/// </remarks>
/// <param name="DeduplicationId">The identifier ChirpStack gave the uplink once it deduplicated the gateways' copies.</param>
/// <param name="Time">When the uplink was received, as ChirpStack wrote it, or <c>null</c>.</param>
/// <param name="ApplicationId">The application the device belongs to.</param>
/// <param name="DeviceName">The name the device was given in ChirpStack.</param>
/// <param name="DevEui">The device EUI, as lowercase hex.</param>
/// <param name="DevAddr">The device's address, or <c>null</c> when the event names none.</param>
/// <param name="Adr">Whether the device had adaptive data rate on.</param>
/// <param name="DataRate">The data rate, as the region numbers them.</param>
/// <param name="Fcnt">The uplink frame counter.</param>
/// <param name="Fport">The application port, or <c>null</c> for a frame that carried none.</param>
/// <param name="Confirmed">Whether the uplink was confirmed.</param>
/// <param name="Data">The application payload, decoded from base64.</param>
/// <param name="FrequencyHz">The carrier it was heard on, in hertz, or <c>null</c>.</param>
/// <param name="Receptions">Every gateway that heard it.</param>
/// <param name="BestReception">
/// The position in <paramref name="Receptions"/> of the gateway that heard it with the highest
/// signal-to-noise ratio, or <c>null</c> when none did.
/// </param>
public sealed record ChirpstackUplinkEvent(
    string DeduplicationId,
    string? Time,
    string ApplicationId,
    string DeviceName,
    string DevEui,
    uint? DevAddr,
    bool Adr,
    byte DataRate,
    uint Fcnt,
    byte? Fport,
    bool Confirmed,
    byte[] Data,
    uint? FrequencyHz,
    IReadOnlyList<ChirpstackReception> Receptions,
    int? BestReception)
{
    /// <summary>Reads an uplink event from the JSON ChirpStack published.</summary>
    /// <param name="json">The MQTT message's payload.</param>
    /// <returns>The event.</returns>
    /// <exception cref="PamojaException">
    /// The text is not a JSON object, the event names no device EUI, or a field does not read
    /// as what it should.
    /// </exception>
    public static ChirpstackUplinkEvent FromJson(string json)
    {
        byte[] text = Encoding.UTF8.GetBytes(json);
        Status.ThrowIfError(NativeMethods.pamoja_chirpstack_uplink_parse(text, (nuint)text.Length, out IntPtr uplink));
        try
        {
            Status.ThrowIfError(NativeMethods.pamoja_chirpstack_uplink_summary(uplink, out PamojaChirpstackUplinkSummary summary));
            List<ChirpstackReception> receptions = new((int)summary.ReceptionCount);
            for (uint index = 0; index < summary.ReceptionCount; index++)
            {
                Status.ThrowIfError(NativeMethods.pamoja_chirpstack_uplink_reception(
                    uplink, index, out PamojaChirpstackReception reception));
                receptions.Add(new ChirpstackReception(
                    Hex(reception.Gateway.ToArray()),
                    reception.RssiDbm,
                    reception.SnrDb));
            }

            int? best = NativeMethods.pamoja_chirpstack_uplink_best_reception(uplink, out uint bestIndex) == PamojaStatus.Ok
                ? (int)bestIndex
                : null;
            Status.ThrowIfError(NativeMethods.pamoja_chirpstack_uplink_data(uplink, out IntPtr data));
            return new ChirpstackUplinkEvent(
                OwnedString.Read(NativeMethods.pamoja_chirpstack_uplink_text(uplink, NativeMethods.ChirpstackDeduplicationId)),
                OwnedString.ReadOrNull(NativeMethods.pamoja_chirpstack_uplink_text(uplink, NativeMethods.ChirpstackTime)),
                OwnedString.Read(NativeMethods.pamoja_chirpstack_uplink_text(uplink, NativeMethods.ChirpstackApplicationId)),
                OwnedString.Read(NativeMethods.pamoja_chirpstack_uplink_text(uplink, NativeMethods.ChirpstackDeviceName)),
                Hex(summary.DevEui.ToArray()),
                summary.HasDevAddr != 0 ? summary.DevAddr : null,
                summary.Adr != 0,
                summary.DataRate,
                summary.Fcnt,
                summary.HasFport != 0 ? summary.Fport : null,
                summary.Confirmed != 0,
                OwnedBuffer.Take(data),
                summary.HasFrequency != 0 ? summary.FrequencyHz : null,
                receptions,
                best);
        }
        finally
        {
            NativeMethods.pamoja_chirpstack_uplink_free(uplink);
        }
    }

    /// <summary>
    /// The MQTT topic every application's uplink events are published on, with wildcards in
    /// place of the application and the device.
    /// </summary>
    public const string AllApplicationsTopic = "application/+/device/+/event/up";

    /// <summary>Builds the MQTT topic an application's uplink events are published on.</summary>
    /// <param name="applicationId">The application's identifier, as ChirpStack shows it.</param>
    /// <returns>The topic, <c>application/&lt;id&gt;/device/+/event/up</c>.</returns>
    public static string UplinkTopic(string applicationId) =>
        OwnedString.Read(NativeMethods.pamoja_chirpstack_uplink_topic(applicationId));

    private static string Hex(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();
}
