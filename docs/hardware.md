# Hardware

pamoja is a software library, so it runs on whatever its host runs on. This page is narrower
and more useful than that: it lists the parts the drivers were written against, the radios they
budget airtime for, and the boards the project builds and tests on. The buses and protocols
themselves are on [Buses and links](buses.md), because a protocol is not something you can buy.

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

**Where to buy.** Each part lists a few product pages from the makers' own stores and the
larger distributors, the cheapest reputable option first, with the price each page listed
on the day it was read. Those are places to buy, not sources: nothing on a card is taken from
them. A workflow reads every page again each week, orders the offers cheapest first, and opens
a pull request with what moved, so the day on the card is never more than a week old. A vendor
whose site refuses scripted readers is priced from a listing of its page, and the card says so.
An entry that is not a thing you buy, firmware, a standard, a protocol, or an instruction set,
carries no price of its own; its list names the boards that carry it instead.

**How this page stays honest.** The entries are tied to the code. Adding a driver module under
`pamoja-sensors` or `pamoja-actuators` without an entry here fails `cargo xtask docs --check`,
and so does adding a LoRaWAN channel plan the page does not list.

<!-- table: hardware -->
## Sensors

Parts pamoja decodes byte for byte, each with a driver written from the datasheet linked beside it.

<nav class="hw-index" aria-label="Sensors index"><a href="#bme280">BME280</a><a href="#ds18b20">DS18B20</a><a href="#ina219">INA219</a><a href="#ads1115">ADS1115</a><a href="#sht3x">SHT3x-DIS</a><a href="#scd4x">SCD40 and SCD41</a><a href="#tmp117">TMP117</a><a href="#hdc1080">HDC1080</a><a href="#opt3001">OPT3001</a><a href="#ina226">INA226</a><a href="#bmp280">BMP280</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="bme280">
<header class="hw-head">

### BME280 {#bme280}

<p class="hw-by">Bosch Sensortec</p>
<p class="hw-summary">Reports humidity, pressure and temperature as raw counts plus per-chip calibration coefficients, which the driver compensates.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C up to 3.4 MHz, or SPI up to 10 MHz</dd></div><div><dt>Ranges</dt><dd>0 to 100 %RH, 300 to 1100 hPa, -40 to +85 °C</dd></div><div><dt>Accuracy</dt><dd>±3 %RH from 20 to 80 %RH at 25 °C, ±1.0 hPa, ±0.5 °C from 0 to 65 °C</dd></div><div><dt>Addresses</dt><dd>0x76 with SDO to ground, 0x77 with SDO to VDDIO; SDO must not float</dd></div><div><dt>Supply</dt><dd>1.71 to 3.6 V main, 1.2 to 3.6 V interface</dd></div><div><dt>Current</dt><dd>3.6 µA at 1 Hz for all three, 0.1 µA asleep</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/2652"><span class="hw-main"><b>Adafruit</b><small>Adafruit BME280 I2C or SPI Temperature Humidity Pressure Sensor - STEMMA QT</small></span><span class="hw-price">US$14.95</span></a></li><li><a class="hw-row" href="https://www.sparkfun.com/sparkfun-atmospheric-sensor-breakout-bme280-qwiic.html"><span class="hw-main"><b>SparkFun</b><small>SparkFun Atmospheric Sensor Breakout - BME280 (Qwiic)</small></span><span class="hw-price">US$16.95</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/bme280-breakout"><span class="hw-main"><b>Pimoroni</b><small>BME280 Breakout - Temperature, Pressure, Humidity Sensor</small></span><span class="hw-price">£13.80</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf"><span class="hw-main"><b>Datasheet</b><small>Bosch BST-BME280-DS002</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/bme280.rs"><span class="hw-main"><b>Driver source</b><small><code>bme280.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="ds18b20">
<header class="hw-head">

### DS18B20 {#ds18b20}

<p class="hw-by">Analog Devices, originally Maxim Integrated</p>
<p class="hw-summary">A digital thermometer that returns Celsius over a single data line, each part carrying its own 64-bit serial code so a bus can hold many.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>1-Wire, many devices on one pin</dd></div><div><dt>Range</dt><dd>-55 °C to +125 °C</dd></div><div><dt>Accuracy</dt><dd>±0.5 °C from -10 °C to +85 °C</dd></div><div><dt>Resolution</dt><dd>9 to 12 bits, user programmable</dd></div><div><dt>Conversion</dt><dd>750 ms maximum at 12 bits, halving with each bit dropped</dd></div><div><dt>Supply</dt><dd>3.0 to 5.5 V, or parasite power from the data line</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/374"><span class="hw-main"><b>Adafruit</b><small>DS18B20 Digital temperature sensor + extras</small></span><span class="hw-price">US$3.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/ds18b20-one-wire-digital-temperature-sensor"><span class="hw-main"><b>The Pi Hut</b><small>DS18B20+ One Wire Digital Temperature Sensor</small></span><span class="hw-price">£7.00</span></a></li><li><a class="hw-row" href="https://www.sparkfun.com/temperature-sensor-waterproof-ds18b20.html"><span class="hw-main"><b>SparkFun</b><small>Temperature Sensor - Waterproof (DS18B20)</small></span><span class="hw-price">US$10.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.analog.com/media/en/technical-documentation/data-sheets/ds18b20.pdf"><span class="hw-main"><b>Datasheet</b><small>ADI 19-7487 Rev 6</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ds18b20.rs"><span class="hw-main"><b>Driver source</b><small><code>ds18b20.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="ina219">
<header class="hw-head">

### INA219 {#ina219}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">Measures the drop across an external shunt and the bus voltage, and reports current and power once its calibration register is set.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, 16 addresses from two pins</dd></div><div><dt>Bus voltage</dt><dd>senses 0 to 26 V</dd></div><div><dt>Shunt full scale</dt><dd>±40, ±80, ±160 or ±320 mV by PGA setting</dd></div><div><dt>ADC</dt><dd>12-bit, selectable down to 9-bit or averaged</dd></div><div><dt>Registers</dt><dd>10 µV per shunt count, 4 mV per bus count</dd></div><div><dt>Supply</dt><dd>3 to 5.5 V, 0.7 mA typical</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.dfrobot.com/product-1827.html"><span class="hw-main"><b>DFRobot</b><small>Gravity: I2C Digital Wattmeter</small></span><span class="hw-price">US$6.90</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/904"><span class="hw-main"><b>Adafruit</b><small>INA219 High Side DC Current Sensor Breakout - 26V ±3.2A Max - STEMMA QT</small></span><span class="hw-price">US$9.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/adafruit-ina219-high-side-dc-current-sensor-breakout-26v-3-2a-max"><span class="hw-main"><b>The Pi Hut</b><small>INA219 High Side DC Current Sensor Breakout - 26V ±3.2A Max (STEMMA QT)</small></span><span class="hw-price">£9.60</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/ina219.pdf"><span class="hw-main"><b>Datasheet</b><small>TI SBOS448G</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ina219.rs"><span class="hw-main"><b>Driver source</b><small><code>ina219.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="ads1115">
<header class="hw-head">

### ADS1115 {#ads1115}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">A 16-bit delta-sigma ADC that digitises four single-ended or two differential inputs through a programmable gain amplifier.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, four addresses from one pin</dd></div><div><dt>Resolution</dt><dd>16 bits</dd></div><div><dt>Inputs</dt><dd>four single-ended or two differential</dd></div><div><dt>Full scale</dt><dd>±0.256 V to ±6.144 V in six PGA steps</dd></div><div><dt>Rate</dt><dd>8 to 860 samples per second</dd></div><div><dt>Supply</dt><dd>2.0 to 5.5 V, 150 µA in continuous conversion</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Grove-ADS1115-16-bit-ADC-p-4599.html"><span class="hw-main"><b>Seeed Studio</b><small>Grove - 4 Channel 16-bit ADC (ADS1115) with Programmable Amplifier Gain</small></span><span class="hw-price">US$12.00</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/1085"><span class="hw-main"><b>Adafruit</b><small>ADS1115 16-Bit ADC - 4 Channel with Programmable Gain Amplifier - STEMMA QT / Qwiic</small></span><span class="hw-price">US$14.95</span></a></li><li><a class="hw-row" href="https://www.dfrobot.com/product-1730.html"><span class="hw-main"><b>DFRobot</b><small>Gravity: I2C ADS1115 16-Bit ADC Module</small></span><span class="hw-price">US$15.50</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/ads1115.pdf"><span class="hw-main"><b>Datasheet</b><small>TI SBAS444E</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ads1115.rs"><span class="hw-main"><b>Driver source</b><small><code>ads1115.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sht3x">
<header class="hw-head">

### SHT3x-DIS {#sht3x}

