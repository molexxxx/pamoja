"""The CoAP guide example; see docs/guides/coap.md."""

# ANCHOR: example
import asyncio

from pamoja.coap import CoapClient, Reliability
from pamoja.core import PamojaError


async def main() -> None:
    # CoAP runs over UDP and opens no session, so connecting only binds a local socket.
    # Nothing is listening on the far side here, and nothing needs to be.
    reporter = CoapClient(
        host="127.0.0.1", port=5683, reliability=Reliability.NON_CONFIRMABLE
    )
    assert not await reporter.is_connected()
    await reporter.connect()
    assert await reporter.is_connected()

    # Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    # is what a battery-powered node sends when one missed reading costs nothing.
    await reporter.send("sensors/1/temperature", b"21.5")

    # Confirmable delivery retransmits until an ACK arrives. RFC 7252 fixes the defaults
    # at a two-second wait and four retransmissions; both are cut short here.
    commander = CoapClient(
        host="127.0.0.1",
        port=5683,
        reliability=Reliability.CONFIRMABLE,
        ack_timeout_ms=20,
        max_retransmits=1,
    )
    await commander.connect()
    try:
        await commander.send("actuators/valve", b"open")
    except PamojaError:
        pass
    else:
        raise AssertionError("an unacknowledged command should be reported, not dropped")

    await reporter.disconnect()
    assert not await reporter.is_connected()


asyncio.run(main())
# ANCHOR_END: example
