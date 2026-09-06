# Hardware

pamoja is a software library, so it runs on whatever its host runs on. This page is narrower
and more useful than that: it lists the parts the drivers were written against, the buses and
protocols the crates implement, the radios they budget airtime for, and the boards the project
builds and tests on.

Nothing here is a compatibility promise. It is a record of what the code was written from, so
you can tell at a glance whether a part you already have is one pamoja decodes byte for byte,
one it reaches over a bus it speaks, or one you will be writing a driver for.

**Where the figures come from.** Every part links one document under "Read and build": the
manufacturer's own datasheet, or the standard that defines the thing, or the maker's own
documentation for a board. The figures on the card come from that document and nowhere else.
There are no distributor listings, datasheet mirrors, or tutorial sites among them, and
`cargo xtask links` fetches every one so a rotted link fails the build rather than sitting there
looking authoritative. The exception is the handful of vendors whose
sites refuse any scripted client; those entries say so in the data file, and a person opens
them instead.

**What the cost line means.** A coarse band for a typical breakout module or board in USD, to
tell a two dollar sensor from a two hundred dollar autopilot. It is not a quote, it is not a
1000-unit chip price, and it is not tracked against any vendor.

**Where to buy.** Each part lists two or three product pages from the makers' own stores and
the larger distributors, the cheapest reputable option first, with the price each page listed
on the day it was read. Those are places to buy, not sources: nothing on a card is taken from
them. A vendor whose site refuses scripted readers is priced from a listing of its page, and
the card says so.

**How this page stays honest.** The entries are tied to the code. Adding a driver module under
`pamoja-sensors` or `pamoja-actuators` without an entry here fails `cargo xtask docs --check`,
and so does adding a LoRaWAN channel plan the page does not list.

<!-- table: hardware -->
### Sensors

Parts pamoja decodes byte for byte, each with a driver written from the datasheet linked beside it.