<p class="hw-by">Sensirion</p>
<p class="hw-summary">Reports humidity and temperature as calibrated 16-bit words, each followed by a CRC-8, which the driver checks and converts with the fixed linear formulas.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C up to 1 MHz, with optional clock stretching</dd></div><div><dt>Ranges</dt><dd>0 to 100 %RH, -40 to +125 °C</dd></div><div><dt>Accuracy</dt><dd>typical ±2 %RH and ±0.2 °C (SHT30, SHT31), ±1.5 %RH and ±0.1 °C (SHT35)</dd></div><div><dt>Repeatability</dt><dd>0.08 to 0.21 %RH and 0.04 to 0.15 °C across the three modes, measured in 15, 6, or 4 ms</dd></div><div><dt>Addresses</dt><dd>0x44 with ADDR to logic low, 0x45 with ADDR to logic high; ADDR must not float</dd></div><div><dt>Supply</dt><dd>2.15 to 5.5 V</dd></div><div><dt>Current</dt><dd>0.2 µA idle in single-shot mode, 600 µA measuring, 1.7 µA average at one low-repeatability measurement per second</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Grove-Temperature-Humidity-Sensor-SHT31-p-2655.html"><span class="hw-main"><b>Seeed Studio</b><small>Grove - Temperature &amp; Humidity Sensor (SHT31)</small></span><span class="hw-price">US$9.90</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/2857"><span class="hw-main"><b>Adafruit</b><small>Adafruit Sensirion SHT31-D Temperature &amp; Humidity Sensor</small></span><span class="hw-price">US$13.95</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/4099"><span class="hw-main"><b>Adafruit</b><small>SHT-30 Mesh-protected Weather-proof Temperature/Humidity Sensor - 1M Cable</small></span><span class="hw-price">US$24.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://sensirion.com/media/documents/213E6A3B/63A5A569/Datasheet_SHT3x_DIS.pdf"><span class="hw-main"><b>Datasheet</b><small>Sensirion SHT3x-DIS datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/sht3x.rs"><span class="hw-main"><b>Driver source</b><small><code>sht3x.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="scd4x">
<header class="hw-head">

### SCD40 and SCD41 {#scd4x}

<p class="hw-by">Sensirion</p>
<p class="hw-summary">A photoacoustic NDIR CO2 sensor with a built-in humidity and temperature sensor; every word it sends carries a CRC-8 the driver checks.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C at 0x62, up to 100 kHz</dd></div><div><dt>CO2 range</dt><dd>0 to 40000 ppm output; specified 400 to 2000 ppm (SCD40) and 400 to 5000 ppm (SCD41)</dd></div><div><dt>CO2 accuracy</dt><dd>±(50 ppm + 5 %) for the SCD40, ±(40 ppm + 5 %) for the SCD41</dd></div><div><dt>Humidity and temperature</dt><dd>0 to 100 %RH within ±6 %RH, -10 to 60 °C within ±0.8 °C</dd></div><div><dt>Update interval</dt><dd>5 s periodic, about 30 s low power, single shot on the SCD41</dd></div><div><dt>Supply</dt><dd>2.4 to 5.5 V, 15 mA average and 175 mA peak at 3.3 V in periodic measurement</dd></div><div><dt>Typical cost</dt><dd>$20 to $60 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/5187"><span class="hw-main"><b>Adafruit</b><small>Adafruit SCD-40 - True CO2, Temperature and Humidity Sensor - STEMMA QT / Qwiic</small></span><span class="hw-price">US$44.95</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/5190"><span class="hw-main"><b>Adafruit</b><small>Adafruit SCD-41 - True CO2 Temperature and Humidity Sensor - STEMMA QT / Qwiic</small></span><span class="hw-price">US$49.95</span></a></li><li><a class="hw-row" href="https://www.seeedstudio.com/Grove-CO2-Temperature-Humidity-Sensor-SCD41-p-5025.html"><span class="hw-main"><b>Seeed Studio</b><small>Grove - CO2 &amp; Temperature &amp; Humidity Sensor - SCD41</small></span><span class="hw-price">US$52.90</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://sensirion.com/media/documents/E0F04247/631EF271/CD_DS_SCD40_SCD41_Datasheet_D1.pdf"><span class="hw-main"><b>Datasheet</b><small>Sensirion SCD4x datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/scd4x.rs"><span class="hw-main"><b>Driver source</b><small><code>scd4x.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="tmp117">
<header class="hw-head">

### TMP117 {#tmp117}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">A 16-bit digital thermometer accurate to ±0.1 °C with no calibration, with programmable alert limits, a user offset, and 48 bits of EEPROM.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C and SMBus, 1 kHz to 400 kHz, four addresses from one pin</dd></div><div><dt>Range</dt><dd>-55 °C to +150 °C</dd></div><div><dt>Accuracy</dt><dd>±0.1 °C from -20 °C to 50 °C, ±0.3 °C over the full range</dd></div><div><dt>Resolution</dt><dd>16 bits, 7.8125 m°C per count</dd></div><div><dt>Addresses</dt><dd>0x48 to 0x4B with ADD0 tied to GND, V+, SDA, or SCL</dd></div><div><dt>Conversion</dt><dd>15.5 ms typical one-shot, 1 s cycle with 8 averages at reset</dd></div><div><dt>Supply</dt><dd>1.7 to 5.5 V up to 70 °C, 1.8 to 5.5 V to 150 °C</dd></div><div><dt>Current</dt><dd>3.5 µA typical at a 1 Hz cycle, 150 nA in shutdown</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/4821"><span class="hw-main"><b>Adafruit</b><small>Adafruit TMP117 ±0.1°C High Accuracy I2C Temperature Sensor - STEMMA QT / Qwiic</small></span><span class="hw-price">US$11.50</span></a></li><li><a class="hw-row" href="https://www.sparkfun.com/sparkfun-high-precision-temperature-sensor-tmp117-qwiic.html"><span class="hw-main"><b>SparkFun</b><small>SparkFun High Precision Temperature Sensor - TMP117 (Qwiic)</small></span><span class="hw-price">US$16.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/tmp117.pdf"><span class="hw-main"><b>Datasheet</b><small>TI TMP117 datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/tmp117.rs"><span class="hw-main"><b>Driver source</b><small><code>tmp117.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="hdc1080">
<header class="hw-head">

### HDC1080 {#hdc1080}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">Reports humidity and temperature as 16-bit fractions of full scale with no per-chip calibration, which the driver decodes in exact integer arithmetic alongside the configuration register.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, 10 to 400 kHz</dd></div><div><dt>Ranges</dt><dd>0 to 100 %RH; temperature sensor -40 to +125 °C, humidity sensor -20 to +70 °C (functional to +85 °C)</dd></div><div><dt>Accuracy</dt><dd>±2 %RH typical; ±0.2 °C typical, ±0.4 °C max from 5 to 60 °C</dd></div><div><dt>Resolution</dt><dd>14, 11, or 8 bit humidity in 6.50, 3.85, or 2.50 ms; 14 or 11 bit temperature in 6.35 or 3.65 ms</dd></div><div><dt>Address</dt><dd>fixed 0x40; manufacturer id 0x5449, device id 0x1050, 40-bit serial number</dd></div><div><dt>Supply</dt><dd>2.7 to 5.5 V; 1.3 µA average at 1 sample/s for 11-bit humidity and temperature, 100 nA asleep</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.lcsc.com/product-detail/C82227.html"><span class="hw-main"><b>LCSC</b><small>Texas Instruments HDC1080DMBR</small></span><span class="hw-price">US$1.46 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://www.digikey.com/en/products/detail/texas-instruments/HDC1080DMBR/5878653"><span class="hw-main"><b>Digi-Key</b><small>Texas Instruments HDC1080DMBR</small><small class="hw-note">listed price; the page refuses scripted readers</small></span><span class="hw-price">US$2.27 on 2026-09-06</span></a></li></ul><p class="hw-when">prices as listed on the day named</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/hdc1080.pdf"><span class="hw-main"><b>Datasheet</b><small>TI HDC1080 datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/hdc1080.rs"><span class="hw-main"><b>Driver source</b><small><code>hdc1080.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="opt3001">
<header class="hw-head">

### OPT3001 {#opt3001}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">A single-chip lux meter filtered to the human eye's response that reports illuminance as a 4-bit exponent and 12-bit mantissa, ranging itself across twelve full-scale settings.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C or SMBus up to 2.6 MHz, four addresses from the ADDR pin, INT pin</dd></div><div><dt>Range</dt><dd>0.01 to 83865.6 lux, 12 binary-weighted full-scale ranges or automatic ranging</dd></div><div><dt>Resolution</dt><dd>0.01 lux per LSB at the lowest range, 23-bit effective dynamic range</dd></div><div><dt>Accuracy</dt><dd>2000 lux reads 1800 to 2200 lux, 0.2 % matching between ranges, 0.2 % response at 850 nm</dd></div><div><dt>Conversion</dt><dd>100 ms or 800 ms, 720 to 880 ms in the 800 ms setting</dd></div><div><dt>Addresses</dt><dd>0x44 to 0x47 with ADDR to GND, VDD, SDA, or SCL</dd></div><div><dt>Supply</dt><dd>1.6 to 3.6 V, 1.8 µA active and 0.3 µA shutdown typical, I/O 5.5 V tolerant</dd></div><div><dt>Temperature</dt><dd>-40 to +85 °C</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/rak1903-opt3001dnpr-ambient-light-sensor"><span class="hw-main"><b>RAKwireless</b><small>Light Sensor Texas Instruments OPT3001 | RAK1903</small></span><span class="hw-price">US$3.60</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/opt3001.pdf"><span class="hw-main"><b>Datasheet</b><small>TI OPT3001 datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/opt3001.rs"><span class="hw-main"><b>Driver source</b><small><code>opt3001.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="ina226">
<header class="hw-head">

