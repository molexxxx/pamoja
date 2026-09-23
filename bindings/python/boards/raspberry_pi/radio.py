"""A LoRa radio on the header: an RFM95W breakout on SPI0, beaconing a reading and printing
every frame it hears in between.

Wire the breakout's VIN to a 3V3 pin, GND to ground, SCK to GPIO11, MISO to GPIO9, MOSI to
GPIO10, CS to GPIO8 (CE0), and RST to GPIO25, and screw on an antenna for the band before
powering it. Two boards running it hear each other. See docs/boards/raspberry-pi.md.
"""

# ANCHOR: example
import math
import time

from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, LoraRadio, sx127x

# The header's first SPI chip select, the GPIO chip its lines are on, and the line the
# breakout's reset pin is wired to.
SPI = "/dev/spidev0.0"
CHIP = "/dev/gpiochip0"
RESET_LINE = 25

# The channel this node uses, the data rate it sends at, and how long it listens between
# beacons.
FREQUENCY_HZ = 868_100_000
DATA_RATE = 3
LISTEN_US = 10_000_000


def main() -> None:
    # The regional plan decides the channel's power ceiling and its duty cycle, so no limit
    # below is a number anyone has to remember.
    plan = plan_for("EU868")
    link = plan.link_settings(DATA_RATE)
    ceiling_dbm = plan.max_eirp_dbm(FREQUENCY_HZ)
    permille = plan.duty_cycle_permille(FREQUENCY_HZ)

    # A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
    # ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
    whip = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
    output_dbm = math.floor(whip.max_transmit_power_dbm(ceiling_dbm))

    # Opening resets the chip and reads its version back, so a wiring mistake is caught
    # here rather than on the first frame.
    radio = LoraRadio.open_sx127x(SPI, CHIP, RESET_LINE, sx127x.PaOutput.PA_BOOST)
    with radio:
        radio.configure(FREQUENCY_HZ, link, output_dbm)
        print(
            f"beacon on {FREQUENCY_HZ} Hz at DR{DATA_RATE}, "
            f"{output_dbm} dBm under a {ceiling_dbm} dBm ceiling"
        )

        # The duty cycle is the radio's other budget: each frame buys silence in proportion
        # to its airtime, and the guard says when the next one may go out.
        duty = DutyCycle(permille)
        started = time.monotonic_ns()
        reading = 0

        while True:
            # Listening returns as soon as a frame arrives, and a frame comes with the
            # levels it was heard at: how strong it was, and how far above the noise.
            heard = radio.receive(LISTEN_US)
            if heard.outcome == "Frame":
                print(
                    f"heard  {heard.payload.decode(errors='replace')} at "
                    f"{heard.rssi_dbm:.0f} dBm, SNR {heard.snr_db:.1f} dB"
                )
            elif heard.outcome == "Corrupt":
                print("heard  a frame whose CRC failed")

            now_us = (time.monotonic_ns() - started) // 1000
            if duty.ready(now_us):
                frame = f"pi reading {reading}"
                airtime_us = radio.transmit(frame.encode())
                duty.transmitted(now_us, link, len(frame))
                print(f"sent   {frame} in {airtime_us} us on air")
                reading += 1
# ANCHOR_END: example


if __name__ == "__main__":
    main()