<div class="hw-cards">
<article class="hw-card" id="bme280">
<div class="hw-head"><h4>BME280</h4><p class="hw-by">Bosch Sensortec</p><p class="hw-summary">Reports humidity, pressure and temperature as raw counts plus per-chip calibration coefficients, which the driver compensates.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C up to 3.4 MHz, or SPI up to 10 MHz</dd></div><div><dt>Ranges</dt><dd>0 to 100 %RH, 300 to 1100 hPa, -40 to +85 °C</dd></div><div><dt>Accuracy</dt><dd>±3 %RH from 20 to 80 %RH at 25 °C, ±1.0 hPa, ±0.5 °C from 0 to 65 °C</dd></div><div><dt>Addresses</dt><dd>0x76 with SDO to ground, 0x77 with SDO to VDDIO; SDO must not float</dd></div><div><dt>Supply</dt><dd>1.71 to 3.6 V main, 1.2 to 3.6 V interface</dd></div><div><dt>Current</dt><dd>3.6 µA at 1 Hz for all three, 0.1 µA asleep</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.adafruit.com/product/2652">US$14.95</a> at Adafruit</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.adafruit.com/product/2652"><b>Adafruit</b><span class="hw-price">US$14.95</span><small>Adafruit BME280 I2C or SPI Temperature Humidity Pressure Sensor - STEMMA QT</small></a></li><li><a class="hw-offer" href="https://shop.pimoroni.com/products/bme280-breakout"><b>Pimoroni</b><span class="hw-price">£11.50</span><small>BME280 Breakout - Temperature, Pressure, Humidity Sensor</small></a></li><li><a class="hw-offer" href="https://www.sparkfun.com/sparkfun-atmospheric-sensor-breakout-bme280-qwiic.html"><b>SparkFun</b><span class="hw-price">US$16.95</span><small>SparkFun Atmospheric Sensor Breakout - BME280 (Qwiic)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf">Datasheet <small>Bosch BST-BME280-DS002</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/bme280.rs">Driver source <code>bme280.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html">Crate <code>pamoja-sensors</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="ds18b20">
<div class="hw-head"><h4>DS18B20</h4><p class="hw-by">Analog Devices, originally Maxim Integrated</p><p class="hw-summary">A digital thermometer that returns Celsius over a single data line, each part carrying its own 64-bit serial code so a bus can hold many.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>1-Wire, many devices on one pin</dd></div><div><dt>Range</dt><dd>-55 °C to +125 °C</dd></div><div><dt>Accuracy</dt><dd>±0.5 °C from -10 °C to +85 °C</dd></div><div><dt>Resolution</dt><dd>9 to 12 bits, user programmable</dd></div><div><dt>Conversion</dt><dd>750 ms maximum at 12 bits, halving with each bit dropped</dd></div><div><dt>Supply</dt><dd>3.0 to 5.5 V, or parasite power from the data line</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board; the lowest listed price is <a href="https://www.adafruit.com/product/374">US$3.95</a> at Adafruit</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.adafruit.com/product/374"><b>Adafruit</b><span class="hw-price">US$3.95</span><small>DS18B20 Digital temperature sensor + extras</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/ds18b20-one-wire-digital-temperature-sensor"><b>The Pi Hut</b><span class="hw-price">£7.00</span><small>DS18B20+ One Wire Digital Temperature Sensor</small></a></li><li><a class="hw-offer" href="https://www.sparkfun.com/temperature-sensor-waterproof-ds18b20.html"><b>SparkFun</b><span class="hw-price">US$10.95</span><small>Temperature Sensor - Waterproof (DS18B20)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.analog.com/media/en/technical-documentation/data-sheets/ds18b20.pdf">Datasheet <small>ADI 19-7487 Rev 6</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ds18b20.rs">Driver source <code>ds18b20.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html">Crate <code>pamoja-sensors</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="ina219">
<div class="hw-head"><h4>INA219</h4><p class="hw-by">Texas Instruments</p><p class="hw-summary">Measures the drop across an external shunt and the bus voltage, and reports current and power once its calibration register is set.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, 16 addresses from two pins</dd></div><div><dt>Bus voltage</dt><dd>senses 0 to 26 V</dd></div><div><dt>Shunt full scale</dt><dd>±40, ±80, ±160 or ±320 mV by PGA setting</dd></div><div><dt>ADC</dt><dd>12-bit, selectable down to 9-bit or averaged</dd></div><div><dt>Registers</dt><dd>10 µV per shunt count, 4 mV per bus count</dd></div><div><dt>Supply</dt><dd>3 to 5.5 V, 0.7 mA typical</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.dfrobot.com/product-1827.html">US$6.90</a> at DFRobot</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.dfrobot.com/product-1827.html"><b>DFRobot</b><span class="hw-price">US$6.90</span><small>Gravity: I2C Digital Wattmeter</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/904"><b>Adafruit</b><span class="hw-price">US$9.95</span><small>INA219 High Side DC Current Sensor Breakout - 26V ±3.2A Max - STEMMA QT</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/adafruit-ina219-high-side-dc-current-sensor-breakout-26v-3-2a-max"><b>The Pi Hut</b><span class="hw-price">£9.60</span><small>INA219 High Side DC Current Sensor Breakout - 26V ±3.2A Max (STEMMA QT)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.ti.com/lit/ds/symlink/ina219.pdf">Datasheet <small>TI SBOS448G</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ina219.rs">Driver source <code>ina219.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html">Crate <code>pamoja-sensors</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="ads1115">
<div class="hw-head"><h4>ADS1115</h4><p class="hw-by">Texas Instruments</p><p class="hw-summary">A 16-bit delta-sigma ADC that digitises four single-ended or two differential inputs through a programmable gain amplifier.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, four addresses from one pin</dd></div><div><dt>Resolution</dt><dd>16 bits</dd></div><div><dt>Inputs</dt><dd>four single-ended or two differential</dd></div><div><dt>Full scale</dt><dd>±0.256 V to ±6.144 V in six PGA steps</dd></div><div><dt>Rate</dt><dd>8 to 860 samples per second</dd></div><div><dt>Supply</dt><dd>2.0 to 5.5 V, 150 µA in continuous conversion</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.dfrobot.com/product-1730.html">US$8.90</a> at DFRobot</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.dfrobot.com/product-1730.html"><b>DFRobot</b><span class="hw-price">US$8.90</span><small>Gravity: I2C ADS1115 16-Bit ADC Module</small></a></li><li><a class="hw-offer" href="https://www.seeedstudio.com/Grove-ADS1115-16-bit-ADC-p-4599.html"><b>Seeed Studio</b><span class="hw-price">US$12.00</span><small>Grove - 4 Channel 16-bit ADC (ADS1115) with Programmable Amplifier Gain</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/1085"><b>Adafruit</b><span class="hw-price">US$14.95</span><small>ADS1115 16-Bit ADC - 4 Channel with Programmable Gain Amplifier - STEMMA QT / Qwiic</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.ti.com/lit/ds/symlink/ads1115.pdf">Datasheet <small>TI SBAS444E</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ads1115.rs">Driver source <code>ads1115.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html">Crate <code>pamoja-sensors</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers guide</a></li></ul></section>
</div>
</article>
</div>

### Actuators

Parts pamoja drives: a PWM generator for servos, and the step-and-direction carriers a stepper sequence walks.

