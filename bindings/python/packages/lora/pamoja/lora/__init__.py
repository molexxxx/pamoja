"""Idiomatic LoRa link-budget facade.

LoRa buys kilometers of range on license-free bands at tiny power, and the price
is time: a transmission occupies the channel for a duration the radio settings
fix, and the regional rules cap how much of the time a node may transmit. This is
the arithmetic that keeps a node inside that budget, with no radio involved.

It also works out how far a link reaches: the EIRP an antenna and cable leave, the
free-space loss of a path, the first Fresnel zone, the sensitivity of a receiver,
and the power 47 CFR 15.247 allows through an antenna, in decibels resolved to a
hundredth.
"""

from __future__ import annotations

from pamoja._native import (
    ChannelPlan,
    ChannelPlanBuilder,
    LoraBeacon,
    LoraChannelBlock,
    LinkBudget,
    LoraDataRate,
    LoraLink,
    LoraMaxPayload,
    LoraPlanInfo,
    LoraSubBand,
    lora_demodulator_snr_db,
    lora_fcc_max_conducted_dbm,
    lora_free_space_loss_db,
    lora_fresnel_radius_mm,
    lora_noise_floor_dbm,
)

__all__ = [
    "ChannelPlan",
    "ChannelPlanBuilder",
    "GATEWAY_NOISE_FIGURE_DB",
    "LinkBudget",
    "LoraBeacon",
    "LoraChannelBlock",
    "LoraDataRate",
    "LoraLink",
    "LoraMaxPayload",
    "LoraPlanInfo",
    "LoraSubBand",
    "RADIO_NOISE_FIGURE_DB",
    "REGIONS",
    "demodulator_snr_db",
    "fcc_max_conducted_dbm",
    "free_space_loss_db",
    "fresnel_radius_mm",
    "link",
    "messages_per_hour",
    "messages_per_hour_at",
    "noise_floor_dbm",
    "plan_for",
]

#: The bands with a published channel plan.
REGIONS = (
    "EU868",
    "US915",
    "EU433",
    "AU915",
    "CN470",
    "AS923",
    "KR920",
    "IN865",
    "RU864",
)

#: A typical noise figure for a Semtech sub-GHz LoRa radio, in dB. Semtech AN1200.22
#: takes it as the receiver behind the SX1272 and SX1276 datasheet sensitivities.
RADIO_NOISE_FIGURE_DB = 6.0

#: A typical noise figure for a LoRaWAN gateway receiver, in dB, from Semtech TN1300.05.
GATEWAY_NOISE_FIGURE_DB = 3.0


def link(
    spreading_factor: int,
    bandwidth_hz: int,
    coding_rate_denominator: int = 5,
    preamble_symbols: int = 8,
    explicit_header: bool = True,
    crc: bool = True,
) -> LoraLink:
    """Describe a LoRa link, clamping every value to its LoRa range.

    The defaults are coding rate 4/5, an eight-symbol preamble, an explicit
    header, and CRC on, which is a typical uplink.

    :param spreading_factor: The spreading factor, clamped to 5 (fastest) to 12
        (longest range).
    :param bandwidth_hz: The channel bandwidth in hertz, such as ``125_000``.
    :param coding_rate_denominator: The coding-rate denominator, clamped to 5 to 8.
    :param preamble_symbols: The preamble length in symbols.
    :param explicit_header: Whether the frame carries an explicit header.
    :param crc: Whether the frame carries a CRC.
    :returns: The link, which answers for its own airtime and off time.
    """
    return LoraLink(
        spreading_factor,
        bandwidth_hz,
        coding_rate_denominator,
        preamble_symbols,
        explicit_header,
        crc,
    )


def messages_per_hour(
    settings: LoraLink, payload_len: int, duty_cycle_permille: int
) -> int:
    """Return how many transmissions of a payload fit in an hour under a limit.

    The airtime plus the silence it forces is what one transmission really costs,
    so this is the message budget a deployment plans against.

    :param settings: The link settings.
    :param payload_len: The payload length in bytes.
    :param duty_cycle_permille: The limit in parts per thousand, so ``10`` is 1%.
    :returns: The number of whole transmissions per hour, or ``0`` when the limit
        forbids transmitting.
    """
    off_time = settings.min_off_time_us(payload_len, duty_cycle_permille)
    if off_time is None:
        return 0
    return 3_600_000_000 // (settings.airtime_us(payload_len) + off_time)


def plan_for(region: str) -> ChannelPlan:
    """Return the published channel plan for a region.

    A channel plan is what a regulator and the LoRa Alliance publish about one
    band: which data rates exist, what each carries, how much of the time a node
    may hold a frequency, and where it listens for a downlink. The plan reports
    those facts and costs a transmission out against them; it never refuses one,
    because a deployment may hold licensed spectrum or be working under emergency
    provisions and only the operator knows which.

    :param region: The band to describe, such as ``EU868``. See :data:`REGIONS`.
    :returns: The plan, which answers every question about that band.
    :raises ValueError: If no published region goes by that name.

    >>> plan = plan_for("EU868")
    >>> plan.name
    'EU863-870'
    >>> plan.link_settings(0).spreading_factor
    12
    >>> plan.duty_cycle_permille(868_100_000)
    10
    """
    return ChannelPlan.for_region(region)


