"""The CoAP guide example; see docs/guides/coap.md."""

# ANCHOR: example
import asyncio

from pamoja.coap import CoapClient, Reliability
from pamoja.core import PamojaError


async def main() -> None:
    # CoAP runs over UDP and opens no session, so connecting only binds a local socket.
    # Nothing is listening on the far side here, and for a non-confirmable send nothing
    # needs to be.
    reporter = CoapClient(
        host="127.0.0.1", port=5683, reliability=Reliability.NON_CONFIRMABLE
    )
    await reporter.connect()
    print(f"reporter  connected: {await reporter.is_connected()}")

    # Non-confirmable delivery is at most once: the datagram leaves unacknowledged, which
    # is what a battery-powered node sends when one missed reading costs nothing.
    await reporter.send("sensors/1/temperature", b"21.5")
    print("reporter  sent 21.5 and did not wait for an answer")

    # A command is different: it has to arrive. Confirmable delivery retransmits until an
    # acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait and
    # four retransmissions; both are cut short here so the guide does not sit waiting.
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
        print("commander the valve acknowledged the command")
    except PamojaError as error:
        print(f"commander gave up unacknowledged: {error}")

    await reporter.disconnect()
    print(f"reporter  disconnected: {not await reporter.is_connected()}")


asyncio.run(main())
# ANCHOR_END: example