<div class="hw-cards">
<article class="hw-card" id="pca9685">
<div class="hw-head"><h4>PCA9685</h4><p class="hw-by">NXP Semiconductors</p><p class="hw-summary">A 16-channel PWM generator that produces servo and dimming pulses; each output sinks 25 mA, so load current is switched by an external driver.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, up to 62 devices per bus</dd></div><div><dt>Channels</dt><dd>16, all sharing one frequency</dd></div><div><dt>Resolution</dt><dd>12-bit, 4096 steps per output</dd></div><div><dt>Frequency</dt><dd>typically 24 Hz to 1526 Hz, 200 Hz at reset</dd></div><div><dt>Supply</dt><dd>2.3 to 5.5 V, inputs and outputs 5.5 V tolerant</dd></div><div><dt>Drive</dt><dd>sinks 25 mA, sources 10 mA at 5 V; larger loads need external drivers</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.sparkfun.com/sparkfun-servo-phat-for-raspberry-pi.html">US$13.95</a> at SparkFun</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.sparkfun.com/sparkfun-servo-phat-for-raspberry-pi.html"><b>SparkFun</b><span class="hw-price">US$13.95</span><small>SparkFun Servo pHAT for Raspberry Pi (DEV-15316)</small></a></li><li><a class="hw-offer" href="https://www.seeedstudio.com/Grove-16-Channel-PWM-Driver-PCA9685.html"><b>Seeed Studio</b><span class="hw-price">US$14.20</span><small>Grove - 16-Channel PWM Driver (PCA9685)</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/815"><b>Adafruit</b><span class="hw-price">US$14.95</span><small>Adafruit 16-Channel 12-bit PWM/Servo Driver - I2C interface</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf">Datasheet <small>NXP PCA9685 Rev. 4</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/pca9685.rs">Driver source <code>pca9685.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html">Crate <code>pamoja-actuators</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="uln2003">
<div class="hw-head"><h4>ULN2003A</h4><p class="hw-by">Texas Instruments</p><p class="hw-summary">A Darlington array that switches the coil current directly, for the four-wire steppers the coil sequencer walks a pattern across.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>seven logic inputs, one per channel</dd></div><div><dt>Channels</dt><dd>seven NPN Darlington pairs</dd></div><div><dt>Collector current</dt><dd>500 mA rated, single output</dd></div><div><dt>Output voltage</dt><dd>50 V maximum</dd></div><div><dt>Inductive loads</dt><dd>common-cathode output clamp diodes included</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board; the lowest listed price is <a href="https://www.seeedstudio.com/Gear-Stepper-Motor-Driver-Pack-p-3200.html">US$6.90</a> at Seeed Studio</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.seeedstudio.com/Gear-Stepper-Motor-Driver-Pack-p-3200.html"><b>Seeed Studio</b><span class="hw-price">US$6.90</span><small>Gear Stepper Motor Driver Pack (ULN2003 driver board with 28BYJ-48 motor)</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/stepper-motor-driver-pack"><b>The Pi Hut</b><span class="hw-price">£6.70</span><small>Stepper Motor Driver Pack (Seeed 105990072)</small></a></li><li><a class="hw-offer" href="https://www.digikey.com/en/products/detail/texas-instruments/ULN2003AN/277624"><b>Digi-Key</b><span class="hw-price">US$0.97</span><small>ULN2003AN, 16-DIP (bare chip)</small><em>from a listing, since the page refuses scripted readers</em></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.ti.com/product/ULN2003A">Datasheet <small>TI ULN2003A</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs">Driver source <code>stepper.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html">Crate <code>pamoja-actuators</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="a4988">
<div class="hw-head"><h4>A4988</h4><p class="hw-by">Allegro MicroSystems</p><p class="hw-summary">A bipolar stepper driver that turns one pulse on STEP into one microstep, sequencing the coils and regulating current in hardware.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>step and direction, with three mode pins</dd></div><div><dt>Motor supply</dt><dd>8 to 35 V</dd></div><div><dt>Output current</dt><dd>±2 A maximum</dd></div><div><dt>Steps</dt><dd>full, 1/2, 1/4, 1/8 and 1/16</dd></div><div><dt>Logic supply</dt><dd>3 to 5.5 V</dd></div><div><dt>Note</dt><dd>The datasheet states no continuous per-phase current against a cooling condition</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board; the lowest listed price is <a href="https://www.adafruit.com/product/6109">US$6.95</a> at Adafruit</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.adafruit.com/product/6109"><b>Adafruit</b><span class="hw-price">US$6.95</span><small>Adafruit A4988 Stepper Motor Driver Breakout Board</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/adafruit-a4988-stepper-motor-driver-breakout-board"><b>The Pi Hut</b><span class="hw-price">£6.70</span><small>Adafruit A4988 Stepper Motor Driver Breakout Board</small></a></li><li><a class="hw-offer" href="https://www.pololu.com/product/1182"><b>Pololu</b><span class="hw-price">US$8.95</span><small>A4988 Stepper Motor Driver Carrier</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.allegromicro.com/-/media/files/datasheets/a4988-datasheet.pdf">Datasheet <small>Allegro A4988 Rev. 8</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs">Driver source <code>stepper.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html">Crate <code>pamoja-actuators</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="drv8825">
<div class="hw-head"><h4>DRV8825</h4><p class="hw-by">Texas Instruments</p><p class="hw-summary">A bipolar stepper driver with two H-bridges and an indexer, taking step and direction down to 1/32 step.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>step and direction, with three mode pins</dd></div><div><dt>Motor supply</dt><dd>8.2 to 45 V</dd></div><div><dt>Output current</dt><dd>up to 2.5 A per output at 24 V and 25 °C, with heat sinking</dd></div><div><dt>Steps</dt><dd>full, 1/2, 1/4, 1/8, 1/16 and 1/32</dd></div><div><dt>Decay</dt><dd>slow, fast or mixed</dd></div><div><dt>Protection</dt><dd>overcurrent, thermal shutdown and undervoltage lockout, reported on nFAULT</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://shop.m5stack.com/products/atomic-stepmotor-base-drv8825">US$8.95</a> at M5Stack</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://shop.m5stack.com/products/atomic-stepmotor-base-drv8825"><b>M5Stack</b><span class="hw-price">US$8.95</span><small>ATOMIC Stepmotor Base (DRV8825)</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/atomic-stepmotor-base-drv8825"><b>The Pi Hut</b><span class="hw-price">£8.70</span><small>ATOMIC Stepmotor Base (DRV8825)</small></a></li><li><a class="hw-offer" href="https://www.pololu.com/product/2133"><b>Pololu</b><span class="hw-price">US$15.95</span><small>DRV8825 Stepper Motor Driver Carrier, High Current</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.ti.com/lit/ds/symlink/drv8825.pdf">Datasheet <small>TI SLVSA73F</small></a></li><li><a class="hw-link" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs">Driver source <code>stepper.rs</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html">Crate <code>pamoja-actuators</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers guide</a></li></ul></section>
</div>
</article>
</div>

