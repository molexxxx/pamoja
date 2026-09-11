using Pamoja.Native.Interop;

namespace Pamoja.Lora;

/// <summary>
/// The gains and losses of a LoRa link, from the transmitting radio to the receiving one.
/// </summary>
/// <remarks>
/// The properties are the parts of a deployment a maker chooses: how hard the radio
/// drives, which antenna goes on each end, and how much cable sits between each antenna
/// and its radio. Every value is in decibels and resolves to a hundredth of a decibel. A
/// new budget is 0 dBm between isotropic antennas with no cable, heard with
/// <see cref="RadioNoiseFigureDb"/>. The static members give each term of a budget on
/// its own, from the documents that define them.
/// </remarks>
public sealed record LoraLinkBudget
{
    /// <summary>A typical noise figure for a Semtech sub-GHz LoRa radio, in dB.</summary>
    /// <remarks>
    /// Semtech AN1200.22 takes 6 dB as the receiver behind the SX1272 and SX1276
    /// datasheet sensitivities.
    /// </remarks>
    public const double RadioNoiseFigureDb = 6;

    /// <summary>A typical noise figure for a LoRaWAN gateway receiver, in dB.</summary>
    /// <remarks>Semtech TN1300.05 works with 3 dB for a gateway.</remarks>
    public const double GatewayNoiseFigureDb = 3;

    /// <summary>The power the transmitting radio delivers at its antenna port, in dBm.</summary>
    public double TransmitPowerDbm { get; init; }

    /// <summary>The gain of the transmitting antenna over an isotropic antenna, in dBi.</summary>
    public double TransmitAntennaGainDbi { get; init; }

    /// <summary>
    /// The loss in the cable and connectors between the transmitting radio and its
    /// antenna, in dB.
    /// </summary>
    public double TransmitCableLossDb { get; init; }

    /// <summary>The gain of the receiving antenna over an isotropic antenna, in dBi.</summary>
    public double ReceiveAntennaGainDbi { get; init; }

    /// <summary>
    /// The loss in the cable and connectors between the receiving antenna and its radio,
    /// in dB.
    /// </summary>
    public double ReceiveCableLossDb { get; init; }

    /// <summary>
    /// The noise figure of the receiver, in dB, such as <see cref="RadioNoiseFigureDb"/>
    /// for a node or <see cref="GatewayNoiseFigureDb"/> for a gateway.
    /// </summary>
    public double NoiseFigureDb { get; init; } = RadioNoiseFigureDb;

    /// <summary>The equivalent isotropically radiated power, in dBm.</summary>
    /// <remarks>
    /// The transmit power plus the transmitting antenna gain, less the transmitting cable
    /// loss. This is the figure regional power ceilings limit.
    /// </remarks>
    public double EirpDbm => Db(NativeMethods.pamoja_lora_link_budget_eirp_centi_dbm(ToNative()));

    /// <summary>Returns the power that reaches the receiving radio across a path, in dBm.</summary>
    /// <param name="pathLossDb">
    /// The loss between the two antennas, such as <see cref="FreeSpaceLossDb"/>.
    /// </param>
    /// <returns>
    /// The EIRP less the path loss, plus the receiving antenna gain, less the receiving
    /// cable loss.
    /// </returns>
    public double ReceivedDbm(double pathLossDb) =>
        Db(NativeMethods.pamoja_lora_link_budget_received_centi_dbm(ToNative(), Centi(pathLossDb)));

    /// <summary>Returns the weakest signal the receiver can demodulate on a link, in dBm.</summary>
    /// <param name="link">
    /// The link settings, whose spreading factor and bandwidth set the floor.
    /// </param>
    /// <returns>
    /// The noise floor of the channel, raised by the noise figure and lowered by the
    /// demodulator SNR.
    /// </returns>
    public double SensitivityDbm(LoraLink link) =>
        Db(NativeMethods.pamoja_lora_link_budget_sensitivity_centi_dbm(ToNative(), link.Native));

    /// <summary>Returns the most path loss the link survives, in dB.</summary>
    /// <param name="link">
    /// The link settings, whose spreading factor and bandwidth set the sensitivity.
    /// </param>
    /// <returns>The loss above which the link does not close.</returns>
    public double MaxPathLossDb(LoraLink link) =>
        Db(NativeMethods.pamoja_lora_link_budget_max_path_loss_centi_db(ToNative(), link.Native));

    /// <summary>Returns how far above the sensitivity a signal arrives across a path, in dB.</summary>
    /// <param name="link">
    /// The link settings, whose spreading factor and bandwidth set the sensitivity.
    /// </param>
    /// <param name="pathLossDb">The loss between the two antennas.</param>
    /// <returns>
    /// The link margin, which is negative where the path loses more than the link survives.
    /// </returns>
    public double MarginDb(LoraLink link, double pathLossDb) =>
        Db(NativeMethods.pamoja_lora_link_budget_margin_centi_db(
            ToNative(), link.Native, Centi(pathLossDb)));

    /// <summary>
    /// Returns the most transmit power that keeps the EIRP at or under a ceiling, in dBm.
    /// </summary>
    /// <param name="eirpCeilingDbm">
    /// The EIRP limit, such as <see cref="LoraChannelPlan.MaxEirpDbm"/> for a frequency.
    /// </param>
    /// <returns>
    /// The ceiling less the transmitting antenna gain, plus the transmitting cable loss. A
    /// higher-gain antenna leaves less power for the radio.
    /// </returns>
    public double MaxTransmitPowerDbm(double eirpCeilingDbm) =>
        Db(NativeMethods.pamoja_lora_link_budget_max_transmit_power_centi_dbm(
            ToNative(), Centi(eirpCeilingDbm)));