def messages_per_hour_at(
    plan: ChannelPlan, data_rate: int, payload_len: int, frequency_hz: int
) -> int | None:
    """Return how many transmissions fit in an hour at a data rate the plan defines.

    This is the budget question a deployment actually asks: not what the radio
    can do, but how often it may speak on this band at this setting. The duty
    cycle of the frequency it transmits on decides the answer.

    :param plan: The channel plan to read.
    :param data_rate: The uplink data-rate number.
    :param payload_len: The payload length in bytes.
    :param frequency_hz: The frequency the node transmits on.
    :returns: The number of whole transmissions per hour, or ``None`` when the
        plan does not describe that data rate or frequency.

    >>> plan = plan_for("EU868")
    >>> messages_per_hour_at(plan, 5, 20, 868_100_000) > 0
    True
    """
    settings = plan.link_settings(data_rate)
    permille = plan.duty_cycle_permille(frequency_hz)
    if settings is None or permille is None:
        return None
    return messages_per_hour(settings, payload_len, permille)


def noise_floor_dbm(bandwidth_hz: int) -> float:
    """Return the thermal noise power in a channel, in dBm.

    This is -174 dBm/Hz plus ``10 log10`` of the bandwidth, as Semtech AN1200.22
    derives it. LoRa demodulates below this floor by the processing gain its
    spreading factor buys, and :func:`demodulator_snr_db` gives how far.

    :param bandwidth_hz: The channel bandwidth in hertz; ``0`` counts as one hertz.
    :returns: The noise power in dBm, to a hundredth of a decibel.

    >>> noise_floor_dbm(125_000)
    -123.03
    """
    return lora_noise_floor_dbm(bandwidth_hz)


def demodulator_snr_db(spreading_factor: int) -> float:
    """Return the signal-to-noise ratio the LoRa demodulator needs, in dB.

    These are the typical figures of Table 6-1 in the Semtech SX1261/2 datasheet,
    from -2.5 dB at SF5 to -20 dB at SF12. A negative ratio is a signal received
    below the noise.

    :param spreading_factor: The spreading factor, clamped to 5 to 12.
    :returns: The required SNR in dB.

    >>> demodulator_snr_db(12)
    -20.0
    """
    return lora_demodulator_snr_db(spreading_factor)


def free_space_loss_db(distance_m: int, frequency_hz: int) -> float:
    """Return the free-space basic transmission loss between isotropic antennas, in dB.

    This is Recommendation ITU-R P.525-5, equation (5): the loss with nothing in the
    way, which grows by 6.02 dB each time the distance doubles. Terrain that enters
    the first Fresnel ellipsoid adds to it; see :func:`fresnel_radius_mm`.

    :param distance_m: The distance between the antennas in meters.
    :param frequency_hz: The carrier frequency in hertz.
    :returns: The loss in dB, to a hundredth of a decibel.

    >>> free_space_loss_db(5_000, 868_100_000)
    105.2
    """
    return lora_free_space_loss_db(distance_m, frequency_hz)


def fresnel_radius_mm(near_m: int, far_m: int, frequency_hz: int) -> int:
    """Return the radius of the first Fresnel ellipsoid at a point on a path, in millimeters.

    This is Recommendation ITU-R P.526-16, equation (2). The radius is widest halfway
    along the path, and the diffraction zone starts where the clearance falls to 60%
    of it.

    :param near_m: The distance from one antenna to the point, in meters.
    :param far_m: The distance from the point to the other antenna, in meters.
    :param frequency_hz: The carrier frequency in hertz.
    :returns: The radius in millimeters, or ``0`` for a path of no length.

    >>> fresnel_radius_mm(2_500, 2_500, 868_100_000)
    20777
    """
    return lora_fresnel_radius_mm(near_m, far_m, frequency_hz)


def fcc_max_conducted_dbm(
    antenna_gain_dbi: float, hopping_channels: int | None = None
) -> float | None:
    """Return the most conducted power 47 CFR 15.247 allows through an antenna, in dBm.

    The limit applies to a 902-928 MHz transmitter: 1 W for digital modulation and
    for hopping on at least 50 channels, and 0.25 W for hopping on 25 to 49, less
    every decibel the antenna gain exceeds 6 dBi.

    :param antenna_gain_dbi: The directional gain of the transmitting antenna.
    :param hopping_channels: The number of hopping channels, or ``None`` for a
        system using digital modulation.
    :returns: The limit in dBm, or ``None`` for hopping on fewer than 25 channels,
        which paragraph (b)(2) sets no limit for.

    >>> fcc_max_conducted_dbm(9.0)
    27.0
    >>> fcc_max_conducted_dbm(2.15, hopping_channels=24) is None
    True
    """
    return lora_fcc_max_conducted_dbm(antenna_gain_dbi, hopping_channels)