### Buses and field protocols

The wires between a gateway and the parts above, and the framing each one carries.

<div class="hw-cards">
<article class="hw-card" id="i2c">
<div class="hw-head"><h4>I2C</h4><p class="hw-by">NXP Semiconductors</p><p class="hw-summary">The two-wire addressable bus that hangs sensors and peripherals off one controller; the address a chip answers on is what the gpio crate carries.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>two open-drain lines, SDA and SCL</dd></div><div><dt>Rates</dt><dd>100 kbit/s standard, 400 kbit/s fast, 1 Mbit/s fast-mode plus, 3.4 Mbit/s high speed</dd></div><div><dt>Addressing</dt><dd>7-bit and 10-bit targets, which may share a bus in every speed mode</dd></div><div><dt>Lines</dt><dd>SDA and SCL, both bidirectional, pulled up and driven open-drain for wired-AND</dd></div><div><dt>Loading</dt><dd>bus capacitance, not a device count, limits how many hang on it</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.nxp.com/docs/en/user-guide/UM10204.pdf">Specification <small>NXP UM10204</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html">Crate <code>pamoja-gpio</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html">Crate <code>pamoja-sensors</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html">Crate <code>pamoja-actuators</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO guide</a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html">Sensor drivers guide</a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html">Actuator drivers guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="spi">
<div class="hw-head"><h4>SPI</h4><p class="hw-by">No standards body; described from Microchip's reference manual</p><p class="hw-summary">A full-duplex synchronous link between a controller and one selected peripheral at a time; no standard defines it, so pin names, timing and rate come from each device's own datasheet.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>four wires: data in, data out, clock, select</dd></div><div><dt>Signals</dt><dd>serial data in, serial data out, a shift clock, and an active-low select</dd></div><div><dt>Clock</dt><dd>driven by the controller, and only while there is data to move</dd></div><div><dt>Modes</dt><dd>four combinations of clock polarity and clock edge, any of which may be chosen</dd></div><div><dt>Rate</dt><dd>set by the controller's clock prescalers; no standard fixes one</dd></div><div><dt>Words</dt><dd>8 or 16 bits per transfer</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://ww1.microchip.com/downloads/en/DeviceDoc/70005185a.pdf">Specification <small>Microchip DS70005185A</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html">Crate <code>pamoja-gpio</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/gpio.html">I2C, SPI, and GPIO guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="modbus-rtu">
<div class="hw-head"><h4>Modbus RTU over RS-485</h4><p class="hw-by">Modbus Organization</p><p class="hw-summary">Request and reply in binary frames over a two-wire trunk, the way meters, drives and PLCs are wired along one cable.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>RS-485 balanced pair, half duplex</dd></div><div><dt>Physical layer</dt><dd>a two-wire EIA/TIA-485 interface, terminated at both ends of the trunk</dd></div><div><dt>Devices</dt><dd>32 on a segment without a repeater</dd></div><div><dt>Addresses</dt><dd>1 to 247, with 0 as broadcast</dd></div><div><dt>Rates</dt><dd>9600 and 19200 bit/s required, 19200 the default; 1200 to 115200 optional</dd></div><div><dt>Length</dt><dd>1000 m at 9600 baud on AWG26 or heavier; drops no more than 20 m</dd></div><div><dt>Framing</dt><dd>address, function, data and a 16-bit CRC in at most 256 bytes; frames separated by 3.5 character times, and a gap over 1.5 discards one</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.modbus.org/file/secure/modbusoverserial.pdf">Specification <small>Modbus.org V1.02</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html">Crate <code>pamoja-modbus</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html">Crate <code>pamoja-serial</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/serial.html">Serial framing guide</a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/modbus.html">Modbus RTU guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="can">
<div class="hw-head"><h4>CAN 2.0 and CAN FD</h4><p class="hw-by">ISO</p><p class="hw-summary">The multi-master vehicle bus whose identifier both labels a message and arbitrates access. The standard itself is paid, so the figures below are what its catalogue page states and what the crate implements.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>differential bus</dd></div><div><dt>Document</dt><dd>ISO 11898-1:2024, edition 3, published May 2024</dd></div><div><dt>Scope</dt><dd>the data link layer and the physical coding sublayer</dd></div><div><dt>pamoja-can</dt><dd>standard 11-bit and extended 29-bit identifiers, always masked to their width</dd></div><div><dt>pamoja-can</dt><dd>classic frames up to 8 bytes, and CAN FD frames up to 64 bytes at the discrete FD lengths</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.iso.org/standard/86384.html">Specification <small>ISO 11898-1:2024</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html">Crate <code>pamoja-can</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939 guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="j1939">
<div class="hw-head"><h4>SAE J1939</h4><p class="hw-by">SAE International</p><p class="hw-summary">The higher-layer protocol trucks, buses and off-highway machines run over CAN, defined as a set of SAE documents under one top-level recommended practice.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>CAN</dd></div><div><dt>Top-level document</dt><dd>J1939_202603, revised March 2026, first issued April 2000</dd></div><div><dt>Structure</dt><dd>the top-level document describes the subordinate documents and defines the terms they share</dd></div><div><dt>Committee</dt><dd>Truck and Bus Control and Communications Network</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.sae.org/standards/content/j1939_202603/">Specification <small>SAE J1939</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html">Crate <code>pamoja-can</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/can.html">CAN and J1939 guide</a></li></ul></section>
</div>
</article>
</div>

