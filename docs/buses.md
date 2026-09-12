# Buses and links

Everything a node measures or moves reaches it over a few wires, and each set of
wires has its own rules: how many lines, who talks when, how a device is picked
out, how fast it can go, how far it reaches. This page is those rules, in the
plain terms the guides use, for every bus and link pamoja speaks. Each section
says what the bus is for, what to get right, and which crate carries the logic,
and links the document the figures come from. The [Hardware](hardware.md) page
holds the same documents as cards, and the board pages show the wiring on a
[Raspberry Pi](boards/raspberry-pi.md), an [ESP32](boards/esp32.md), and an
[RP2040](boards/rp2040.md).

One idea runs through all of them. pamoja carries the part of each bus that is
exact and easy to get wrong, the address byte, the clock mode, the checksum, the
airtime, and leaves the wiring to the board. A driver is written against the
`embedded-hal` traits in [`pamoja-hal`](guides/hal.md), so the same code runs
over a microcontroller's peripheral, over the kernel's device file on a Linux
board, and over a scripted bus in a test with nothing plugged in.

## I2C

Two wires, data and clock, shared by every device on the bus, each pulled up to
the supply and driven only low, so any device can hold a line down and none can
short another. One controller starts every transfer and addresses a target by a
7-bit address, or a 10-bit one where a bus is crowded; the two forms may share a
bus in every speed mode. The specification names four rates: 100 kbit/s
standard, 400 kbit/s fast, 1 Mbit/s fast-mode plus, and 3.4 Mbit/s high speed.
What limits the number of devices is the capacitance of the wiring, not a
count.

What to get right: the address, which a breakout board often lets you change
with a solder jumper, so a datasheet's `0x76` may be `0x77` on the bench; the
pull-ups, which a breakout usually carries and a bare chip does not; and the
rate, which every device on the bus must accept.

