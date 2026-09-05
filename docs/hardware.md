# Hardware

pamoja is a software library, so it runs on whatever its host runs on. This page is narrower
and more useful than that: it lists the parts the drivers were written against, the buses and
protocols the crates implement, the radios they budget airtime for, and the boards the project
builds and tests on.

Nothing here is a compatibility promise. It is a record of what the code was written from, so
you can tell at a glance whether a part you already have is one pamoja decodes byte for byte,
one it reaches over a bus it speaks, or one you will be writing a driver for.

**Where the figures come from.** Every row links one document: the manufacturer's own datasheet
or product page, or the standard that defines the thing. The figures in the row come from that
document and nowhere else. There are no distributor listings, datasheet mirrors, or tutorial
sites in this table, and `cargo xtask links` fetches every link so a rotted one fails the build
rather than sitting there looking authoritative. The exception is the handful of vendors whose
sites refuse any scripted client; those entries say so in the data file, and a person opens
them instead.

**What the cost column means.** A coarse band for a typical breakout module or board in USD, to
tell a two dollar sensor from a two hundred dollar autopilot. It is not a quote, it is not a
1000-unit chip price, and it is not tracked against any vendor.

**How this page stays honest.** The entries are tied to the code. Adding a driver module under
`pamoja-sensors` or `pamoja-actuators` without an entry here fails `cargo xtask docs --check`,
and so does adding a LoRaWAN channel plan the page does not list.

<!-- table: hardware -->
### Sensors

Parts pamoja decodes byte for byte, each with a driver written from the datasheet linked beside it.