### Radios and long-range links

Reaching a network that is not there: the transceivers, the channel plans they must obey, and the short-range meshes.

<div class="hw-cards">
<article class="hw-card" id="sx1276">
<div class="hw-head"><h4>SX1276</h4><p class="hw-by">Semtech</p><p class="hw-summary">The sub-GHz LoRa and FSK transceiver on most 137 to 1020 MHz end devices, and the radio a pamoja airtime budget is computed for.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Coverage</dt><dd>137 MHz to 1020 MHz</dd></div><div><dt>Modulation</dt><dd>LoRa, FSK, GFSK, MSK, GMSK and OOK</dd></div><div><dt>Transmit power</dt><dd>+20 dBm at 100 mW, with a separate +14 dBm high efficiency amplifier</dd></div><div><dt>Sensitivity</dt><dd>down to -148 dBm, for a maximum link budget of 168 dB</dd></div><div><dt>Receive current</dt><dd>9.9 mA, with 200 nA register retention</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://sparkfun.com/products/18085">US$14.95</a> at SparkFun</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://sparkfun.com/products/18085"><b>SparkFun</b><span class="hw-price">US$14.95</span><small>LoRa Transceiver Module (RFM95CW), 137 to 1020 MHz, SX1276 based</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/3072"><b>Adafruit</b><span class="hw-price">US$19.95</span><small>Adafruit RFM95W LoRa Radio Transceiver Breakout - 868 or 915 MHz</small></a></li><li><a class="hw-offer" href="https://shop.pimoroni.com/products/adafruit-rfm95w-lora-radio-transceiver-breakout"><b>Pimoroni</b><span class="hw-price">£16.25</span><small>Adafruit RFM95W LoRa Radio Transceiver Breakout - 868 or 915 MHz (ADA3072)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1276">Datasheet <small>Semtech SX1276</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html">Crate <code>pamoja-lora</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="sx1262">
<div class="hw-head"><h4>SX1262</h4><p class="hw-by">Semtech</p><p class="hw-summary">The later sub-GHz LoRa transceiver, covering 150 to 960 MHz with a higher power amplifier and a much lower receive current.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Coverage</dt><dd>150 MHz to 960 MHz, continuous</dd></div><div><dt>Modulation</dt><dd>LoRa, LR-FHSS, FSK, GFSK, MSK and GMSK</dd></div><div><dt>Transmit power</dt><dd>+22 dBm high efficiency amplifier</dd></div><div><dt>Sensitivity</dt><dd>down to -148 dBm, for a maximum link budget of 170 dB</dd></div><div><dt>Receive current</dt><dd>4.6 mA, with an integrated DC-DC converter and LDO</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.seeedstudio.com/Wio-SX1262-for-XIAO-p-6379.html">US$4.99</a> at Seeed Studio</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.seeedstudio.com/Wio-SX1262-for-XIAO-p-6379.html"><b>Seeed Studio</b><span class="hw-price">US$4.99</span><small>Wio-SX1262 for XIAO, 862 to 930 MHz</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/wio-sx1262-for-xiao"><b>The Pi Hut</b><span class="hw-price">£4.80</span><small>Wio-SX1262 for XIAO, 862 to 930 MHz</small></a></li><li><a class="hw-offer" href="https://heltec.org/project/wifi-lora-32-v3/"><b>Heltec Automation</b><span class="hw-price">US$17.90 to US$19.90</span><small>WiFi LoRa 32 (V3), ESP32-S3 + SX1262 LoRa node, 863 to 870 or 902 to 928 MHz variants</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1262">Datasheet <small>Semtech SX1262</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html">Crate <code>pamoja-lora</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="lorawan-rp002">
<div class="hw-head"><h4>LoRaWAN Regional Parameters</h4><p class="hw-by">LoRa Alliance</p><p class="hw-summary">The document that fixes the channel plan, data rates, transmit limits and maximum payload a LoRaWAN device must obey in each regulatory region.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>LoRaWAN over sub-GHz LoRa</dd></div><div><dt>Revision cited</dt><dd>RP002-1.0.5, October 2025, Final</dd></div><div><dt>Fixes per region</dt><dd>channel frequencies, data rates, output power encoding, join CFList, receive windows and maximum payload</dd></div><div><dt>Limits</dt><dd>duty cycle in the dynamic-plan regions, dwell time where frequency-hopping rules apply</dd></div><div><dt>EU863-870</dt><dd>at least 24 stored channels, with 868.1, 868.3 and 868.5 MHz at DR0 to DR5 mandatory</dd></div><div><dt>pamoja implements nine of its plans</dt><dd>EU863-870, US902-928, EU433, AU915-928, CN470-510, AS923, KR920-923, IN865-867 and RU864-870</dd></div></dl>
<div class="hw-foot">
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://resources.lora-alliance.org/technical-specifications/rp002-1-0-5-lorawan-regional-parameters">Datasheet <small>LoRa Alliance RP002-1.0.5</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html">Crate <code>pamoja-lora</code></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lorawan/index.html">Crate <code>pamoja-lorawan</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/lora.html">LoRa airtime guide</a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/lorawan.html">LoRaWAN guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="esp-now">
<div class="hw-head"><h4>ESP-NOW</h4><p class="hw-by">Espressif Systems</p><p class="hw-summary">Espressif's connectionless protocol: short packets straight between Wi-Fi devices with no access point and no association step.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>Wi-Fi PHY, vendor-specific action frames</dd></div><div><dt>Payload</dt><dd>250 bytes on v1.0 devices, 1470 bytes on v2.0</dd></div><div><dt>Default bit rate</dt><dd>1 Mbps</dd></div><div><dt>Peers</dt><dd>20 paired devices, of which at most 17 encrypted, 7 by default</dd></div><div><dt>Security</dt><dd>CCMP per IEEE 802.11-2012, with a 16-byte primary master key and per-peer local keys</dd></div><div><dt>Interfaces</dt><dd>sends over either the station or the SoftAP interface</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.seeedstudio.com/Seeed-XIAO-ESP32C3-p-5431.html">US$4.99</a> at Seeed Studio</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.seeedstudio.com/Seeed-XIAO-ESP32C3-p-5431.html"><b>Seeed Studio</b><span class="hw-price">US$4.99</span><small>Seeed Studio XIAO ESP32-C3</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/seeed-xiao-esp32c3"><b>The Pi Hut</b><span class="hw-price">£5.10</span><small>Seeed XIAO ESP32C3 (board with loose headers)</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/5337"><b>Adafruit</b><span class="hw-price">US$9.95</span><small>ESP32-C3 DevKitM-01 - 4 MB SPI Flash (Espressif ESP32-C3-MINI-1)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/network/esp_now.html">Datasheet <small>Espressif ESP-IDF</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html">Crate <code>pamoja-mesh</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/mesh.html">Mesh frames guide</a></li></ul></section>
</div>
</article>
</div>

