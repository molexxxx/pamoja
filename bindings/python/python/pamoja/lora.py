"""Idiomatic LoRa link-budget facade.

LoRa buys kilometres of range on license-free bands at tiny power, and the price
is time: a transmission occupies the channel for a duration the radio settings
fix, and the regional rules cap how much of the time a node may transmit. This is
the arithmetic that keeps a node inside that budget, with no radio involved.
"""

from __future__ import annotations

from ._core import (
    ChannelPlan,
    ChannelPlanBuilder,
    LoraBeacon,
    LoraChannelBlock,
    LoraDataRate,
    LoraLink,
    LoraMaxPayload,
    LoraPlanInfo,
    LoraSubBand,
)

__all__ = [
    "ChannelPlan",
    "ChannelPlanBuilder",
    "LoraBeacon",
    "LoraChannelBlock",
    "LoraDataRate",
    "LoraLink",
    "LoraMaxPayload",
    "LoraPlanInfo",
    "LoraSubBand",
    "REGIONS",
    "link",
    "messages_per_hour",
    "messages_per_hour_at",
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

    :param spreading_factor: The spreading factor, clamped to 7 (fastest) to 12
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