### INA226 {#ina226}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">Measures the drop across a shunt and the bus voltage on either side of the load, reports current and power once calibrated, and drives an alert pin from one programmable limit.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C or SMBus up to 2.94 MHz, 16 addresses from two pins</dd></div><div><dt>Bus voltage</dt><dd>senses 0 to 36 V, 1.25 mV per count</dd></div><div><dt>Shunt full scale</dt><dd>±81.92 mV, 2.5 µV per count</dd></div><div><dt>ADC</dt><dd>16-bit, 140 µs to 8.244 ms per conversion, 1 to 1024 samples averaged</dd></div><div><dt>Accuracy</dt><dd>0.1 % gain error and 10 µV shunt offset, maximum</dd></div><div><dt>Alert</dt><dd>open-drain pin on a shunt, bus, or power limit, or conversion ready</dd></div><div><dt>Supply</dt><dd>2.7 to 5.5 V, 330 µA typical, 2 µA maximum powered down</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.mouser.com/ProductDetail/Texas-Instruments/INA226AIDGSR"><span class="hw-main"><b>Mouser</b><small>Texas Instruments INA226AIDGSR</small><small class="hw-note">listed price; the page refuses scripted readers</small></span><span class="hw-price">US$2.66 on 2026-09-06</span></a></li><li><a class="hw-row" href="https://shop.m5stack.com/products/ina226-1a-current-voltage-power-monitor-unit"><span class="hw-main"><b>M5Stack</b><small>INA226-1A Current Voltage Power Monitor Unit</small></span><span class="hw-price">US$8.50 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/ina226-1a-current-voltage-power-monitor-unit"><span class="hw-main"><b>The Pi Hut</b><small>INA226-1A Current Voltage Power Monitor Unit</small></span><span class="hw-price">£8.20 on 2026-09-07</span></a></li></ul><p class="hw-when">prices as listed on the day named</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/ina226.pdf"><span class="hw-main"><b>Datasheet</b><small>TI INA226 datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/ina226.rs"><span class="hw-main"><b>Driver source</b><small><code>ina226.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="bmp280">
<header class="hw-head">

### BMP280 {#bmp280}

<p class="hw-by">Bosch Sensortec</p>
<p class="hw-summary">Reports barometric pressure and temperature as 20-bit raw counts plus per-chip calibration coefficients, which the driver compensates; the BME280 without the humidity element.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C up to 3.4 MHz, or SPI (3 or 4 wire) up to 10 MHz</dd></div><div><dt>Ranges</dt><dd>300 to 1100 hPa, -40 to +85 °C (full accuracy 0 to +65 °C)</dd></div><div><dt>Accuracy</dt><dd>±1.0 hPa absolute from 0 to 65 °C, ±0.12 hPa relative from 700 to 900 hPa, ±0.5 °C at 25 °C</dd></div><div><dt>Resolution</dt><dd>0.16 Pa and 0.0003 °C at ×16 oversampling; 1.3 Pa RMS noise unfiltered, 0.2 Pa with the IIR filter</dd></div><div><dt>Addresses</dt><dd>0x76 with SDO to ground, 0x77 with SDO to VDDIO; SDO must not float</dd></div><div><dt>Supply</dt><dd>1.71 to 3.6 V main, 1.2 to 3.6 V interface</dd></div><div><dt>Current</dt><dd>2.8 µA at 1 Hz forced mode, lowest power; 0.1 µA asleep</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Grove-Barometer-Sensor-BMP280.html"><span class="hw-main"><b>Seeed Studio</b><small>Grove - BMP280 I2C and SPI Barometric and Temperature Sensor</small></span><span class="hw-price">US$9.80</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/2651"><span class="hw-main"><b>Adafruit</b><small>Adafruit BMP280 I2C or SPI Barometric Pressure &amp; Altitude Sensor - STEMMA QT</small></span><span class="hw-price">US$9.95</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/bmp280-breakout-temperature-pressure-altitude-sensor"><span class="hw-main"><b>Pimoroni</b><small>BMP280 Breakout - Temperature, Pressure, Altitude Sensor</small></span><span class="hw-price">£9.30</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bmp280-ds001.pdf"><span class="hw-main"><b>Datasheet</b><small>Bosch BST-BMP280-DS001</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-sensors/src/bmp280.rs"><span class="hw-main"><b>Driver source</b><small><code>bmp280.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-sensors</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/sensors.html"><span class="hw-main"><b>Sensor drivers guide</b><small>Datasheet-anchored decoders for eleven parts</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
</div>

## Actuators

Parts pamoja drives: a PWM generator for servos, and the step-and-direction carriers a stepper sequence walks.

<nav class="hw-index" aria-label="Actuators index"><a href="#pca9685">PCA9685</a><a href="#uln2003">ULN2003A</a><a href="#a4988">A4988</a><a href="#drv8825">DRV8825</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="pca9685">
<header class="hw-head">

### PCA9685 {#pca9685}

<p class="hw-by">NXP Semiconductors</p>
<p class="hw-summary">A 16-channel PWM generator that produces servo and dimming pulses; each output sinks 25 mA, so load current is switched by an external driver.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>I2C, up to 62 devices per bus</dd></div><div><dt>Channels</dt><dd>16, all sharing one frequency</dd></div><div><dt>Resolution</dt><dd>12-bit, 4096 steps per output</dd></div><div><dt>Frequency</dt><dd>typically 24 Hz to 1526 Hz, 200 Hz at reset</dd></div><div><dt>Supply</dt><dd>2.3 to 5.5 V, inputs and outputs 5.5 V tolerant</dd></div><div><dt>Drive</dt><dd>sinks 25 mA, sources 10 mA at 5 V; larger loads need external drivers</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.sparkfun.com/sparkfun-servo-phat-for-raspberry-pi.html"><span class="hw-main"><b>SparkFun</b><small>SparkFun Servo pHAT for Raspberry Pi (DEV-15316)</small></span><span class="hw-price">US$13.95</span></a></li><li><a class="hw-row" href="https://www.seeedstudio.com/Grove-16-Channel-PWM-Driver-PCA9685.html"><span class="hw-main"><b>Seeed Studio</b><small>Grove - 16-Channel PWM Driver (PCA9685)</small></span><span class="hw-price">US$14.20</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/815"><span class="hw-main"><b>Adafruit</b><small>Adafruit 16-Channel 12-bit PWM/Servo Driver - I2C interface</small></span><span class="hw-price">US$14.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf"><span class="hw-main"><b>Datasheet</b><small>NXP PCA9685 Rev. 4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/pca9685.rs"><span class="hw-main"><b>Driver source</b><small><code>pca9685.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-actuators</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html"><span class="hw-main"><b>Actuator drivers guide</b><small>PCA9685 PWM and servo pulses</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="uln2003">
<header class="hw-head">

### ULN2003A {#uln2003}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">A Darlington array that switches the coil current directly, for the four-wire steppers the coil sequencer walks a pattern across.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>seven logic inputs, one per channel</dd></div><div><dt>Channels</dt><dd>seven NPN Darlington pairs</dd></div><div><dt>Collector current</dt><dd>500 mA rated, single output</dd></div><div><dt>Output voltage</dt><dd>50 V maximum</dd></div><div><dt>Inductive loads</dt><dd>common-cathode output clamp diodes included</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.digikey.com/en/products/detail/texas-instruments/ULN2003AN/277624"><span class="hw-main"><b>Digi-Key</b><small>ULN2003AN, 16-DIP (bare chip)</small><small class="hw-note">listed price; the page refuses scripted readers</small></span><span class="hw-price">US$0.97 on 2026-09-06</span></a></li><li><a class="hw-row" href="https://www.seeedstudio.com/Gear-Stepper-Motor-Driver-Pack-p-3200.html"><span class="hw-main"><b>Seeed Studio</b><small>Gear Stepper Motor Driver Pack (ULN2003 driver board with 28BYJ-48 motor)</small></span><span class="hw-price">US$6.90 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/stepper-motor-driver-pack"><span class="hw-main"><b>The Pi Hut</b><small>Stepper Motor Driver Pack (Seeed 105990072)</small></span><span class="hw-price">£6.70 on 2026-09-07</span></a></li></ul><p class="hw-when">prices as listed on the day named</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/product/ULN2003A"><span class="hw-main"><b>Datasheet</b><small>TI ULN2003A</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs"><span class="hw-main"><b>Driver source</b><small><code>stepper.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-actuators</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html"><span class="hw-main"><b>Actuator drivers guide</b><small>PCA9685 PWM and servo pulses</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="a4988">
<header class="hw-head">

### A4988 {#a4988}

<p class="hw-by">Allegro MicroSystems</p>
<p class="hw-summary">A bipolar stepper driver that turns one pulse on STEP into one microstep, sequencing the coils and regulating current in hardware.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>step and direction, with three mode pins</dd></div><div><dt>Motor supply</dt><dd>8 to 35 V</dd></div><div><dt>Output current</dt><dd>±2 A maximum</dd></div><div><dt>Steps</dt><dd>full, 1/2, 1/4, 1/8 and 1/16</dd></div><div><dt>Logic supply</dt><dd>3 to 5.5 V</dd></div><div><dt>Note</dt><dd>The datasheet states no continuous per-phase current against a cooling condition</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/6109"><span class="hw-main"><b>Adafruit</b><small>Adafruit A4988 Stepper Motor Driver Breakout Board</small></span><span class="hw-price">US$6.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/adafruit-a4988-stepper-motor-driver-breakout-board"><span class="hw-main"><b>The Pi Hut</b><small>Adafruit A4988 Stepper Motor Driver Breakout Board</small></span><span class="hw-price">£6.70</span></a></li><li><a class="hw-row" href="https://www.pololu.com/product/1182"><span class="hw-main"><b>Pololu</b><small>A4988 Stepper Motor Driver Carrier</small></span><span class="hw-price">US$8.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.allegromicro.com/-/media/files/datasheets/a4988-datasheet.pdf"><span class="hw-main"><b>Datasheet</b><small>Allegro A4988 Rev. 8</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs"><span class="hw-main"><b>Driver source</b><small><code>stepper.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-actuators</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html"><span class="hw-main"><b>Actuator drivers guide</b><small>PCA9685 PWM and servo pulses</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="drv8825">
<header class="hw-head">