pamoja: the address-frame encoding and the reserved-address checks are in
[`pamoja-gpio`](guides/gpio.md); the bus traits, the scripted bus, and the
Linux backend are in [`pamoja-hal`](guides/hal.md); the drivers for the
[shipped sensors](guides/sensors.md) and [actuators](guides/actuators.md) run
over it. Source: NXP's
[UM10204](https://www.nxp.com/docs/en/user-guide/UM10204.pdf), the I2C-bus
specification and user manual, revision 7.0.

## SPI

Four wires: a clock from the controller, one data line each way, and a select
line per device, pulled low to speak to that device and no other. There are no
addresses; the select line is the address. Data moves both ways at once, and
the clock's idle level and sampling edge are a pair of settings, polarity and
phase, that the datasheet gives as a mode number from 0 to 3. Rates run to
tens of megahertz, which is why displays, flash, and radios sit on SPI rather
than I2C.

What to get right: the mode, since a wrong one reads garbage rather than
failing; a select line per device, wired to a pin the software knows; and the
speed the part accepts.

pamoja: the mode-to-polarity-and-phase mapping is in
[`pamoja-gpio`](guides/gpio.md); the bus traits and the Linux backend are in
[`pamoja-hal`](guides/hal.md); the BME280 and BMP280 drivers run over it as
well as over I2C. Source: Microchip's SPI reference manual,
[DS70005185A](https://ww1.microchip.com/downloads/en/DeviceDoc/70005185a.pdf).
No standards body defines SPI, so pin names, timing and rate come from each
device's own datasheet.

## 1-Wire

One data line and ground, and often that is all: the line carries power to the
device as well as data, so a waterproof thermometer hangs off a long cable with
two conductors. Every device carries a 64-bit code with a family byte and a
CRC, and the controller finds them all with a search that walks the code bit by
bit. Timing does the rest: a reset pulse and a presence pulse, then slots of a
few microseconds in which a bit is written or read.

What to get right: the slot timing, which user space on a Linux board cannot
hold, so the kernel's own driver does it there; the pull-up, which a bus with
parasitic power needs stronger than usual; and the CRC, which is the only sign
a long cable corrupted a reading.

pamoja: the bit-banged bus, the ROM commands, the search, and the CRC are in
[`pamoja-hal`](guides/hal.md), timed as the Linux kernel's `w1` driver times
them; the DS18B20 driver runs over that bus on a microcontroller and reads the
kernel's `w1_slave` file on a Linux board. Source: the kernel's
[`w1`](https://github.com/torvalds/linux/tree/master/drivers/w1) sources, as
the [hardware page](hardware.md#ds18b20) records for the part.

## UART

Two wires, one each way, with no clock: each side agrees in advance on a rate
and frames every byte itself. The frame is a start bit, eight data bits (nine
where a bus needs an address flag), an optional parity bit, and one or two
stop bits, which the reference manual calls non-return-to-zero and which the
most common setting writes as `8, N, 1`. A baud rate generator derives the
rate from the chip's clock, and both ends must land on the same figure.

What to get right: transmit crosses to receive, so one board's TX wires to the
other's RX; the levels, since a board's UART is 3.3 V logic and a PC serial
port or an RS-232 cable is not; and the framing, since a bare stream of bytes
has no packet boundaries of its own.

pamoja: [`pamoja-serial`](guides/serial.md) carries the framing, SLIP and COBS
byte stuffing with streaming decoders, so a byte stream carries discrete
packets; [`pamoja-mavlink`](guides/mavlink.md) opens a serial port for a flight
controller. Source: Microchip's UART reference manual,
[DS70000582](https://ww1.microchip.com/downloads/en/DeviceDoc/70000582e.pdf).

## PWM

One output pin switched on and off at a fixed frequency, where the fraction of
each period spent on, the duty cycle, is the value. A hobby servo reads the
width of the pulse, not the fraction: a frame near 50 Hz with a pulse of about
1 to 2 ms sets its angle. A dimmer or a heater reads the fraction. A
microcontroller has PWM channels on chip; a Linux board has a few pins with
hardware PWM, and a dedicated chip such as the PCA9685 adds sixteen channels
over I2C, timed from its own oscillator with a prescale register that fixes the
frame rate.

What to get right: the frequency the load wants, and the prescale value that
produces it, which is a rounding calculation the datasheet spells out; and the
counts that correspond to a pulse width at that frequency.

pamoja: [`pamoja-actuators`](guides/actuators.md) carries the PCA9685's
frequency-to-prescale formula, its mode registers, and the servo pulse mapping,
and its driver runs the chip over I2C. Source: the PCA9685 datasheet, on the
[hardware page](hardware.md#pca9685).

## ADC

An analog input turns a voltage into a number, and three figures describe it:
the resolution in bits, the reference voltage that the top count corresponds
to, and the rate in samples per second. A microcontroller has a few channels on
chip; the RP2040's is 12-bit at 500 ksps and the ESP32-C3 has two 12-bit
converters. A Raspberry Pi has none, which is what a chip such as the ADS1115
is for: 16 bits, a programmable gain that sets the input range, and up to four
inputs, read over I2C.

What to get right: the input range, since a signal above the reference clips
silently; the gain, which changes what one count means; and the calibration
from counts to the quantity measured, which is the maker's own two points, as
the [own device guide](guides/device.md) shows for a soil probe.

pamoja: [`pamoja-sensors`](guides/sensors.md) carries the ADS1115's
configuration, conversion time, and sample decoding, and its driver reads the
chip over I2C; the calibration helpers are in [`pamoja-kit`](guides/kit.md).
Source: the ADS1115 datasheet, on the [hardware page](hardware.md#ads1115).

## RS-485 and Modbus

A UART's framing carried on a balanced pair instead of two single-ended wires,
which is what lets it run hundreds of meters through an electrically noisy
plant. The design guide states the standard's figures: a driver puts at least
1.5 V across the pair, a receiver detects 200 mV, noise couples into both wires
alike and cancels, the cable is twisted pair with a characteristic impedance of
120 ohms, and the trunk is terminated with a 120 ohm resistor at each end.
Standard drivers carry 32 unit loads; transceivers rated at a fraction of a
unit load allow more nodes on one bus. The bus is a daisy chain with short
stubs, and one node transmits at a time, so a driver enable line goes with the
data.

Modbus RTU is the protocol most field instruments speak over it: one byte of
address, one of function code, the data, and a CRC-16, with a silence of three
and a half characters marking each frame's edges. A Linux board or a
microcontroller joins the bus through a transceiver on a UART, or a USB
adapter that carries one.

pamoja: [`pamoja-modbus`](guides/modbus.md) carries the CRC, the RTU frame,
the standard requests, and response decoding. Sources: TI's RS-485 design
guide, [SLLA272](https://www.ti.com/lit/an/slla272d/slla272d.pdf), and the
[Modbus over serial line specification](https://www.modbus.org/file/secure/modbusoverserial.pdf),
V1.02, which fixes the addresses at 1 to 247 with 0 as broadcast, and the frame
at 256 bytes.

## CAN

A balanced pair like RS-485, terminated at both ends, on which every node may
transmit and the identifier at the head of each frame settles who wins when two
start together: the lower identifier carries on and the other backs off, with
no collision and no lost frame. Classic CAN carries up to eight bytes at up to
1 Mbit/s under an 11-bit or a 29-bit identifier; CAN FD carries up to 64 bytes
and switches to a faster rate for the data. On a truck or a tractor, J1939 gives
the 29-bit identifier a structure of priority, message, and source address.

What to get right: the bit rate every node shares; the termination, once per
end; and the transceiver, since a microcontroller's CAN controller, the
ESP32-C3's TWAI among them, speaks logic levels and needs a transceiver chip to
drive the pair, and a Raspberry Pi or a Pico needs a controller and transceiver
together, such as an MCP2515 over SPI.

pamoja: [`pamoja-can`](guides/can.md) carries classic and FD frames with both
identifier widths, the FD length encoding, and J1939 identifier decoding.
Sources: [ISO 11898-1:2024](https://www.iso.org/standard/86384.html), whose
own text is paid, and the SAE J1939 document set under
[J1939_202603](https://www.sae.org/standards/content/j1939_202603/).

## LoRa

A radio rather than a bus: a chirp spread-spectrum modulation that trades speed
for range and sensitivity, so a node reaches kilometers on milliwatts and sends
a few bytes at a time. Three settings fix the trade: the spreading factor, the
bandwidth, and the coding rate, and together with the payload length they fix
the time each packet occupies the air. In most regions a node may hold the
channel only a small fraction of the time, so the airtime figure is also a rule
to keep. LoRaWAN adds the network layer on top: device addresses, keys, a
message integrity code, encryption, join procedures, and regional channel
plans.

What to get right: the airtime of each packet and the wait it obliges before
the next; the region's plan; and the keys, since a frame with a wrong integrity
code is dropped without a word.

The antenna, the feed line, the connectors, and the power a region allows are on
[Radios and antennas](radio.md).

pamoja: [`pamoja-lora`](guides/lora.md) carries exact time-on-air and
duty-cycle off-time; [`pamoja-lorawan`](guides/lorawan.md) carries the MAC
framing, the integrity code, the encryption, the join, and the regional plans.
Sources: the Semtech SX1276 and SX1262 cards on the
[hardware page](hardware.md#radios-and-long-range-links), and the LoRa Alliance
[RP002-1.0.5 regional parameters](https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters).
