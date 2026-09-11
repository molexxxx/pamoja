"""The LoRa radio guide example; see docs/guides/radios.md."""

# ANCHOR: example
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import DutyCycle, sx126x

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

# The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
# transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
airtime = link.airtime_us(10)
events = sx126x.Irq.TX_DONE | sx126x.Irq.TIMEOUT
commands = [
    ("standby", sx126x.set_standby()),
    ("packet type", sx126x.set_packet_type_lora()),
    ("frequency", sx126x.set_rf_frequency(frequency)),
    ("pa config", sx126x.set_pa_config(power)),
    ("tx params", sx126x.set_tx_params(power, 40)),
    ("modulation", sx126x.set_lora_modulation_params(link)),
    ("packet", sx126x.set_lora_packet_params(link, 10, False)),
    ("irq", sx126x.set_dio_irq_params(events, events)),
    ("tx", sx126x.set_tx(airtime + 1_000_000)),
]
for name, data in commands:
    print(f"{name:<12}{data.hex(' ')}")

# Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
# chip back in standby.
irq = sx126x.irq(bytes([0x00, 0x01]))
sent = sx126x.Irq.TX_DONE in irq
timed_out = sx126x.Irq.TIMEOUT in irq
print(f"sent      tx done {sent}, timed out {timed_out}")
status = sx126x.status(0x2C)
print(f"status    {status.chip_mode}, {status.command_status}")

# A frame that arrives later comes with the signal levels it was heard at.
heard = sx126x.packet_status(bytes([0xDB, 0xF6, 0xE0]))
print(f"received  RSSI {heard.rssi_dbm} dBm, SNR {heard.snr_db} dB")

# The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
# ninety-nine times as long in silence before the next.
guard = DutyCycle(eu868.duty_cycle_permille(frequency))
held = guard.transmitted(0, link, 10)
print(f"airtime   {held} us, next frame after {guard.wait_us(0)} us")
# ANCHOR_END: example

# The bytes each command carries are pinned once, in the crate tests and the generated
# conformance vectors, so a guide asserts behavior instead.
assert power.setting_dbm == 14
assert len(commands) == 9
assert sent and not timed_out
assert held == airtime
assert not guard.ready(0)
assert guard.ready(held * 100)

# ANCHOR: rfm95w
from pamoja.lora import LinkBudget, plan_for
from pamoja.radios import sx126x, sx127x

# An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
# 16 dBm ceiling leave it the same 14 dBm, set through three registers.
band = plan_for("EU868")
channel = 868_100_000
dr3 = band.link_settings(3)
antenna = LinkBudget(transmit_antenna_gain_dbi=2.15, transmit_cable_loss_db=0.5)
rfm95w = sx127x.tx_power_under_ceiling(
    sx127x.PaOutput.PA_BOOST, antenna, band.max_eirp_dbm(channel)
)
print(
    f"rfm95w    {rfm95w.output_dbm} dBm on PA_BOOST: RegPaConfig {rfm95w.pa_config:02x}, "
    f"RegPaDac {rfm95w.pa_dac:02x}, RegOcp {rfm95w.ocp:02x}"
)

# The carrier and the modem go into registers while the chip stands by, and TX mode sends the
# frame the FIFO holds.
modem = sx127x.modem(dr3, channel)
print(f"carrier   RegFrf {sx127x.frequency_word(channel):06x}")
print(
    f"modem     RegModemConfig {modem.modem_config_1:02x} {modem.modem_config_2:02x} "
    f"{modem.modem_config_3:02x}"
)
print(f"tx mode   RegOpMode {sx127x.lora_op_mode(sx127x.Mode.TX):02x}")

# A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give its
# levels on the high frequency port.
flags = sx127x.Irq(0x50)
received = sx127x.Irq.RX_DONE in flags
corrupt = sx127x.Irq.PAYLOAD_CRC_ERROR in flags
print(f"irq       rx done {received}, crc error {corrupt}")
packet = sx127x.packet_status(bytes([0xF6, 0x30]), channel)
print(
    f"received  RSSI {packet.rssi_dbm} dBm, SNR {packet.snr_db} dB, "
    f"signal {packet.signal_rssi_dbm} dBm"
)


# An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
def fits(data_rate: int) -> bool:
    return sx126x.llcc68_supports(band.link_settings(data_rate))


print(f"llcc68    DR3 {fits(3)}, DR2 {fits(2)}")
# ANCHOR_END: rfm95w

assert rfm95w.output_dbm == 14
assert rfm95w.pa_config == 0xFC
assert modem.modem_config_2 == 0x94
assert received and not corrupt
assert fits(3) and not fits(2)

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