### DRV8825 {#drv8825}

<p class="hw-by">Texas Instruments</p>
<p class="hw-summary">A bipolar stepper driver with two H-bridges and an indexer, taking step and direction down to 1/32 step.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>step and direction, with three mode pins</dd></div><div><dt>Motor supply</dt><dd>8.2 to 45 V</dd></div><div><dt>Output current</dt><dd>up to 2.5 A per output at 24 V and 25 °C, with heat sinking</dd></div><div><dt>Steps</dt><dd>full, 1/2, 1/4, 1/8, 1/16 and 1/32</dd></div><div><dt>Decay</dt><dd>slow, fast or mixed</dd></div><div><dt>Protection</dt><dd>overcurrent, thermal shutdown and undervoltage lockout, reported on nFAULT</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://shop.m5stack.com/products/atomic-stepmotor-base-drv8825"><span class="hw-main"><b>M5Stack</b><small>ATOMIC Stepmotor Base (DRV8825)</small></span><span class="hw-price">US$8.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/atomic-stepmotor-base-drv8825"><span class="hw-main"><b>The Pi Hut</b><small>ATOMIC Stepmotor Base (DRV8825)</small></span><span class="hw-price">£8.70</span></a></li><li><a class="hw-row" href="https://www.pololu.com/product/2133"><span class="hw-main"><b>Pololu</b><small>DRV8825 Stepper Motor Driver Carrier, High Current</small></span><span class="hw-price">US$15.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.ti.com/lit/ds/symlink/drv8825.pdf"><span class="hw-main"><b>Datasheet</b><small>TI SLVSA73F</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-actuators/src/stepper.rs"><span class="hw-main"><b>Driver source</b><small><code>stepper.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-actuators</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/actuators.html"><span class="hw-main"><b>Actuator drivers guide</b><small>PCA9685 PWM and servo pulses</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/buses.html"><span class="hw-main"><b>Buses and links</b><small>what the interface line means, and how to wire it</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
</div>

## Radios and long-range links

Reaching a network that is not there: the transceivers, the gateway concentrators, and the short-range meshes.

<nav class="hw-index" aria-label="Radios and long-range links index"><a href="#sx1276">SX1276</a><a href="#sx1262">SX1262</a><a href="#llcc68">LLCC68</a><a href="#sx1302">SX1302</a><a href="#sx1303">SX1303</a><a href="#sx1250">SX1250</a><a href="#rak2287">RAK2287 WisLink concentrator</a><a href="#rak5146">RAK5146 WisLink concentrator</a><a href="#wm1302">WM1302 LoRaWAN gateway module</a><a href="#esp-now">ESP-NOW</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="sx1276">
<header class="hw-head">

### SX1276 {#sx1276}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The sub-GHz LoRa and FSK transceiver on most 137 to 1020 MHz end devices, and the radio a pamoja airtime budget is computed for.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Coverage</dt><dd>137 MHz to 1020 MHz</dd></div><div><dt>Modulation</dt><dd>LoRa, FSK, GFSK, MSK, GMSK and OOK</dd></div><div><dt>Transmit power</dt><dd>+20 dBm at 100 mW, with a separate +14 dBm high efficiency amplifier</dd></div><div><dt>Sensitivity</dt><dd>down to -148 dBm, for a maximum link budget of 168 dB</dd></div><div><dt>Receive current</dt><dd>9.9 mA, with 200 nA register retention</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://sparkfun.com/products/18085"><span class="hw-main"><b>SparkFun</b><small>LoRa Transceiver Module (RFM95CW), 137 to 1020 MHz, SX1276 based</small></span><span class="hw-price">US$14.95</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/3072"><span class="hw-main"><b>Adafruit</b><small>Adafruit RFM95W LoRa Radio Transceiver Breakout - 868 or 915 MHz</small></span><span class="hw-price">US$19.95</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/adafruit-rfm95w-lora-radio-transceiver-breakout?variant=19595325639"><span class="hw-main"><b>Pimoroni</b><small>Adafruit RFM95W LoRa Radio Transceiver Breakout - 868 or 915 MHz (ADA3072)</small></span><span class="hw-price">£19.50</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1276"><span class="hw-main"><b>Datasheet</b><small>Semtech SX1276</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-radios/src/sx127x.rs"><span class="hw-main"><b>Driver source</b><small><code>sx127x.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate for LoRa airtime and range</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html"><span class="hw-main"><b>Crate for LoRa radios</b><small><code>pamoja-radios</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/radios.html"><span class="hw-main"><b>LoRa radios guide</b><small>The Semtech SX126x and SX127x LoRa radios</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx1262">
<header class="hw-head">

### SX1262 {#sx1262}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The later sub-GHz LoRa transceiver, covering 150 to 960 MHz with a higher power amplifier and a much lower receive current.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Coverage</dt><dd>150 MHz to 960 MHz, continuous</dd></div><div><dt>Modulation</dt><dd>LoRa, LR-FHSS, FSK, GFSK, MSK and GMSK</dd></div><div><dt>Transmit power</dt><dd>+22 dBm high efficiency amplifier</dd></div><div><dt>Sensitivity</dt><dd>down to -148 dBm, for a maximum link budget of 170 dB</dd></div><div><dt>Receive current</dt><dd>4.6 mA, with an integrated DC-DC converter and LDO</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Wio-SX1262-for-XIAO-p-6379.html"><span class="hw-main"><b>Seeed Studio</b><small>Wio-SX1262 for XIAO, 862 to 930 MHz</small></span><span class="hw-price">US$4.99 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/wio-sx1262-for-xiao"><span class="hw-main"><b>The Pi Hut</b><small>Wio-SX1262 for XIAO, 862 to 930 MHz</small></span><span class="hw-price">£4.80 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://heltec.org/project/wifi-lora-32-v3/"><span class="hw-main"><b>Heltec Automation</b><small>WiFi LoRa 32 (V3), ESP32-S3 + SX1262 LoRa node, 863 to 870 or 902 to 928 MHz variants</small></span><span class="hw-price">US$17.90 to US$19.90 on 2026-09-06</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/sx1262-868mhz-lora-node-module-for-raspberry-pi-pico"><span class="hw-main"><b>The Pi Hut</b><small>SX1262 868 MHz LoRa node module for Raspberry Pi Pico, with antenna and battery holder</small></span><span class="hw-price">£14.40 on 2026-09-11</span></a></li><li><a class="hw-row" href="https://www.waveshare.com/pico-lora-sx1262-868m.htm"><span class="hw-main"><b>Waveshare</b><small>Pico-LoRa-SX1262, SX1262 LoRa node module for Raspberry Pi Pico, EU868 and other bands</small></span><span class="hw-price">US$19.99 on 2026-09-11</span></a></li></ul><p class="hw-when">prices as listed on the day named</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/sx1262"><span class="hw-main"><b>Datasheet</b><small>Semtech SX1262</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-radios/src/sx126x.rs"><span class="hw-main"><b>Driver source</b><small><code>sx126x.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate for LoRa airtime and range</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html"><span class="hw-main"><b>Crate for LoRa radios</b><small><code>pamoja-radios</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/radios.html"><span class="hw-main"><b>LoRa radios guide</b><small>The Semtech SX126x and SX127x LoRa radios</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="llcc68">
<header class="hw-head">

### LLCC68 {#llcc68}

<p class="hw-by">Semtech</p>
<p class="hw-summary">A sub-GHz LoRa transceiver for medium-range indoor and indoor-to-outdoor links, pin-to-pin compatible with the SX1262.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Transmit power</dt><dd>+22 dBm high efficiency amplifier</dd></div><div><dt>Receive current</dt><dd>4.6 mA</dd></div><div><dt>Sensitivity</dt><dd>down to -129 dBm</dd></div><div><dt>Blocking immunity</dt><dd>88 dB at 1 MHz offset; co-channel rejection 19 dB in LoRa mode</dd></div><div><dt>Channel activity detection</dt><dd>automatic, with ultra-fast AFC</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://shop.m5stack.com/products/lora-unit-433mhz-with-antenna-e220"><span class="hw-main"><b>M5Stack</b><small>LoRa Unit 433MHz with Antenna (E220), an LLCC68 module</small></span><span class="hw-price">US$12.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/lora-unit-with-antenna-e220"><span class="hw-main"><b>The Pi Hut</b><small>LoRa Unit with Antenna (E220), 920 MHz, an LLCC68 module</small></span><span class="hw-price">£33.80</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-connect/llcc68"><span class="hw-main"><b>Datasheet</b><small>Semtech LLCC68</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://github.com/molexxxx/pamoja/blob/main/crates/pamoja-radios/src/sx126x.rs"><span class="hw-main"><b>Driver source</b><small><code>sx126x.rs</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate for LoRa airtime and range</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_radios/index.html"><span class="hw-main"><b>Crate for LoRa radios</b><small><code>pamoja-radios</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/radios.html"><span class="hw-main"><b>LoRa radios guide</b><small>The Semtech SX126x and SX127x LoRa radios</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx1302">
<header class="hw-head">