| Part | Interface | Cost | Source |
| --- | --- | --- | --- |
| [BME280](#bme280) | I2C up to 3.4 MHz, or SPI up to 10 MHz | $5 to $20 | [Bosch BST-BME280-DS002](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf) |
| [DS18B20](#ds18b20) | 1-Wire, many devices on one pin | under $5 | [ADI 19-7487 Rev 6](https://www.analog.com/media/en/technical-documentation/data-sheets/ds18b20.pdf) |
| [INA219](#ina219) | I2C, 16 addresses from two pins | $5 to $20 | [TI SBOS448G](https://www.ti.com/lit/ds/symlink/ina219.pdf) |
| [ADS1115](#ads1115) | I2C, four addresses from one pin | $5 to $20 | [TI SBAS444E](https://www.ti.com/lit/ds/symlink/ads1115.pdf) |

#### BME280 {#bme280}

**Bosch Sensortec.** Reports humidity, pressure and temperature as raw counts plus per-chip calibration coefficients, which the driver compensates.

- Ranges: 0 to 100 %RH, 300 to 1100 hPa, -40 to +85 °C
- Accuracy: ±3 %RH from 20 to 80 %RH at 25 °C, ±1.0 hPa, ±0.5 °C from 0 to 65 °C
- Addresses: 0x76 with SDO to ground, 0x77 with SDO to VDDIO; SDO must not float
- Supply: 1.71 to 3.6 V main, 1.2 to 3.6 V interface
- Current: 3.6 µA at 1 Hz for all three, 0.1 µA asleep

From [BME280 combined humidity and pressure sensor data sheet, BST-BME280-DS002](https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf).

#### DS18B20 {#ds18b20}

**Analog Devices, originally Maxim Integrated.** A digital thermometer that returns Celsius over a single data line, each part carrying its own 64-bit serial code so a bus can hold many.

- Range: -55 °C to +125 °C
- Accuracy: ±0.5 °C from -10 °C to +85 °C
- Resolution: 9 to 12 bits, user programmable
- Conversion: 750 ms maximum at 12 bits, halving with each bit dropped
- Supply: 3.0 to 5.5 V, or parasite power from the data line

From [DS18B20 Programmable Resolution 1-Wire Digital Thermometer data sheet, 19-7487 Rev 6](https://www.analog.com/media/en/technical-documentation/data-sheets/ds18b20.pdf).

#### INA219 {#ina219}

**Texas Instruments.** Measures the drop across an external shunt and the bus voltage, and reports current and power once its calibration register is set.

- Bus voltage: senses 0 to 26 V
- Shunt full scale: ±40, ±80, ±160 or ±320 mV by PGA setting
- ADC: 12-bit, selectable down to 9-bit or averaged
- Registers: 10 µV per shunt count, 4 mV per bus count
- Supply: 3 to 5.5 V, 0.7 mA typical

From [INA219 Zero-Drift, Bidirectional Current/Power Monitor With I2C Interface, SBOS448G](https://www.ti.com/lit/ds/symlink/ina219.pdf).

#### ADS1115 {#ads1115}

**Texas Instruments.** A 16-bit delta-sigma ADC that digitises four single-ended or two differential inputs through a programmable gain amplifier.

- Resolution: 16 bits
- Inputs: four single-ended or two differential
- Full scale: ±0.256 V to ±6.144 V in six PGA steps
- Rate: 8 to 860 samples per second
- Supply: 2.0 to 5.5 V, 150 µA in continuous conversion

From [ADS111x Ultra-Small, Low-Power, I2C-Compatible, 860SPS, 16-Bit ADCs, SBAS444E](https://www.ti.com/lit/ds/symlink/ads1115.pdf).

### Actuators

Parts pamoja drives: a PWM generator for servos, and the step-and-direction carriers a stepper sequence walks.

| Part | Interface | Cost | Source |
| --- | --- | --- | --- |
| [PCA9685](#pca9685) | I2C, up to 62 devices per bus | $5 to $20 | [NXP PCA9685 Rev. 4](https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf) |
| [ULN2003A](#uln2003) | seven logic inputs, one per channel | under $5 | [TI ULN2003A](https://www.ti.com/product/ULN2003A) |
| [A4988](#a4988) | step and direction, with three mode pins | under $5 | [Allegro A4988 Rev. 8](https://www.allegromicro.com/-/media/files/datasheets/a4988-datasheet.pdf) |
| [DRV8825](#drv8825) | step and direction, with three mode pins | $5 to $20 | [TI SLVSA73F](https://www.ti.com/lit/ds/symlink/drv8825.pdf) |

#### PCA9685 {#pca9685}

**NXP Semiconductors.** A 16-channel PWM generator that produces servo and dimming pulses; each output sinks 25 mA, so load current is switched by an external driver.

- Channels: 16, all sharing one frequency
- Resolution: 12-bit, 4096 steps per output
- Frequency: typically 24 Hz to 1526 Hz, 200 Hz at reset
- Supply: 2.3 to 5.5 V, inputs and outputs 5.5 V tolerant
- Drive: sinks 25 mA, sources 10 mA at 5 V; larger loads need external drivers

From [PCA9685 16-channel, 12-bit PWM Fm+ I2C-bus LED controller, Rev. 4](https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf).

#### ULN2003A {#uln2003}

**Texas Instruments.** A Darlington array that switches the coil current directly, for the four-wire steppers the coil sequencer walks a pattern across.

- Channels: seven NPN Darlington pairs
- Collector current: 500 mA rated, single output
- Output voltage: 50 V maximum
- Inductive loads: common-cathode output clamp diodes included

From [ULN2003A product page, high-voltage high-current Darlington transistor array](https://www.ti.com/product/ULN2003A).

#### A4988 {#a4988}

**Allegro MicroSystems.** A bipolar stepper driver that turns one pulse on STEP into one microstep, sequencing the coils and regulating current in hardware.

- Motor supply: 8 to 35 V
- Output current: ±2 A maximum
- Steps: full, 1/2, 1/4, 1/8 and 1/16
- Logic supply: 3 to 5.5 V
- The datasheet states no continuous per-phase current against a cooling condition

From [A4988 DMOS Microstepping Driver with Translator and Overcurrent Protection, Rev. 8](https://www.allegromicro.com/-/media/files/datasheets/a4988-datasheet.pdf).

#### DRV8825 {#drv8825}

**Texas Instruments.** A bipolar stepper driver with two H-bridges and an indexer, taking step and direction down to 1/32 step.

- Motor supply: 8.2 to 45 V
- Output current: up to 2.5 A per output at 24 V and 25 °C, with heat sinking
- Steps: full, 1/2, 1/4, 1/8, 1/16 and 1/32
- Decay: slow, fast or mixed
- Protection: overcurrent, thermal shutdown and undervoltage lockout, reported on nFAULT

From [DRV8825 Stepper Motor Controller IC, SLVSA73F](https://www.ti.com/lit/ds/symlink/drv8825.pdf).

### Buses and field protocols

The wires between a gateway and the parts above, and the framing each one carries.

| Part | Interface | Cost | Source |
| --- | --- | --- | --- |
| [I2C](#i2c) | two open-drain lines, SDA and SCL | not applicable | [NXP UM10204](https://www.nxp.com/docs/en/user-guide/UM10204.pdf) |
| [SPI](#spi) | four wires: data in, data out, clock, select | not applicable | [Microchip DS70005185A](https://ww1.microchip.com/downloads/en/DeviceDoc/70005185a.pdf) |
| [Modbus RTU over RS-485](#modbus-rtu) | RS-485 balanced pair, half duplex | not applicable | [Modbus.org V1.02](https://www.modbus.org/file/secure/modbusoverserial.pdf) |
| [CAN 2.0 and CAN FD](#can) | differential bus | not applicable | [ISO 11898-1:2024](https://www.iso.org/standard/86384.html) |
| [SAE J1939](#j1939) | CAN | not applicable | [SAE J1939](https://www.sae.org/standards/content/j1939_202603/) |

#### I2C {#i2c}

**NXP Semiconductors.** The two-wire addressable bus that hangs sensors and peripherals off one controller; the address a chip answers on is what the gpio crate carries.

- Rates: 100 kbit/s standard, 400 kbit/s fast, 1 Mbit/s fast-mode plus, 3.4 Mbit/s high speed
- Addressing: 7-bit and 10-bit targets, which may share a bus in every speed mode
- Lines: SDA and SCL, both bidirectional, pulled up and driven open-drain for wired-AND
- Loading: bus capacitance, not a device count, limits how many hang on it

From [UM10204 I2C-bus specification and user manual, Rev. 7.0](https://www.nxp.com/docs/en/user-guide/UM10204.pdf).

#### SPI {#spi}

**No standards body; described from Microchip's reference manual.** A full-duplex synchronous link between a controller and one selected peripheral at a time; no standard defines it, so pin names, timing and rate come from each device's own datasheet.

- Signals: serial data in, serial data out, a shift clock, and an active-low select
- Clock: driven by the controller, and only while there is data to move
- Modes: four combinations of clock polarity and clock edge, any of which may be chosen
- Rate: set by the controller's clock prescalers; no standard fixes one
- Words: 8 or 16 bits per transfer

From [Serial Peripheral Interface (SPI), dsPIC33/PIC24 Family Reference Manual, DS70005185A](https://ww1.microchip.com/downloads/en/DeviceDoc/70005185a.pdf).

#### Modbus RTU over RS-485 {#modbus-rtu}

**Modbus Organization.** Request and reply in binary frames over a two-wire trunk, the way meters, drives and PLCs are wired along one cable.

- Physical layer: a two-wire EIA/TIA-485 interface, terminated at both ends of the trunk
- Devices: 32 on a segment without a repeater
- Addresses: 1 to 247, with 0 as broadcast
- Rates: 9600 and 19200 bit/s required, 19200 the default; 1200 to 115200 optional
- Length: 1000 m at 9600 baud on AWG26 or heavier; drops no more than 20 m
- Framing: address, function, data and a 16-bit CRC in at most 256 bytes; frames separated by 3.5 character times, and a gap over 1.5 discards one

From [MODBUS over Serial Line Specification and Implementation Guide V1.02](https://www.modbus.org/file/secure/modbusoverserial.pdf).

#### CAN 2.0 and CAN FD {#can}

**ISO.** The multi-master vehicle bus whose identifier both labels a message and arbitrates access. The standard itself is paid, so the figures below are what its catalogue page states and what the crate implements.

- Document: ISO 11898-1:2024, edition 3, published May 2024
- Scope: the data link layer and the physical coding sublayer
- pamoja-can: standard 11-bit and extended 29-bit identifiers, always masked to their width
- pamoja-can: classic frames up to 8 bytes, and CAN FD frames up to 64 bytes at the discrete FD lengths

From [ISO 11898-1:2024 Road vehicles, Controller area network (CAN), Part 1: Data link layer and physical coding sublayer](https://www.iso.org/standard/86384.html).

#### SAE J1939 {#j1939}

**SAE International.** The higher-layer protocol trucks, buses and off-highway machines run over CAN, defined as a set of SAE documents under one top-level recommended practice.

- Top-level document: J1939_202603, revised March 2026, first issued April 2000
- Structure: the top-level document describes the subordinate documents and defines the terms they share
- Committee: Truck and Bus Control and Communications Network

From [SAE J1939 document set, top-level document J1939_202603](https://www.sae.org/standards/content/j1939_202603/).

### Radios and long-range links

Reaching a network that is not there: the transceivers, the channel plans they must obey, and the short-range meshes.

| Part | Interface | Cost | Source |
| --- | --- | --- | --- |
| [SX1276](#sx1276) | SPI | $5 to $20 | [Semtech SX1276](https://www.semtech.com/products/wireless-rf/lora-connect/sx1276) |
| [SX1262](#sx1262) | SPI | $5 to $20 | [Semtech SX1262](https://www.semtech.com/products/wireless-rf/lora-connect/sx1262) |
| [LoRaWAN Regional Parameters](#lorawan-rp002) | LoRaWAN over sub-GHz LoRa | not applicable | [LoRa Alliance RP002-1.0.5](https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters) |
| [ESP-NOW](#esp-now) | Wi-Fi PHY, vendor-specific action frames | $5 to $20 | [Espressif ESP-IDF](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/network/esp_now.html) |

#### SX1276 {#sx1276}

**Semtech.** The sub-GHz LoRa and FSK transceiver on most 137 to 1020 MHz end devices, and the radio a pamoja airtime budget is computed for.

- Coverage: 137 MHz to 1020 MHz
- Modulation: LoRa, FSK, GFSK, MSK, GMSK and OOK
- Transmit power: +20 dBm at 100 mW, with a separate +14 dBm high efficiency amplifier
- Sensitivity: down to -148 dBm, for a maximum link budget of 168 dB
- Receive current: 9.9 mA, with 200 nA register retention

From [SX1276 product page, Semtech LoRa Connect](https://www.semtech.com/products/wireless-rf/lora-connect/sx1276).

#### SX1262 {#sx1262}

**Semtech.** The later sub-GHz LoRa transceiver, covering 150 to 960 MHz with a higher power amplifier and a much lower receive current.

- Coverage: 150 MHz to 960 MHz, continuous
- Modulation: LoRa, LR-FHSS, FSK, GFSK, MSK and GMSK
- Transmit power: +22 dBm high efficiency amplifier
- Sensitivity: down to -148 dBm, for a maximum link budget of 170 dB
- Receive current: 4.6 mA, with an integrated DC-DC converter and LDO

From [SX1262 product page, Semtech LoRa Connect](https://www.semtech.com/products/wireless-rf/lora-connect/sx1262).

#### LoRaWAN Regional Parameters {#lorawan-rp002}

**LoRa Alliance.** The document that fixes the channel plan, data rates, transmit limits and maximum payload a LoRaWAN device must obey in each regulatory region.

- Revision cited: RP002-1.0.5, October 2025, Final
- Fixes per region: channel frequencies, data rates, output power encoding, join CFList, receive windows and maximum payload
- Limits: duty cycle in the dynamic-plan regions, dwell time where frequency-hopping rules apply
- EU863-870: at least 24 stored channels, with 868.1, 868.3 and 868.5 MHz at DR0 to DR5 mandatory
- pamoja implements nine of its plans: EU863-870, US902-928, EU433, AU915-928, CN470-510, AS923, KR920-923, IN865-867 and RU864-870

From [RP002-1.0.5 LoRaWAN Regional Parameters, LoRa Alliance Technical Committee, October 2025](https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters).

#### ESP-NOW {#esp-now}

**Espressif Systems.** Espressif's connectionless protocol: short packets straight between Wi-Fi devices with no access point and no association step.

- Payload: 250 bytes on v1.0 devices, 1470 bytes on v2.0
- Default bit rate: 1 Mbps
- Peers: 20 paired devices, of which at most 17 encrypted, 7 by default
- Security: CCMP per IEEE 802.11-2012, with a 16-byte primary master key and per-peer local keys
- Interfaces: sends over either the station or the SoftAP interface

From [ESP-NOW, ESP-IDF Programming Guide, stable, ESP32](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/network/esp_now.html).

### Boards, targets and autopilots

Where pamoja itself runs, and what it has been built and tested against.

| Part | Interface | Cost | Source |
| --- | --- | --- | --- |
| [Raspberry Pi 5](#raspberry-pi-5) | - | $60 to $200 | [Raspberry Pi 5](https://www.raspberrypi.com/products/raspberry-pi-5/) |
| [Raspberry Pi Zero 2 W](#raspberry-pi-zero-2-w) | - | $5 to $20 | [Raspberry Pi Zero 2 W](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/) |
| [ESP32](#esp32) | - | $5 to $20 | [Espressif ESP32 datasheet](https://documentation.espressif.com/esp32_datasheet_en.pdf) |
| [ArduPilot](#ardupilot) | MAVLink | $60 to $200 | [ArduPilot](https://ardupilot.org/dev/index.html) |
| [PX4 Autopilot](#px4) | MAVLink | $60 to $200 | [PX4](https://docs.px4.io/main/en/) |
| [Pixhawk standard](#pixhawk) | standardised connectors and pin-outs | $60 to $200 | [Pixhawk standards](https://pixhawk.org/standards/) |
| [Cortex-M4 with FPU, bare metal](#cortex-m4f) | - | $5 to $20 | [Arm Cortex-M4](https://www.arm.com/products/silicon-ip-cpu/cortex-m/cortex-m4) |

#### Raspberry Pi 5 {#raspberry-pi-5}

**Raspberry Pi Ltd.** The gateway-class Linux board: the full std build, every transport, and the dashboard.

- SoC: Broadcom BCM2712, quad-core 64-bit Arm Cortex-A76 at 2.4 GHz
- Memory: LPDDR4X, in 1, 2, 4, 8 and 16 GB
- Connectivity: gigabit Ethernet with PoE+ via a HAT, dual-band 802.11ac, Bluetooth 5.0
- I/O: the standard 40-pin header, two USB 3.0 and two USB 2.0 ports, a UART debug port
- The page lists the 16 GB model at $305, above the band for the smaller ones

From [Raspberry Pi 5 product page](https://www.raspberrypi.com/products/raspberry-pi-5/).

#### Raspberry Pi Zero 2 W {#raspberry-pi-zero-2-w}

**Raspberry Pi Ltd.** The small, low-cost Linux board, running the same std build as the Pi 5 on the same 40-pin footprint.

- SoC: RP3A0, quad-core 64-bit Arm Cortex-A53 at 1 GHz
- Memory: 512 MB
- Wireless: 2.4 GHz 802.11 b/g/n, Bluetooth 4.2 and BLE, onboard antenna
- I/O: HAT-compatible 40-pin header footprint, unpopulated; micro USB OTG

From [Raspberry Pi Zero 2 W product page](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/).

#### ESP32 {#esp32}

**Espressif Systems.** The Wi-Fi and Bluetooth microcontroller behind most small sensor nodes, with the buses above and a CAN controller on chip.

- CPU: Xtensa 32-bit LX6, single or dual core, up to 240 MHz
- Memory: 448 KB ROM, 520 KB SRAM, 16 KB RTC SRAM
- Wireless: 802.11 b/g/n up to 150 Mbit/s, Bluetooth 4.2 BR/EDR and LE
- Peripherals: four SPI, two I2C, three UART, 34 GPIO
- CAN: a TWAI controller compatible with ISO 11898-1
- pamoja: not a CI target; Xtensa needs Espressif's own Rust toolchain

From [ESP32 Series Datasheet, version 5.3](https://documentation.espressif.com/esp32_datasheet_en.pdf).

#### ArduPilot {#ardupilot}

**ArduPilot Dev Team.** Open autopilot firmware for copters, planes, rovers and submarines; the firmware is free, and the flight controller it runs on is what costs money.

- Vehicles: multicopters, helicopters, fixed wing, rovers, submarines and antenna trackers
- Protocol: MAVLink between ground station, flight controller and peripherals
- Licence: GPL v3 or later
- Verified: the SITL interop job in CI flies it in simulation against pamoja-mavlink

From [ArduPilot development site](https://ardupilot.org/dev/index.html).

#### PX4 Autopilot {#px4}

**Dronecode Foundation.** Open autopilot firmware for multirotors, fixed wing, VTOL, helicopters and rovers, under a permissive licence; again, the board is the cost.

- Vehicles: multirotors, fixed wing, VTOL, helicopters and rovers
- Protocol: MAVLink for the ground station link
- Licence: BSD 3-clause
- Verified: the SITL interop job in CI flies it in simulation against pamoja-mavlink

From [PX4 Autopilot User Guide](https://docs.px4.io/main/en/).

#### Pixhawk standard {#pixhawk}

**Dronecode Foundation.** The open hardware standard that fixes flight controller pin-outs, connectors and layout so boards from different vendors interchange; a board built to it is what costs money.

- Autopilot standards listed: v5X (DS-011), v6X (DS-012), v6U (DS-016) and v6C (DS-018)
- Also: the connector standard (DS-009), the autopilot bus standard (DS-010) and the payload bus standard (DS-014); a smart battery standard is in draft
- Each fixes pin-outs, block diagrams, PCB layout guidelines and connector specifications

From [Pixhawk Reference Standards](https://pixhawk.org/standards/).

#### Cortex-M4 with FPU, bare metal {#cortex-m4f}

**Arm.** The bare-metal target: CI cross-compiles every no_std crate for thumbv7em-none-eabihf, which is this core with its floating-point unit and no operating system.

- Architecture: Armv7-M
- Floating point: a built-in single-precision FPU, credited with 10x on single-precision operations
- DSP: integrated DSP, SIMD and MAC instructions
- Use cases the page names: industrial control, IoT sensors, AI and ML, audio
- pamoja: the bare-metal job in CI builds pamoja-core, security, audit, update, zenoh, ros2 and mavlink for this target

From [Cortex-M4 product page, Arm](https://www.arm.com/products/silicon-ip-cpu/cortex-m/cortex-m4).
<!-- end -->

## Anything else with a driver

The four sensors and two actuator families above are the parts pamoja decodes itself. They are
not the limit of what it can talk to. Everything below the driver is a trait: implement
[`Sensor`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/trait.Sensor.html) or
[`Actuator`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/trait.Actuator.html) for
your own part and the rest of the library, the profiles, the ladder, the dashboard, treats it
exactly like the ones here. The [sensor drivers guide](guides/sensors.md) and the
[actuator drivers guide](guides/actuators.md) show what that takes.

The same holds for transports. A part reachable over I2C, SPI, a serial line, RS485, or CAN is
reachable through the crates listed under Buses, whether or not its decoder ships here.
