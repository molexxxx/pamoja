"""The LoRa radio guide example; see docs/guides/radios.md."""

# ANCHOR: example
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, SimulatedLoraChip, sx126x

# An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
# 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
# and pigtail decide how hard the amplifier may drive under that cap.
eu868 = plan_for("EU868")
frequency = 868_100_000
link = eu868.link_settings(3)
whip = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
ceiling = eu868.max_eirp_dbm(frequency)
power = sx126x.tx_power_under_ceiling(sx126x.Amplifier.HIGH_POWER, whip, ceiling)
print(f"power     {power.setting_dbm} dBm under a {ceiling} dBm EIRP ceiling")

# A simulated SX1262 stands in for the chip on the node's board, driven by the same code that
# drives a real one, and it reports what that code told it.
chip = SimulatedLoraChip.sx126x(sx126x.Amplifier.HIGH_POWER)
bench = chip.radio()
bench.configure(frequency, link, power.setting_dbm)
tuned = chip.tuning()
print(
    f"tuned     {tuned.frequency_hz / 1e6:.1f} MHz, SF{tuned.link.spreading_factor} "
    f"at {tuned.link.bandwidth_hz // 1000} kHz, {tuned.output_dbm} dBm"
)

# The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band that
# holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long in silence
# before the next.
reading = b"level=0.42"
airtime = bench.transmit(reading)
print(f"sent      {len(chip.sent()[0].payload)} bytes, {airtime} us on air")
guard = DutyCycle(eu868.duty_cycle_permille(frequency))
guard.transmitted(0, link, len(reading))
print(f"silence   the next frame starts {guard.wait_us(0)} us after this one did")

# A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
chip.hear(b"ack", -109, -2.5)
heard = bench.receive(1_000_000)
if heard.outcome == "Frame":
    print(
        f"received  {heard.payload.decode()} at {heard.rssi_dbm:.2f} dBm, "
        f"SNR {heard.snr_db:.2f} dB"
    )

# With nothing on the air the reception times out, and a frame whose CRC fails is dropped
# rather than handed over.
quiet = bench.receive(1_000_000).outcome
chip.hear_corrupt(-121, -12)
broken = bench.receive(1_000_000).outcome
print(f"then      {quiet}, then {broken}")
bench.close()
# ANCHOR_END: example

assert power.setting_dbm == 14
assert tuned.frequency_hz == frequency
assert chip.sent()[0].payload == reading
assert guard.wait_us(0) == airtime * 100
assert (quiet, broken) == ("Timeout", "Corrupt")

# ANCHOR: rfm95w
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import SimulatedLoraChip, sx126x, sx127x

# An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
# 16 dBm ceiling leave it the same 14 dBm.
band = plan_for("EU868")
channel = 868_100_000
dr3 = band.link_settings(3)
antenna = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
rfm95w = sx127x.tx_power_under_ceiling(
    sx127x.PaOutput.PA_BOOST, antenna, band.max_eirp_dbm(channel)
)

# The same driver calls tune it, through registers this time. Its synthesizer steps in 61 Hz,
# so the carrier lands on the step nearest the one asked for.
module = SimulatedLoraChip.sx127x(sx127x.PaOutput.PA_BOOST)
radio = module.radio()
radio.configure(channel, dr3, rfm95w.output_dbm)
carrier = module.tuning()
print(
    f"rfm95w    {carrier.output_dbm} dBm on PA_BOOST, carrier {carrier.frequency_hz} Hz, "
    f"{abs(channel - carrier.frequency_hz)} Hz from {channel}"
)

# The SX1276 gives a packet's strength in whole decibels, and works out the strength of the
# signal itself from the SNR when it arrived under the noise.
module.hear(b"ack", -109, -2.5)
packet = radio.receive(1_000_000)
if packet.outcome == "Frame":
    print(
        f"received  RSSI {packet.rssi_dbm:.2f} dBm, SNR {packet.snr_db:.2f} dB, "
        f"signal {packet.signal_rssi_dbm:.2f} dBm"
    )
radio.close()


def carries(data_rate: int) -> str:
    """Whether an LLCC68 in the RFM95W's place could carry a data rate: DR3, but not DR2,
    which is SF10 at 125 kHz."""
    return "carries" if sx126x.llcc68_supports(band.link_settings(data_rate)) else "cannot carry"


print(f"llcc68    {carries(3)} DR3 and {carries(2)} DR2")
# ANCHOR_END: rfm95w

assert rfm95w.output_dbm == 14
assert carrier.output_dbm == 14
assert abs(channel - carrier.frequency_hz) <= 61
assert carries(3) == "carries"
# ANCHOR: hardware
from pamoja.core import PamojaError
from pamoja.radios import LoraRadio

# An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin on
# GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
spi, gpio_chip, reset_line = "/dev/spidev0.0", "/dev/gpiochip0", 25
print(f"radio     an RFM95W on {spi}, reset on GPIO{reset_line}")
print(f"plan      {channel} Hz at DR3, {rfm95w.output_dbm} dBm on PA_BOOST")

# Opening resets the chip and reads its version back, so a wiring mistake is caught here rather
# than on the first frame. With no radio wired, this is the line that prints.
try:
    radio = LoraRadio.open_sx127x(spi, gpio_chip, reset_line, sx127x.PaOutput.PA_BOOST)
except PamojaError:
    radio = None

if radio is None:
    print("absent    no radio answered, so nothing went out")
else:
    with radio:
        radio.configure(channel, dr3, rfm95w.output_dbm)
        airtime_us = radio.transmit(b"21.5")
        print(f"sent      a reading in {airtime_us} us on air")
# ANCHOR_END: hardware