### SX1302 {#sx1302}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The LoRa baseband chip for gateways: it listens across many channels and spreading factors at once, behind separate RF front ends, and draws far less current than the chips before it.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Sensitivity</dt><dd>up to -141 dBm with the SX1250 front end</dd></div><div><dt>125 kHz reception</dt><dd>8 x 8 channel packet detectors, 8 SF5 to SF12 and 8 SF5 to SF10 demodulators</dd></div><div><dt>Wideband reception</dt><dd>one 125, 250 or 500 kHz LoRa demodulator and one (G)FSK demodulator</dd></div><div><dt>Front ends</dt><dd>direct interface to the SX1250, SX1255, SX1257 and SX1258</dd></div><div><dt>Clock and package</dt><dd>a single 32 MHz clock, QFN68</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/WM1302-LoRaWAN-Gateway-Module-SPI-EU868-p-4889.html"><span class="hw-main"><b>Seeed Studio</b><small>Wio-WM1302 Long Range Gateway Module (SPI), EU868</small></span><span class="hw-price">US$12.99</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1302-rak2287-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK2287 concentrator module, SPI</small></span><span class="hw-price">US$84.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-core/sx1302"><span class="hw-main"><b>Datasheet</b><small>Semtech SX1302</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx1303">
<header class="hw-head">

### SX1303 {#sx1303}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The SX1302's size and pin compatible successor, which adds a fine timestamp so a network can locate a node by time difference of arrival.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Note</dt><dd>Fine timestamp for time difference of arrival (TDOA) network geolocation</dd></div><div><dt>Sensitivity</dt><dd>up to -141 dBm with the SX1250 front end</dd></div><div><dt>125 kHz reception</dt><dd>8 x 8 channel packet detectors, 8 SF5 to SF12 and 8 SF5 to SF10 demodulators</dd></div><div><dt>Wideband reception</dt><dd>one 125, 250 or 500 kHz LoRa demodulator</dd></div><div><dt>Compatibility</dt><dd>size and pin compatible with the SX1302, with all of its features</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1303-rak5146-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK5146 concentrator module, SPI</small></span><span class="hw-price">US$83.00</span></a></li><li><a class="hw-row" href="https://store.rokland.com/products/rak-wireless-rak5146-wislink-lpwan-concentrator-for-lorawan?variant=43285563441235"><span class="hw-main"><b>Rokland</b><small>RAK5146 WisLink LPWAN Concentrator, USB, no LBT, with GPS, US915 (516011)</small></span><span class="hw-price">US$114.97</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-core/sx1303"><span class="hw-main"><b>Datasheet</b><small>Semtech SX1303</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="sx1250">
<header class="hw-head">

### SX1250 {#sx1250}

<p class="hw-by">Semtech</p>
<p class="hw-summary">The sub-GHz RF front end a gateway pairs with the SX1302, covering any license-free band below 1 GHz.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>SPI</dd></div><div><dt>Coverage</dt><dd>150 to 960 MHz ISM bands</dd></div><div><dt>Operation</dt><dd>half duplex, capable of low power operation</dd></div><div><dt>Modulation</dt><dd>constant-envelope schemes such as LoRa or FSK</dd></div><div><dt>Receiver</dt><dd>a built-in differential low noise amplifier</dd></div><div><dt>Control</dt><dd>SPI and a set of general purpose DIO lines</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/WM1302-LoRaWAN-Gateway-Module-SPI-EU868-p-4889.html"><span class="hw-main"><b>Seeed Studio</b><small>Wio-WM1302 Long Range Gateway Module (SPI), EU868</small></span><span class="hw-price">US$12.99</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1303-rak5146-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK5146 concentrator module, SPI</small></span><span class="hw-price">US$83.00</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1302-rak2287-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK2287 concentrator module, SPI</small></span><span class="hw-price">US$84.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.semtech.com/products/wireless-rf/lora-core/sx1250"><span class="hw-main"><b>Datasheet</b><small>Semtech SX1250</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rak2287">
<header class="hw-head">

### RAK2287 WisLink concentrator {#rak2287}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">An SX1302 concentrator on a mini PCIe card, with two SX1250 front ends, a U.FL antenna connector and a GPS for timing.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>mini PCIe, SPI</dd></div><div><dt>Chips</dt><dd>one SX1302 and two SX1250</dd></div><div><dt>Radio</dt><dd>Tx power up to 27 dBm, Rx sensitivity down to -139 dBm at SF12, BW 125 kHz</dd></div><div><dt>Demodulation</dt><dd>up to 10 parallel paths, an 8 x 8 channel packet detector, 8 SF5 to SF12 and 8 SF5 to SF10 demodulators</dd></div><div><dt>Bands</dt><dd>EU868, EU433, CN470, US915, AS923, AU915, KR920 and IN865</dd></div><div><dt>Host</dt><dd>SPI and 3.3 V aux through the mini PCIe connector; ZOE-M8Q GPS with its PPS wired to the SX1302</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1302-rak2287-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK2287, SPI without GPS, 8XX MHz for EU868/RU864/IN865</small></span><span class="hw-price">US$84.00</span></a></li><li><a class="hw-row" href="https://store.rokland.com/products/rak2287-gateway-concentrator-module-for-lorawan-sx1302-lora-core-spi-us-915-mhz-516019?variant=43041812414547"><span class="hw-main"><b>Rokland</b><small>RAK2287 concentrator module, SPI with GPS, US915 (516000)</small></span><span class="hw-price">US$109.97</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.rakwireless.com/product-categories/wislink/rak2287/datasheet/"><span class="hw-main"><b>Datasheet</b><small>RAKwireless RAK2287</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rak5146">
<header class="hw-head">

### RAK5146 WisLink concentrator {#rak5146}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">An SX1303 concentrator on a mini PCIe card, with two SX1250 front ends, an SX126x for listen before talk, and an onboard GPS.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>mini PCIe, SPI or USB</dd></div><div><dt>Chips</dt><dd>one SX1303 and two SX1250, with an SX126x for listen before talk</dd></div><div><dt>Radio</dt><dd>Tx power up to 27 dBm, Rx sensitivity down to -139 dBm at SF12, BW 125 kHz</dd></div><div><dt>Demodulation</dt><dd>up to 10 parallel paths, an 8 x 8 channel packet detector, 8 SF5 to SF12 and 8 SF5 to SF10 demodulators</dd></div><div><dt>Bands</dt><dd>EU868, EU433, RU864, CN470, US915, AS923, AU915, KR920 and IN865</dd></div><div><dt>Host</dt><dd>SPI or USB through the mini PCIe connector, powered from 3.3 V aux</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/wislink-concentrator-module-sx1303-rak5146-lorawan"><span class="hw-main"><b>RAKwireless</b><small>RAK5146, SPI, no LBT and no GPS, 8XX MHz for EU868/RU864/IN865</small></span><span class="hw-price">US$83.00</span></a></li><li><a class="hw-row" href="https://store.rokland.com/products/rak-wireless-rak5146-wislink-lpwan-concentrator-for-lorawan?variant=43285563441235"><span class="hw-main"><b>Rokland</b><small>RAK5146 WisLink LPWAN Concentrator, USB, no LBT, with GPS, US915 (516011)</small></span><span class="hw-price">US$114.97</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.rakwireless.com/product-categories/wislink/rak5146/datasheet/"><span class="hw-main"><b>Datasheet</b><small>RAKwireless RAK5146</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="wm1302">
<header class="hw-head">

### WM1302 LoRaWAN gateway module {#wm1302}

<p class="hw-by">Seeed Studio</p>
<p class="hw-summary">An SX1302 gateway module on a mini PCIe card with the standard 52-pin edge connector, in SPI and USB versions for EU868 and US915.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>mini PCIe, SPI or USB</dd></div><div><dt>Chip</dt><dd>the SX1302 baseband with an SX1250 front end</dd></div><div><dt>Radio</dt><dd>sensitivity down to -139 dBm at SF12; TX power up to 26 dBm at 3.3 V</dd></div><div><dt>Versions</dt><dd>SPI and USB, each on US915 and EU868</dd></div><div><dt>Plans</dt><dd>EU868, US915, AS923, AS920, AU915, KR920 and IN865</dd></div><div><dt>Form factor</dt><dd>mini PCIe with a 52-pin edge connector</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/WM1302-LoRaWAN-Gateway-Module-SPI-EU868-p-4889.html"><span class="hw-main"><b>Seeed Studio</b><small>Wio-WM1302 Long Range Gateway Module (SPI), EU868</small></span><span class="hw-price">US$12.99</span></a></li><li><a class="hw-row" href="https://www.seeedstudio.com/WM1302-LoRaWAN-Gateway-Module-SPI-US915-SKY66420-p-5455.html"><span class="hw-main"><b>Seeed Studio</b><small>WM1302 LoRaWAN Gateway Module (SPI), US915, with SKY66420</small></span><span class="hw-price">US$25.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://wiki.seeedstudio.com/WM1302_module/"><span class="hw-main"><b>Datasheet</b><small>Seeed Studio WM1302</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="esp-now">
<header class="hw-head">

### ESP-NOW {#esp-now}

