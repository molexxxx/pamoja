"""The CoAP guide example; see docs/guides/coap.md."""

# ANCHOR: example
import asyncio

from pamoja.coap import CoapClient, CoapServer, Reliability
from pamoja.core import PamojaError


async def main():
    # The gateway in the orchard's shed. It takes moisture readings from every row, and
    # holds the irrigation valve's state for the rows to observe. Port 0 lets the system
    # pick a free port, which the rows are pointed at below.
    gateway = CoapServer("127.0.0.1:0")
    await gateway.connect()
    await gateway.subscribe("orchard/+/moisture")
    await gateway.send("orchard/valve", "closed")
    port = gateway.local_port
    print("gateway   takes moisture readings on orchard/+/moisture")

    # A battery-powered sensor in row 7. Its reading is confirmable, so it waits for the
    # gateway's acknowledgment and retransmits until one comes back.
    row7 = CoapClient(host="127.0.0.1", port=port, ack_timeout_ms=200)
    await row7.connect()
    await row7.send("orchard/row-7/moisture", "31")
    print("row-7     reported 31, and the gateway acknowledged it")
    reading = await gateway.recv()
    print(f"gateway   took {reading.text} from {reading.topic}")

    # Observing the valve registers the row with the gateway, which answers with the
    # valve's state now and notifies every change after it, as RFC 7641 describes.
    await row7.subscribe("orchard/valve")
    current = await row7.recv()
    print(f"row-7     observes {current.topic}, which reads {current.text}")
    await gateway.send("orchard/valve", "open")
    observers = gateway.observers("orchard/valve")
    print(f"gateway   opened the valve for {observers} observer")
    change = await row7.recv()
    print(f"row-7     {change.topic} now reads {change.text}")

    # A path the gateway does not take is answered 4.04, and a confirmable send reports
    # that rather than counting the reading as delivered.
    try:
        await row7.send("orchard/row-7/battery", "3.1")
        print("row-7     the battery reading was taken, which should never happen")
    except PamojaError as error:
        print(f"row-7     battery refused: {error}")

    # Row 8 sends non-confirmable: once and unacknowledged, which costs the least radio
    # time and suits a reading whose loss costs nothing.
    row8 = CoapClient(host="127.0.0.1", port=port, reliability=Reliability.NON_CONFIRMABLE)
    await row8.connect()
    await row8.send("orchard/row-8/moisture", "27")
    print("row-8     sent 27 without waiting for an answer")
    unconfirmed = await gateway.recv()
    print(f"gateway   took {unconfirmed.text} from {unconfirmed.topic}")

    # Row 9 is pointed at port 1, where nothing listens. A confirmable send retransmits on
    # a doubling wait and then gives up. RFC 7252's defaults would take more than a minute
    # to get there, so this one waits 20 ms and retransmits once.
    row9 = CoapClient(host="127.0.0.1", port=1, ack_timeout_ms=20, max_retransmits=1)
    await row9.connect()
    try:
        await row9.send("orchard/row-9/moisture", "29")
        print("row-9     an empty port acknowledged it, which should never happen")
    except PamojaError as error:
        print(f"row-9     gave up unacknowledged: {error}")

    for link in (row7, row8, row9):
        await link.disconnect()
    await gateway.disconnect()
    return reading, current, change, observers, unconfirmed


reading, current, change, observers, unconfirmed = asyncio.run(main())
# ANCHOR_END: example

assert reading.topic == "orchard/row-7/moisture"
assert current.text == "closed"
assert change.text == "open"
assert observers == 1
assert unconfirmed.topic == "orchard/row-8/moisture"
