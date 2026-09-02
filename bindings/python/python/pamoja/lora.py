"""Idiomatic LoRa link-budget facade.

LoRa buys kilometres of range on license-free bands at tiny power, and the price
is time: a transmission occupies the channel for a duration the radio settings
fix, and the regional rules cap how much of the time a node may transmit. This is
the arithmetic that keeps a node inside that budget, with no radio involved.
"""

from __future__ import annotations

from ._core import LoraLink

__all__ = ["LoraLink", "link", "messages_per_hour"]


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