<p class="hw-by">Espressif Systems</p>
<p class="hw-summary">Espressif's connectionless protocol: short packets straight between Wi-Fi devices with no access point and no association step.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>Wi-Fi PHY, vendor-specific action frames</dd></div><div><dt>Payload</dt><dd>250 bytes on v1.0 devices, 1470 bytes on v2.0</dd></div><div><dt>Default bit rate</dt><dd>1 Mbps</dd></div><div><dt>Peers</dt><dd>20 paired devices, of which at most 17 encrypted, 7 by default</dd></div><div><dt>Security</dt><dd>CCMP per IEEE 802.11-2012, with a 16-byte primary master key and per-peer local keys</dd></div><div><dt>Interfaces</dt><dd>sends over either the station or the SoftAP interface</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Boards that speak it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Seeed-XIAO-ESP32C3-p-5431.html"><span class="hw-main"><b>Seeed Studio</b><small>Seeed Studio XIAO ESP32-C3</small></span><span class="hw-price">US$4.99</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/seeed-xiao-esp32c3?variant=53975115661697"><span class="hw-main"><b>The Pi Hut</b><small>Seeed XIAO ESP32C3 (board with loose headers)</small></span><span class="hw-price">£5.10</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/5337"><span class="hw-main"><b>Adafruit</b><small>ESP32-C3 DevKitM-01 - 4 MB SPI Flash (Espressif ESP32-C3-MINI-1)</small></span><span class="hw-price">US$9.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/network/esp_now.html"><span class="hw-main"><b>Datasheet</b><small>Espressif ESP-IDF</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-mesh</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/mesh.html"><span class="hw-main"><b>Mesh frames guide</b><small>Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>range, loss, duty cycle, and what the regulator allows</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
</div>

## Antennas, cables and protection

What stands between a radio and the air: the antenna, the pigtail that reaches it, and the arrestor on the mast.

<nav class="hw-index" aria-label="Antennas, cables and protection index"><a href="#rakarg13">5.8 dBi fiberglass antenna, 863 to 870 MHz</a><a href="#rakarg14">5.8 dBi fiberglass antenna, 902 to 928 MHz</a><a href="#rak-mhf-sma-pigtail">MHF to SMA or RP-SMA pigtail</a><a href="#adafruit-sma-ufl-cable">SMA to u.FL adapter cable</a><a href="#rak-lightning-arrestor">N-type lightning arrestor</a><a href="#l-com-lightning-protector">N-type lightning surge protector, DC to 6 GHz</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="rakarg13">
<header class="hw-head">

### 5.8 dBi fiberglass antenna, 863 to 870 MHz {#rakarg13}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">An outdoor fiberglass gateway antenna for the European band, an omnidirectional, vertically polarized dipole that comes with its installation kit.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>N-type male</dd></div><div><dt>Band</dt><dd>863 to 870 MHz</dd></div><div><dt>Gain</dt><dd>5.8 dBi peak as specified; 6.15 to 6.33 dBi measured from 864 to 870 MHz</dd></div><div><dt>VSWR</dt><dd>2.0 or better; 1.13 measured at 868 MHz</dd></div><div><dt>Pattern</dt><dd>omnidirectional, vertical polarization, 50 ohms, efficiency up to 79%</dd></div><div><dt>Size</dt><dd>29.8 mm by 800.0 mm, white fiberglass radome</dd></div><div><dt>Environment</dt><dd>operating from -30 to +65 C, 5 to 95% humidity</dd></div><div><dt>Typical cost</dt><dd>$20 to $60 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.sparkfun.com/lora-fiberglass-antenna-type-n-5-8dbi-863-870mhz.html"><span class="hw-main"><b>SparkFun</b><small>LoRa Fiberglass Antenna Type N - 5.8dBi (863-870MHz), with mounting hardware and no cable</small></span><span class="hw-price">US$29.95</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/5-8dbi-fiber-glass-antenna?variant=41100705267910"><span class="hw-main"><b>RAKwireless</b><small>5.8dBi Fiberglass Antenna, N-Type to RP-SMA, White, 863MHz-870MHz, with brackets and an indoor pigtail</small></span><span class="hw-price">US$40.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.rakwireless.com/product-categories/accessories/rakarg13/datasheet/"><span class="hw-main"><b>Datasheet</b><small>RAKwireless RAKARG13</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rakarg14">
<header class="hw-head">

### 5.8 dBi fiberglass antenna, 902 to 928 MHz {#rakarg14}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">An outdoor fiberglass gateway antenna for the 902 to 928 MHz band, an omnidirectional, vertically polarized dipole that comes with its installation kit.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>N-type male</dd></div><div><dt>Band</dt><dd>902 to 930 MHz</dd></div><div><dt>Gain</dt><dd>5.8 dBi peak as specified; 5.4 to 6.1 dBi measured from 902 to 930 MHz</dd></div><div><dt>VSWR</dt><dd>2.0 or better; 1.33 measured at 902 MHz and 1.47 at 930 MHz</dd></div><div><dt>Pattern</dt><dd>omnidirectional, vertical polarization, 50 ohms, efficiency up to 79%</dd></div><div><dt>Size</dt><dd>29.8 mm by 800.0 mm, white fiberglass radome</dd></div><div><dt>Environment</dt><dd>operating from -20 to +65 C, 5 to 95% humidity</dd></div><div><dt>Typical cost</dt><dd>$20 to $60 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.sparkfun.com/lora-fiberglass-antenna-type-n-5-8dbi-902-928mhz.html"><span class="hw-main"><b>SparkFun</b><small>LoRa Fiberglass Antenna Type N - 5.8dBi (902-928MHz), with mounting hardware and no cable</small></span><span class="hw-price">US$14.95</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/5-8dbi-fiber-glass-antenna?variant=39942855033030"><span class="hw-main"><b>RAKwireless</b><small>5.8dBi Fiberglass Antenna, N-Type to RP-SMA, White, 902MHz-928MHz, with brackets and an indoor pigtail</small></span><span class="hw-price">US$40.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.rakwireless.com/product-categories/accessories/rakarg14/datasheet/"><span class="hw-main"><b>Datasheet</b><small>RAKwireless RAKARG14</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rak-mhf-sma-pigtail">
<header class="hw-head">

### MHF to SMA or RP-SMA pigtail {#rak-mhf-sma-pigtail}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">The short coaxial lead from a module's MHF (U.FL-type) socket to a bulkhead connector on the enclosure wall, where the antenna screws on.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>MHF-1 or MHF-4 to SMA or RP-SMA</dd></div><div><dt>Connectors</dt><dd>MHF-1 or MHF-4 on one end, RP-SMA or SMA with a nut and washer on the other</dd></div><div><dt>Lengths</dt><dd>65, 100 and 220 mm</dd></div><div><dt>Note</dt><dd>MHF-1 fits the RAK2287 and RAK5146 concentrators; MHF-4 fits the RAK5166 and RAK5167</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/mhf-to-sma-connector"><span class="hw-main"><b>RAKwireless</b><small>MHF to RP-SMA / SMA Connector</small></span><span class="hw-price">US$5.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/mhf-to-sma-connector"><span class="hw-main"><b>Datasheet</b><small>RAKwireless MHF pigtail</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="adafruit-sma-ufl-cable">
<header class="hw-head">

### SMA to u.FL adapter cable {#adafruit-sma-ufl-cable}

<p class="hw-by">Adafruit</p>
<p class="hw-summary">A short RG178 lead from a board's u.FL socket to an SMA connector fixed through the enclosure wall, where an SMA antenna screws on.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>u.FL to panel-mount SMA</dd></div><div><dt>Cable</dt><dd>RG178, 15 cm (5.9 in) long, not counting the SMA connector</dd></div><div><dt>Connectors</dt><dd>u.FL, also sold as IPEX, IPX or MHF, to a panel-mount SMA</dd></div><div><dt>Fit</dt><dd>SMA only; SMA and RP-SMA do not mate, so check the antenna first</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://shop.pimoroni.com/products/adafruit-sma-to-ufl-u-fl-ipx-ipex-rf-adapter-cable"><span class="hw-main"><b>Pimoroni</b><small>SMA to uFL/u.FL/IPX/IPEX RF Adapter Cable (Adafruit 851)</small></span><span class="hw-price">£3.25</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/851"><span class="hw-main"><b>Adafruit</b><small>SMA to uFL/u.FL/IPX/IPEX RF Adapter Cable</small></span><span class="hw-price">US$3.95</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.adafruit.com/product/851"><span class="hw-main"><b>Datasheet</b><small>Adafruit 851</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="rak-lightning-arrestor">
<header class="hw-head">

### N-type lightning arrestor {#rak-lightning-arrestor}

<p class="hw-by">RAKwireless</p>
<p class="hw-summary">A surge protective device for a gateway's antenna line, fitted on each N-type antenna terminal.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>N-type male to N-type female</dd></div><div><dt>Frequency range</dt><dd>0 to 2700 MHz, 50 ohms</dd></div><div><dt>Loss</dt><dd>0.2 dB or less from 0 to 2000 MHz</dd></div><div><dt>Discharge current</dt><dd>10 kA nominal, 20 kA maximum</dd></div><div><dt>Voltage protection level</dt><dd>1200 V or less</dd></div><div><dt>Ingress protection</dt><dd>IP67</dd></div><div><dt>Typical cost</dt><dd>$20 to $60 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rokland.com/products/rakwireless-lightning-arrestor-n-male-n-female-0-2700-mhz-pid-910033"><span class="hw-main"><b>Rokland</b><small>RAKwireless lightning arrestor N-Male N-Female 0-2700 MHz (PID 910033)</small></span><span class="hw-price">US$24.97</span></a></li><li><a class="hw-row" href="https://store.rakwireless.com/products/lightning-arrestor"><span class="hw-main"><b>RAKwireless</b><small>Lightning Arrestor, N-type</small></span><span class="hw-price">US$32.00</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://store.rakwireless.com/products/lightning-arrestor"><span class="hw-main"><b>Datasheet</b><small>RAKwireless arrestor</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="l-com-lightning-protector">
<header class="hw-head">

