"""The LoRa airtime guide example; see docs/guides/lora.md."""

# ANCHOR: example
from pamoja.lora import messages_per_hour, plan_for

# EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
# that reaches furthest and holds the channel longest.
plan = plan_for("EU868")
link = plan.link_settings(0)
print(f"{plan.name} DR0 is SF{link.spreading_factor} at 125 kHz")

# The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an explicit
# header and CRC on, carrying a ten-byte reading.
airtime = link.airtime_us(10)
print(f"airtime   {airtime / 1e6:.2f} s for ten bytes")

# 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every transmission
# buys ninety-nine times its own length in silence.
channel = 868_100_000
permille = plan.duty_cycle_permille(channel)
print(f"channel   {permille} per mille duty cycle, {plan.max_eirp_dbm(channel)} dBm")

off_time = link.min_off_time_us(10, permille)
print(f"silence   {off_time / 1e6:.1f} s owed after each reading")

# The airtime plus that silence is what one reading really costs, which is the budget a
# deployment plans against.
print(f"budget    {messages_per_hour(link, 10, permille)} readings an hour")

# A frequency in no sub-band the plan describes has no duty cycle to budget against. That
# is a limit published elsewhere, not permission to transmit.
outside = plan.duty_cycle_permille(700_000_000)
print(f"700 MHz  is outside this plan, so it budgets nothing: {outside is None}")
# ANCHOR_END: example

assert plan.name == "EU863-870"
assert link.spreading_factor == 12
assert airtime == 991_232
assert permille == 10
assert plan.max_eirp_dbm(channel) == 16
assert off_time == airtime * 99
assert messages_per_hour(link, 10, permille) == 36
assert outside is None

# ANCHOR: range
import math

from pamoja.lora import (
    GATEWAY_NOISE_FIGURE_DB,
    LinkBudget,
    fcc_max_conducted_dbm,
    free_space_loss_db,
    fresnel_radius_mm,
    plan_for,
)

eu868 = plan_for("EU868")
dr0 = eu868.link_settings(0)
frequency = 868_100_000

# A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
# 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
whip = {"transmit_antenna_gain_dbi": 2.15, "transmit_cable_loss_db": 0.5}
gateway = {
    "receive_antenna_gain_dbi": 6,
    "receive_cable_loss_db": 1.5,
    "noise_figure_db": GATEWAY_NOISE_FIGURE_DB,
}

# The plan caps what leaves the antenna, so the antenna and cable decide how hard the
# radio may drive. A radio takes whole decibels, so the setting rounds down.
ceiling = eu868.max_eirp_dbm(frequency)
most = LinkBudget(**whip).max_transmit_power_dbm(ceiling)
node = LinkBudget(transmit_power_dbm=math.floor(most), **whip, **gateway)
print(f"radio     {most:.2f} dBm allowed, set to {node.transmit_power_dbm:.0f} dBm")
print(f"eirp      {node.eirp_dbm():.2f} dBm under a {ceiling} dBm ceiling")

# The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
# loss the link survives.
sensitivity = node.sensitivity_dbm(dr0)
survives = node.max_path_loss_db(dr0)
print(f"gateway   hears down to {sensitivity:.2f} dBm, so {survives:.2f} dB of path loss")

# Free space at three distances, and what each path leaves to spare.
margins = []
for distance_m in (2_000, 5_000, 15_000):
    loss = free_space_loss_db(distance_m, frequency)
    margin = node.margin_db(dr0, loss)
    print(f"{distance_m // 1_000:>2} km     {loss:.2f} dB lost, {margin:.2f} dB to spare")
    margins.append(margin)

# Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
# diffraction loss, which starts once the clearance falls below 60% of its radius.
radius = fresnel_radius_mm(2_500, 2_500, frequency)
clear = radius * 6 // 10
print(f"fresnel   {radius / 1000:.1f} m at the middle of 5 km, keep {clear / 1000:.1f} m clear")

# In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
# decibel an antenna has over 6 dBi.
limit = fcc_max_conducted_dbm(9, hopping_channels=64)
print(f"fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit:.2f} dBm")
# ANCHOR_END: range

assert most == 14.35
assert node.transmit_power_dbm == 14
assert node.eirp_dbm() == 15.65
assert sensitivity == -140.03
assert survives == 160.18
assert margins == [62.94, 54.98, 45.44]
assert radius == 20_777
assert limit == 27
