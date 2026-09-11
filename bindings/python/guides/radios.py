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