### N-type lightning surge protector, DC to 6 GHz {#l-com-lightning-protector}

<p class="hw-by">L-com</p>
<p class="hw-summary">A gas discharge tube surge protector for an antenna line, with a replaceable tube, DC passing through, and a bulkhead port that seals through an enclosure wall with an O-ring.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>N-type male to bulkhead N-type female</dd></div><div><dt>Frequency range</dt><dd>DC to 6 GHz, 50 ohms</dd></div><div><dt>Loss</dt><dd>insertion loss of 0.6 dB or less and VSWR of 1.2:1 or better, DC to 6 GHz</dd></div><div><dt>Gas tube</dt><dd>90 V breakdown, plus or minus 20%, replaceable (LPX090-6), multi-strike</dd></div><div><dt>Protection</dt><dd>bidirectional, so either port can face the antenna, with a ground lug on the housing</dd></div><div><dt>Size</dt><dd>68 by 23 by 31 mm, 0.11 kg, nickel-plated brass body</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.l-com.com/surge-protector-n-male-to-n-female-bulkhead-0-6-ghz-90v-lightning-protector"><span class="hw-main"><b>L-com</b><small>N-Male to N-Female Bulkhead 0-6 GHz 90V Lightning Protector (AL6-NMNFBW-9)</small></span><span class="hw-price">US$78.99</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-11</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.l-com.com/Images/Downloadables/Datasheets/ds_AL6-NMNFBW-9.pdf"><span class="hw-main"><b>Datasheet</b><small>L-com AL6-NMNFBW-9</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_lora/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-lora</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/lora.html"><span class="hw-main"><b>LoRa airtime and range guide</b><small>Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/radio.html"><span class="hw-main"><b>Radios and antennas</b><small>gain, VSWR, feed line loss, height, and bonding</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
</div>

## Boards you can buy

Where pamoja itself runs, each with a page that wires a part to it and runs the first program.

<nav class="hw-index" aria-label="Boards you can buy index"><a href="#raspberry-pi-5">Raspberry Pi 5</a><a href="#raspberry-pi-zero-2-w">Raspberry Pi Zero 2 W</a><a href="#esp32">ESP32</a><a href="#raspberry-pi-pico">Raspberry Pi Pico</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="raspberry-pi-5">
<header class="hw-head">

### Raspberry Pi 5 {#raspberry-pi-5}

<p class="hw-by">Raspberry Pi Ltd</p>
<p class="hw-summary">The gateway-class Linux board: the full std build, every transport, and the dashboard.</p>
</header>
<dl class="hw-facts"><div><dt>SoC</dt><dd>Broadcom BCM2712, quad-core 64-bit Arm Cortex-A76 at 2.4 GHz</dd></div><div><dt>Memory</dt><dd>LPDDR4X, in 1, 2, 4, 8 and 16 GB</dd></div><div><dt>Connectivity</dt><dd>gigabit Ethernet with PoE+ via a HAT, dual-band 802.11ac, Bluetooth 5.0</dd></div><div><dt>I/O</dt><dd>the standard 40-pin header, two USB 3.0 and two USB 2.0 ports, a UART debug port</dd></div><div><dt>Note</dt><dd>the 16 GB model lists at $305, above the band the smaller ones fall in</dd></div><div><dt>Typical cost</dt><dd>$60 to $200 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.pishop.us/product/raspberry-pi-5-4gb/"><span class="hw-main"><b>PiShop.us</b><small>Raspberry Pi 5, 4 GB</small></span><span class="hw-price">US$110.00</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/raspberry-pi-5?variant=41044580204627"><span class="hw-main"><b>Pimoroni</b><small>Raspberry Pi 5, 4 GB (SC1111)</small></span><span class="hw-price">£105.60</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/raspberry-pi-5?variant=42531604922563"><span class="hw-main"><b>The Pi Hut</b><small>Raspberry Pi 5, 4 GB</small></span><span class="hw-price">£105.60</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.raspberrypi.com/products/raspberry-pi-5/"><span class="hw-main"><b>Documentation</b><small>Raspberry Pi 5</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/boards/raspberry-pi.html"><span class="hw-main"><b>Board page</b><small>wiring, setup, and the first program</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="raspberry-pi-zero-2-w">
<header class="hw-head">

### Raspberry Pi Zero 2 W {#raspberry-pi-zero-2-w}

<p class="hw-by">Raspberry Pi Ltd</p>
<p class="hw-summary">The small, low-cost Linux board, running the same std build as the Pi 5 on the same 40-pin footprint.</p>
</header>
<dl class="hw-facts"><div><dt>SoC</dt><dd>RP3A0, quad-core 64-bit Arm Cortex-A53 at 1 GHz</dd></div><div><dt>Memory</dt><dd>512 MB</dd></div><div><dt>Wireless</dt><dd>2.4 GHz 802.11 b/g/n, Bluetooth 4.2 and BLE, onboard antenna</dd></div><div><dt>I/O</dt><dd>HAT-compatible 40-pin header footprint, unpopulated; micro USB OTG</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.pishop.us/product/raspberry-pi-zero-2-w/"><span class="hw-main"><b>PiShop.us</b><small>Raspberry Pi Zero 2 W (SC1146)</small></span><span class="hw-price">US$17.25</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/raspberry-pi-zero-2-w?variant=39493046075475"><span class="hw-main"><b>Pimoroni</b><small>Raspberry Pi Zero 2 W (SC1176)</small></span><span class="hw-price">£14.40</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/raspberry-pi-zero-2?variant=41181426909379"><span class="hw-main"><b>The Pi Hut</b><small>Raspberry Pi Zero 2 W</small></span><span class="hw-price">£14.40</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/"><span class="hw-main"><b>Documentation</b><small>Raspberry Pi Zero 2 W</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/boards/raspberry-pi.html"><span class="hw-main"><b>Board page</b><small>wiring, setup, and the first program</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="esp32">
<header class="hw-head">

### ESP32 {#esp32}

<p class="hw-by">Espressif Systems</p>
<p class="hw-summary">The Wi-Fi and Bluetooth microcontroller behind most small sensor nodes, with the buses above and a CAN controller on chip.</p>
</header>
<dl class="hw-facts"><div><dt>CPU</dt><dd>Xtensa 32-bit LX6, single or dual core, up to 240 MHz</dd></div><div><dt>Memory</dt><dd>448 KB ROM, 520 KB SRAM, 16 KB RTC SRAM</dd></div><div><dt>Wireless</dt><dd>802.11 b/g/n up to 150 Mbit/s, Bluetooth 4.2 BR/EDR and LE</dd></div><div><dt>Peripherals</dt><dd>four SPI, two I2C, three UART, 34 GPIO</dd></div><div><dt>CAN</dt><dd>a TWAI controller compatible with ISO 11898-1</dd></div><div><dt>pamoja</dt><dd>the ESP32-C3 first program builds in CI on stable Rust; the Xtensa parts need Espressif's own Rust toolchain</dd></div><div><dt>Typical cost</dt><dd>$5 to $20 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.digikey.com/en/products/detail/espressif-systems/ESP32-DEVKITC-32E/12091810"><span class="hw-main"><b>Digi-Key</b><small>ESP32-DEVKITC-32E (Espressif ESP32-DevKitC with ESP32-WROOM-32E)</small><small class="hw-note">listed price; the page refuses scripted readers</small></span><span class="hw-price">US$10.00 on 2026-09-06</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/3269"><span class="hw-main"><b>Adafruit</b><small>Espressif ESP32 Development Board - Developer Edition</small></span><span class="hw-price">US$15.00 on 2026-09-07</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/espressif-esp32-development-board-developer-edition"><span class="hw-main"><b>The Pi Hut</b><small>Espressif ESP32 Development Board - Developer Edition</small></span><span class="hw-price">£14.40 on 2026-09-07</span></a></li></ul><p class="hw-when">prices as listed on the day named</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://documentation.espressif.com/esp32_datasheet_en.pdf"><span class="hw-main"><b>Documentation</b><small>Espressif ESP32 datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/boards/esp32.html"><span class="hw-main"><b>Board page</b><small>wiring, setup, and the first program</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="raspberry-pi-pico">
<header class="hw-head">

### Raspberry Pi Pico {#raspberry-pi-pico}