    /// <summary>Returns the thermal noise power in a channel, in dBm.</summary>
    /// <param name="bandwidthHz">The channel bandwidth in hertz.</param>
    /// <returns>
    /// -174 dBm/Hz plus 10 log10 of the bandwidth, as Semtech AN1200.22 derives it.
    /// </returns>
    public static double NoiseFloorDbm(uint bandwidthHz) =>
        Db(NativeMethods.pamoja_lora_noise_floor_centi_dbm(bandwidthHz));

    /// <summary>
    /// Returns the signal-to-noise ratio the LoRa demodulator needs at a spreading factor, in dB.
    /// </summary>
    /// <param name="spreadingFactor">The spreading factor, clamped to 5 to 12.</param>
    /// <returns>
    /// The typical figure from Table 6-1 of the SX1261/2 datasheet, from -2.5 dB at SF5 to
    /// -20 dB at SF12.
    /// </returns>
    public static double DemodulatorSnrDb(byte spreadingFactor) =>
        Db(NativeMethods.pamoja_lora_demodulator_snr_centi_db(spreadingFactor));

    /// <summary>
    /// Returns the free-space basic transmission loss between isotropic antennas, in dB.
    /// </summary>
    /// <param name="distanceMeters">The distance between the antennas in meters.</param>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>
    /// The loss of Recommendation ITU-R P.525-5, equation (5), which grows by 6.02 dB each
    /// time the distance doubles.
    /// </returns>
    public static double FreeSpaceLossDb(uint distanceMeters, uint frequencyHz) =>
        Db(NativeMethods.pamoja_lora_free_space_loss_centi_db(distanceMeters, frequencyHz));

    /// <summary>
    /// Returns the radius of the first Fresnel ellipsoid at a point on a path, in millimeters.
    /// </summary>
    /// <param name="nearMeters">The distance from one antenna to the point, in meters.</param>
    /// <param name="farMeters">The distance from the point to the other antenna, in meters.</param>
    /// <param name="frequencyHz">The carrier frequency in hertz.</param>
    /// <returns>
    /// The radius of Recommendation ITU-R P.526-16, equation (2), widest halfway along the
    /// path. The diffraction zone starts where the clearance falls to 60% of it.
    /// </returns>
    public static uint FresnelRadiusMillimeters(uint nearMeters, uint farMeters, uint frequencyHz) =>
        NativeMethods.pamoja_lora_fresnel_radius_mm(nearMeters, farMeters, frequencyHz);

    /// <summary>
    /// Returns the most conducted power 47 CFR 15.247 allows a 902-928 MHz transmitter
    /// through an antenna, in dBm.
    /// </summary>
    /// <param name="antennaGainDbi">The directional gain of the transmitting antenna.</param>
    /// <param name="hoppingChannels">
    /// The number of hopping channels, or <c>null</c> for a system using digital modulation.
    /// </param>
    /// <returns>
    /// 1 W for digital modulation and for hopping on at least 50 channels, and 0.25 W for
    /// hopping on 25 to 49, less every decibel the gain exceeds 6 dBi; or <c>null</c> for
    /// hopping on fewer than 25 channels, which paragraph (b)(2) sets no limit for.
    /// </returns>
    public static double? FccMaxConductedDbm(double antennaGainDbi, ushort? hoppingChannels = null)
    {
        if (hoppingChannels is null)
        {
            return Db(NativeMethods.pamoja_lora_fcc_digital_max_conducted_centi_dbm(Centi(antennaGainDbi)));
        }

        int limit = NativeMethods.pamoja_lora_fcc_hopping_max_conducted_centi_dbm(
            hoppingChannels.Value, Centi(antennaGainDbi));
        return limit == int.MinValue ? null : Db(limit);
    }

    /// <summary>Describes this budget the way the C ABI carries it.</summary>
    /// <returns>The budget in hundredths of a decibel.</returns>
    private PamojaLoraLinkBudget ToNative() => new()
    {
        TransmitPowerCentiDbm = Centi(TransmitPowerDbm),
        TransmitAntennaGainCentiDbi = Centi(TransmitAntennaGainDbi),
        TransmitCableLossCentiDb = Centi(TransmitCableLossDb),
        ReceiveAntennaGainCentiDbi = Centi(ReceiveAntennaGainDbi),
        ReceiveCableLossCentiDb = Centi(ReceiveCableLossDb),
        NoiseFigureCentiDb = Centi(NoiseFigureDb),
    };

    /// <summary>Resolves a number of decibels to hundredths of a decibel.</summary>
    /// <param name="db">The value in decibels.</param>
    /// <returns>The nearest hundredth of a decibel.</returns>
    private static int Centi(double db) => (int)Math.Round(db * 100, MidpointRounding.AwayFromZero);

    /// <summary>Returns hundredths of a decibel as a number of decibels.</summary>
    /// <param name="centi">The value in hundredths of a decibel.</param>
    /// <returns>The value in decibels.</returns>
    private static double Db(int centi) => centi / 100.0;
}