### Boards, targets and autopilots

Where pamoja itself runs, and what it has been built and tested against.

<div class="hw-cards">
<article class="hw-card" id="raspberry-pi-5">
<div class="hw-head"><h4>Raspberry Pi 5</h4><p class="hw-by">Raspberry Pi Ltd</p><p class="hw-summary">The gateway-class Linux board: the full std build, every transport, and the dashboard.</p></div>
<dl class="hw-facts"><div><dt>SoC</dt><dd>Broadcom BCM2712, quad-core 64-bit Arm Cortex-A76 at 2.4 GHz</dd></div><div><dt>Memory</dt><dd>LPDDR4X, in 1, 2, 4, 8 and 16 GB</dd></div><div><dt>Connectivity</dt><dd>gigabit Ethernet with PoE+ via a HAT, dual-band 802.11ac, Bluetooth 5.0</dd></div><div><dt>I/O</dt><dd>the standard 40-pin header, two USB 3.0 and two USB 2.0 ports, a UART debug port</dd></div><div><dt>Note</dt><dd>The page lists the 16 GB model at $305, above the band for the smaller ones</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board; the lowest listed price is <a href="https://www.pishop.us/product/raspberry-pi-5-4gb/">US$110.00</a> at PiShop.us</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.pishop.us/product/raspberry-pi-5-4gb/"><b>PiShop.us</b><span class="hw-price">US$110.00</span><small>Raspberry Pi 5, 4 GB</small></a></li><li><a class="hw-offer" href="https://shop.pimoroni.com/products/raspberry-pi-5"><b>Pimoroni</b><span class="hw-price">£88.00</span><small>Raspberry Pi 5, 4 GB (SC1111)</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/raspberry-pi-5"><b>The Pi Hut</b><span class="hw-price">£105.60</span><small>Raspberry Pi 5, 4 GB</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.raspberrypi.com/products/raspberry-pi-5/">Documentation <small>Raspberry Pi 5</small></a></li></ul></section>
</div>
</article>
<article class="hw-card" id="raspberry-pi-zero-2-w">
<div class="hw-head"><h4>Raspberry Pi Zero 2 W</h4><p class="hw-by">Raspberry Pi Ltd</p><p class="hw-summary">The small, low-cost Linux board, running the same std build as the Pi 5 on the same 40-pin footprint.</p></div>
<dl class="hw-facts"><div><dt>SoC</dt><dd>RP3A0, quad-core 64-bit Arm Cortex-A53 at 1 GHz</dd></div><div><dt>Memory</dt><dd>512 MB</dd></div><div><dt>Wireless</dt><dd>2.4 GHz 802.11 b/g/n, Bluetooth 4.2 and BLE, onboard antenna</dd></div><div><dt>I/O</dt><dd>HAT-compatible 40-pin header footprint, unpopulated; micro USB OTG</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://shop.pimoroni.com/products/raspberry-pi-zero-2-w">£12.00</a> at Pimoroni</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://shop.pimoroni.com/products/raspberry-pi-zero-2-w"><b>Pimoroni</b><span class="hw-price">£12.00</span><small>Raspberry Pi Zero 2 W (SC1176)</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/raspberry-pi-zero-2"><b>The Pi Hut</b><span class="hw-price">£14.40</span><small>Raspberry Pi Zero 2 W</small></a></li><li><a class="hw-offer" href="https://www.pishop.us/product/raspberry-pi-zero-2-w/"><b>PiShop.us</b><span class="hw-price">US$17.25</span><small>Raspberry Pi Zero 2 W (SC1146)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/">Documentation <small>Raspberry Pi Zero 2 W</small></a></li></ul></section>
</div>
</article>
<article class="hw-card" id="esp32">
<div class="hw-head"><h4>ESP32</h4><p class="hw-by">Espressif Systems</p><p class="hw-summary">The Wi-Fi and Bluetooth microcontroller behind most small sensor nodes, with the buses above and a CAN controller on chip.</p></div>
<dl class="hw-facts"><div><dt>CPU</dt><dd>Xtensa 32-bit LX6, single or dual core, up to 240 MHz</dd></div><div><dt>Memory</dt><dd>448 KB ROM, 520 KB SRAM, 16 KB RTC SRAM</dd></div><div><dt>Wireless</dt><dd>802.11 b/g/n up to 150 Mbit/s, Bluetooth 4.2 BR/EDR and LE</dd></div><div><dt>Peripherals</dt><dd>four SPI, two I2C, three UART, 34 GPIO</dd></div><div><dt>CAN</dt><dd>a TWAI controller compatible with ISO 11898-1</dd></div><div><dt>pamoja</dt><dd>not a CI target; Xtensa needs Espressif's own Rust toolchain</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.adafruit.com/product/3269">US$15.00</a> at Adafruit</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.adafruit.com/product/3269"><b>Adafruit</b><span class="hw-price">US$15.00</span><small>Espressif ESP32 Development Board - Developer Edition</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/espressif-esp32-development-board-developer-edition"><b>The Pi Hut</b><span class="hw-price">£14.40</span><small>Espressif ESP32 Development Board - Developer Edition</small></a></li><li><a class="hw-offer" href="https://www.digikey.com/en/products/detail/espressif-systems/ESP32-DEVKITC-32E/12091810"><b>Digi-Key</b><span class="hw-price">US$10.00</span><small>ESP32-DEVKITC-32E (Espressif ESP32-DevKitC with ESP32-WROOM-32E)</small><em>from a listing, since the page refuses scripted readers</em></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://documentation.espressif.com/esp32_datasheet_en.pdf">Documentation <small>Espressif ESP32 datasheet</small></a></li></ul></section>
</div>
</article>
<article class="hw-card" id="ardupilot">
<div class="hw-head"><h4>ArduPilot</h4><p class="hw-by">ArduPilot Dev Team</p><p class="hw-summary">Open autopilot firmware for copters, planes, rovers and submarines; the firmware is free, and the flight controller it runs on is what costs money.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>MAVLink</dd></div><div><dt>Vehicles</dt><dd>multicopters, helicopters, fixed wing, rovers, submarines and antenna trackers</dd></div><div><dt>Protocol</dt><dd>MAVLink between ground station, flight controller and peripherals</dd></div><div><dt>Licence</dt><dd>GPL v3 or later</dd></div><div><dt>Verified</dt><dd>the SITL interop job in CI flies it in simulation against pamoja-mavlink</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board; the lowest listed price is <a href="https://holybro.com/products/pixhawk-6c-mini">US$130.99</a> at Holybro</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c-mini"><b>Holybro</b><span class="hw-price">US$130.99</span><small>Pixhawk 6C Mini, Model-B, no power module</small></a></li><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c"><b>Holybro</b><span class="hw-price">US$165.99</span><small>Pixhawk 6C, plastic case, no power module</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://ardupilot.org/dev/index.html">Documentation <small>ArduPilot</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html">Crate <code>pamoja-mavlink</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html">MAVLink guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="px4">
<div class="hw-head"><h4>PX4 Autopilot</h4><p class="hw-by">Dronecode Foundation</p><p class="hw-summary">Open autopilot firmware for multirotors, fixed wing, VTOL, helicopters and rovers, under a permissive licence; again, the board is the cost.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>MAVLink</dd></div><div><dt>Vehicles</dt><dd>multirotors, fixed wing, VTOL, helicopters and rovers</dd></div><div><dt>Protocol</dt><dd>MAVLink for the ground station link</dd></div><div><dt>Licence</dt><dd>BSD 3-clause</dd></div><div><dt>Verified</dt><dd>the SITL interop job in CI flies it in simulation against pamoja-mavlink</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board; the lowest listed price is <a href="https://holybro.com/products/pixhawk-6c-mini">US$130.99</a> at Holybro</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c-mini"><b>Holybro</b><span class="hw-price">US$130.99</span><small>Pixhawk 6C Mini, Model-B, no power module</small></a></li><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c"><b>Holybro</b><span class="hw-price">US$165.99</span><small>Pixhawk 6C, plastic case, no power module</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://docs.px4.io/main/en/">Documentation <small>PX4</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html">Crate <code>pamoja-mavlink</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html">MAVLink guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="pixhawk">
<div class="hw-head"><h4>Pixhawk standard</h4><p class="hw-by">Dronecode Foundation</p><p class="hw-summary">The open hardware standard that fixes flight controller pin-outs, connectors and layout so boards from different vendors interchange; a board built to it is what costs money.</p></div>
<dl class="hw-facts"><div><dt>Interface</dt><dd>standardised connectors and pin-outs</dd></div><div><dt>Autopilot standards listed</dt><dd>v5X (DS-011), v6X (DS-012), v6U (DS-016) and v6C (DS-018)</dd></div><div><dt>Also</dt><dd>the connector standard (DS-009), the autopilot bus standard (DS-010) and the payload bus standard (DS-014); a smart battery standard is in draft</dd></div><div><dt>Note</dt><dd>Each fixes pin-outs, block diagrams, PCB layout guidelines and connector specifications</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board; the lowest listed price is <a href="https://holybro.com/products/pixhawk-6c-mini">US$130.99</a> at Holybro</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c-mini"><b>Holybro</b><span class="hw-price">US$130.99</span><small>Pixhawk 6C Mini, Model-B, no power module</small></a></li><li><a class="hw-offer" href="https://holybro.com/products/pix32-v6"><b>Holybro</b><span class="hw-price">US$146.99</span><small>Pix32 v6, FC module only</small></a></li><li><a class="hw-offer" href="https://holybro.com/products/pixhawk-6c"><b>Holybro</b><span class="hw-price">US$165.99</span><small>Pixhawk 6C, plastic case, no power module</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://pixhawk.org/standards/">Documentation <small>Pixhawk standards</small></a></li><li><a class="hw-link" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html">Crate <code>pamoja-mavlink</code></a></li><li><a class="hw-link guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html">MAVLink guide</a></li></ul></section>
</div>
</article>
<article class="hw-card" id="cortex-m4f">
<div class="hw-head"><h4>Cortex-M4 with FPU, bare metal</h4><p class="hw-by">Arm</p><p class="hw-summary">The bare-metal target: CI cross-compiles every no_std crate for thumbv7em-none-eabihf, which is this core with its floating-point unit and no operating system.</p></div>
<dl class="hw-facts"><div><dt>Architecture</dt><dd>Armv7-M</dd></div><div><dt>Floating point</dt><dd>a built-in single-precision FPU, credited with 10x on single-precision operations</dd></div><div><dt>DSP</dt><dd>integrated DSP, SIMD and MAC instructions</dd></div><div><dt>Use cases the page names</dt><dd>industrial control, IoT sensors, AI and ML, audio</dd></div><div><dt>pamoja</dt><dd>the bare-metal job in CI builds pamoja-core, security, audit, update, zenoh, ros2 and mavlink for this target</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board; the lowest listed price is <a href="https://www.seeedstudio.com/Seeed-XIAO-BLE-nRF52840-p-5201.html">US$9.99</a> at Seeed Studio</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h5>Where to buy <small>prices as listed on 2026-09-06</small></h5><ul class="hw-offers"><li><a class="hw-offer" href="https://www.seeedstudio.com/Seeed-XIAO-BLE-nRF52840-p-5201.html"><b>Seeed Studio</b><span class="hw-price">US$9.99</span><small>Seeed Studio XIAO nRF52840 (XIAO BLE)</small></a></li><li><a class="hw-offer" href="https://www.adafruit.com/product/3800"><b>Adafruit</b><span class="hw-price">US$14.95</span><small>Adafruit ItsyBitsy M4 Express featuring ATSAMD51</small></a></li><li><a class="hw-offer" href="https://thepihut.com/products/stm32f411-blackpill-development-board"><b>The Pi Hut</b><span class="hw-price">£15.90</span><small>STM32F411 BlackPill Development Board (DFRobot DFR0864)</small></a></li></ul></section>
<section class="hw-learn"><h5>Read and build</h5><ul class="hw-links"><li><a class="hw-link" href="https://www.arm.com/products/silicon-ip-cpu/cortex-m/cortex-m4">Documentation <small>Arm Cortex-M4</small></a></li></ul></section>
</div>
</article>
</div>
<!-- end -->

## Anything else with a driver

The four sensors and two actuator families above are the parts pamoja decodes itself. They are
not the limit of what it can talk to. Everything below the driver is a trait: implement
[`Sensor`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/device/trait.Sensor.html) or
[`Actuator`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/device/trait.Actuator.html) for
your own part and the rest of the library, the profiles, the ladder, the dashboard, treats it
exactly like the ones here. The [sensor drivers guide](guides/sensors.md) and the
[actuator drivers guide](guides/actuators.md) show what that takes.

The same holds for transports. A part reachable over I2C, SPI, a serial line, RS485, or CAN is
reachable through the crates listed under Buses, whether or not its decoder ships here.