<p class="hw-by">Raspberry Pi Ltd</p>
<p class="hw-summary">The RP2040 board: two Cortex-M0+ cores and no operating system, running the no_std crates over its own I2C, SPI, and UART peripherals.</p>
</header>
<dl class="hw-facts"><div><dt>Microcontroller</dt><dd>RP2040 with 2 MB flash, dual-core Cortex-M0+ at up to 133 MHz, 264 kB SRAM</dd></div><div><dt>I/O</dt><dd>26 multi-function 3.3 V GPIO on a 40-pin DIP footprint with castellated edges, 3 of them ADC capable</dd></div><div><dt>Peripherals</dt><dd>2 UART, 2 I2C, 2 SPI, 16 PWM channels, a 12-bit 500 ksps ADC, USB 1.1, two PIO blocks</dd></div><div><dt>Programming</dt><dd>hold BOOTSEL at power-up and the board is a USB mass-storage device that takes a UF2 file; a 3-pin SWD port for a debugger</dd></div><div><dt>pamoja</dt><dd>the first program builds in CI for thumbv6m-none-eabi on rp2040-hal</dd></div><div><dt>Typical cost</dt><dd>under $5 for a breakout module or a board</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Where to buy</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.pishop.us/product/raspberry-pi-pico/"><span class="hw-main"><b>PiShop.us</b><small>Raspberry Pi Pico (Non-Wireless)</small></span><span class="hw-price">US$3.95</span></a></li><li><a class="hw-row" href="https://shop.pimoroni.com/products/raspberry-pi-pico"><span class="hw-main"><b>Pimoroni</b><small>Raspberry Pi Pico, without headers (SC0915)</small></span><span class="hw-price">£3.17</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/raspberry-pi-pico"><span class="hw-main"><b>The Pi Hut</b><small>Raspberry Pi Pico, without headers</small></span><span class="hw-price">£3.80</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-09</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://datasheets.raspberrypi.com/pico/pico-datasheet.pdf"><span class="hw-main"><b>Documentation</b><small>Pico datasheet</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/boards/rp2040.html"><span class="hw-main"><b>Board page</b><small>wiring, setup, and the first program</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
</div>

## Autopilots, standards and targets

Firmware, an open hardware standard, and an instruction set. None of these is a thing you buy; each names the boards that carry it.

<nav class="hw-index" aria-label="Autopilots, standards and targets index"><a href="#ardupilot">ArduPilot</a><a href="#px4">PX4 Autopilot</a><a href="#pixhawk">Pixhawk standard</a><a href="#cortex-m4f">Cortex-M4 with FPU, bare metal</a></nav>

<div class="hw-cards">
<article class="hw-card" aria-labelledby="ardupilot">
<header class="hw-head">

### ArduPilot {#ardupilot}

<p class="hw-by">ArduPilot Dev Team</p>
<p class="hw-summary">Open autopilot firmware for copters, planes, rovers and submarines; the firmware is free, and the flight controller it runs on is what costs money.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>MAVLink</dd></div><div><dt>Vehicles</dt><dd>multicopters, helicopters, fixed wing, rovers, submarines and antenna trackers</dd></div><div><dt>Protocol</dt><dd>MAVLink between ground station, flight controller and peripherals</dd></div><div><dt>License</dt><dd>GPL v3 or later</dd></div><div><dt>Verified</dt><dd>the SITL interop job in CI flies it in simulation against pamoja-mavlink</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Flight controllers that run it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c-mini?variant=44511519637693"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C Mini, Model-B, no power module</small></span><span class="hw-price">US$130.99</span></a></li><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c?variant=42783569871037"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C, plastic case, no power module</small></span><span class="hw-price">US$165.99</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://ardupilot.org/dev/index.html"><span class="hw-main"><b>Documentation</b><small>ArduPilot</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-mavlink</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html"><span class="hw-main"><b>MAVLink guide</b><small>MAVLink v1 and v2 framing, signing, named message fields</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="px4">
<header class="hw-head">

### PX4 Autopilot {#px4}

<p class="hw-by">Dronecode Foundation</p>
<p class="hw-summary">Open autopilot firmware for multirotors, fixed wing, VTOL, helicopters and rovers, under a permissive license; again, the board is the cost.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>MAVLink</dd></div><div><dt>Vehicles</dt><dd>multirotors, fixed wing, VTOL, helicopters and rovers</dd></div><div><dt>Protocol</dt><dd>MAVLink for the ground station link</dd></div><div><dt>License</dt><dd>BSD 3-clause</dd></div><div><dt>Verified</dt><dd>the SITL interop job in CI flies it in simulation against pamoja-mavlink</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Flight controllers that run it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c-mini?variant=44511519637693"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C Mini, Model-B, no power module</small></span><span class="hw-price">US$130.99</span></a></li><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c?variant=42783569871037"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C, plastic case, no power module</small></span><span class="hw-price">US$165.99</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://docs.px4.io/main/en/"><span class="hw-main"><b>Documentation</b><small>PX4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-mavlink</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html"><span class="hw-main"><b>MAVLink guide</b><small>MAVLink v1 and v2 framing, signing, named message fields</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="pixhawk">
<header class="hw-head">

### Pixhawk standard {#pixhawk}

<p class="hw-by">Dronecode Foundation</p>
<p class="hw-summary">The open hardware standard that fixes flight controller pin-outs, connectors and layout so boards from different vendors interchange; a board built to it is what costs money.</p>
</header>
<dl class="hw-facts"><div><dt>Interface</dt><dd>standardized connectors and pin-outs</dd></div><div><dt>Autopilot standards</dt><dd>v5X (DS-011), v6X (DS-012), v6U (DS-016) and v6C (DS-018)</dd></div><div><dt>Supporting standards</dt><dd>the connector standard (DS-009), the autopilot bus standard (DS-010) and the payload bus standard (DS-014); a smart battery standard is in draft</dd></div><div><dt>Note</dt><dd>Each fixes pin-outs, block diagrams, PCB layout guidelines and connector specifications</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Boards built to it</h4><ul class="hw-rows"><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c-mini?variant=44511519637693"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C Mini, Model-B, no power module</small></span><span class="hw-price">US$130.99</span></a></li><li><a class="hw-row" href="https://holybro.com/products/pix32-v6?variant=43020083527869"><span class="hw-main"><b>Holybro</b><small>Pix32 v6, FC module only</small></span><span class="hw-price">US$146.99</span></a></li><li><a class="hw-row" href="https://holybro.com/products/pixhawk-6c?variant=42783569871037"><span class="hw-main"><b>Holybro</b><small>Pixhawk 6C, plastic case, no power module</small></span><span class="hw-price">US$165.99</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://pixhawk.org/standards/"><span class="hw-main"><b>Documentation</b><small>Pixhawk standards</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row" href="https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html"><span class="hw-main"><b>Crate</b><small><code>pamoja-mavlink</code></small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li><li><a class="hw-row guide" href="https://pamoja.molex.cloud/docs/guides/mavlink.html"><span class="hw-main"><b>MAVLink guide</b><small>MAVLink v1 and v2 framing, signing, named message fields</small></span><span class="hw-go" aria-hidden="true">&#8594;</span></a></li></ul></section>
</div>
</article>
<article class="hw-card" aria-labelledby="cortex-m4f">
<header class="hw-head">

### Cortex-M4 with FPU, bare metal {#cortex-m4f}

<p class="hw-by">Arm</p>
<p class="hw-summary">The bare-metal target: CI cross-compiles every no_std crate for thumbv7em-none-eabihf, which is this core with its floating-point unit and no operating system.</p>
</header>
<dl class="hw-facts"><div><dt>Architecture</dt><dd>Armv7-M</dd></div><div><dt>Floating point</dt><dd>a built-in single-precision FPU, credited with 10x on single-precision operations</dd></div><div><dt>DSP</dt><dd>integrated DSP, SIMD and MAC instructions</dd></div><div><dt>Designed for</dt><dd>industrial control, IoT sensors, AI and ML, audio</dd></div><div><dt>pamoja</dt><dd>the bare-metal job in CI builds pamoja-core, security, audit, update, zenoh, ros2 and mavlink for this target</dd></div></dl>
<div class="hw-foot">
<section class="hw-buy"><h4>Boards with this core</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.seeedstudio.com/Seeed-XIAO-BLE-nRF52840-p-5201.html"><span class="hw-main"><b>Seeed Studio</b><small>Seeed Studio XIAO nRF52840 (XIAO BLE)</small></span><span class="hw-price">US$9.99</span></a></li><li><a class="hw-row" href="https://www.adafruit.com/product/3800"><span class="hw-main"><b>Adafruit</b><small>Adafruit ItsyBitsy M4 Express featuring ATSAMD51</small></span><span class="hw-price">US$14.95</span></a></li><li><a class="hw-row" href="https://thepihut.com/products/stm32f411-blackpill-development-board"><span class="hw-main"><b>The Pi Hut</b><small>STM32F411 BlackPill Development Board (DFRobot DFR0864)</small></span><span class="hw-price">£15.90</span></a></li></ul><p class="hw-when">prices as listed on 2026-09-07</p></section>
<section class="hw-learn"><h4>Read and build</h4><ul class="hw-rows"><li><a class="hw-row" href="https://www.arm.com/products/silicon-ip-cpu/cortex-m/cortex-m4"><span class="hw-main"><b>Documentation</b><small>Arm Cortex-M4</small></span><span class="hw-go" aria-hidden="true">&#8599;</span></a></li></ul></section>
</div>
</article>
</div>
<!-- end -->

## Anything else with a driver

The parts above are the ones pamoja decodes itself. They are not the limit of what it can
talk to. Everything below the driver is a trait: implement
[`Sensor`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/device/trait.Sensor.html) or
[`Actuator`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/device/trait.Actuator.html) for
your own part and the kit, the profiles, and the ladder take it as they take the ones here. The
[your own device guide](guides/device.md) builds a probe and a valve pamoja has never heard of and
runs them, in all four languages, with nothing plugged in.

The same holds for transports. A part reachable over I2C, SPI, a serial line, RS485, or CAN is
reachable through the crates on [Buses and links](buses.md), whether or not its decoder ships
here.
