# Running a profile

A profile says what a node does: what it reads, when it switches its output, where
it reports, and how often it samples as its battery drains. A wiring file says where
that happens: the part on this site's bus, the line its relay is on, and the broker
down the hall. `pamoja-node` reads the two and runs the node, with no program to
write. Every coop runs the same profile, and each coop has a wiring file of its own.

## Install

```sh
cargo install pamoja-profile --features runner
```

From a checkout of the repository, `cargo install --path crates/pamoja-profile
--features runner` builds the same binary. The runner reaches real parts through the
Linux kernel's I2C, GPIO, and 1-Wire files, so it runs them on a Raspberry Pi or any
Linux board. A replayed reading, a simulated part, and a printed output and link work
on every system, which is how the next section tries a profile on a laptop.

## Try it with nothing wired

Fetch a profile from the [catalog](profiles.md), and write a wiring file beside it
that plays back three readings and prints what the node does:

```sh
curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/brooder-heater.json
```

```json
{
  "$schema": "https://pamoja.molex.cloud/schema/wiring-1.json",
  "site": "coop-2",
  "profile": "brooder-heater.json",
  "sensor": { "part": "replay", "readings": [27.5, 31.8, 32.6] },
  "output": { "print": "heat lamp" },
  "link": { "print": true }
}
```

```sh
pamoja-node try.json --fast
```

```text
coop-2 runs brooder-heater: temperature in celsius from 3 replayed readings, the heat lamp printed, reporting on poultry/brooder/temperature over print
-> poultry/brooder/temperature 27.5
27.5 celsius, heat lamp on, alert OutOfRange
-> poultry/brooder/temperature 31.8
31.8 celsius, heat lamp on
-> poultry/brooder/temperature 32.6
32.6 celsius, heat lamp off
the sensor has no more readings
```

The profile holds the brooder at 32 C with the lamp on below 31.5 and off above 32.5,
so 31.8 leaves the lamp on and 32.6 turns it off. `--fast` ticks again at once rather
than waiting the two minutes the profile asks for. A `"bus": "sim"` in place of the
replay reads a part's simulated twin through its real driver, one fixed measurement at
a time.

## Wire a site

The same profile on a real coop: a BME280 at address `0x77` on the header's I2C bus,
a relay board on GPIO 17 that switches on when the line is driven low, and a broker on
the local network. The [Raspberry Pi page](boards/raspberry-pi.md#wiring-a-bme280)
shows the wires.

```json
{
  "$schema": "https://pamoja.molex.cloud/schema/wiring-1.json",
  "site": "coop-2",
  "profile": "brooder-heater.json",
  "sensor": { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x77" },
  "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
  "link": { "mqtt": "192.168.1.10" }
}
```

`--check` reads both files, says what would run, and opens nothing:

```sh
pamoja-node coop-2.json --check
```

```text
coop-2 runs brooder-heater: temperature in celsius from a bme280 at 0x77 on /dev/i2c-1, the output on /dev/gpiochip0 line 17, reporting on poultry/brooder/temperature over mqtt 192.168.1.10:1883
```

Without it, the node runs until it is stopped. A tick that fails, a loose wire or a
broker that went away, is reported and tried again at the next interval rather than
ending the node.

A part that measures in another unit is converted to the profile's: a BME280 reads
celsius, and a profile that reads fahrenheit gets fahrenheit. The runner converts
temperature between celsius, fahrenheit, and kelvin, pressure between pascal,
hectopascal, millibar, kilopascal, and bar, and voltage between volt and millivolt. A
probe that reads high or low is corrected with the sensor's `scale` and `offset`,
which apply once the reading is in the profile's unit.

## Run it at boot

A node has to come back after a power cut without anyone logging in, which on Linux
means a systemd unit. Copy the binary somewhere every account can run it, put the two
files under `/etc/pamoja`, and write the unit to
`/etc/systemd/system/pamoja-node.service`:

```sh
sudo install ~/.cargo/bin/pamoja-node /usr/local/bin/
```

```ini
[Unit]
Description=pamoja node
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/pamoja-node /etc/pamoja/coop-2.json
Restart=on-failure
RestartSec=10
DynamicUser=yes
SupplementaryGroups=i2c gpio

[Install]
WantedBy=multi-user.target
```

`sudo systemctl enable --now pamoja-node` starts it now and at every boot, and
`journalctl -u pamoja-node -f` follows the lines it prints. The node runs as an
account of its own in the `i2c` and `gpio` groups, so it never needs root.

## On a battery

A node on a panel and a battery reads the battery's voltage through a power monitor,
and the profile's power schedule sets how long it waits between readings as the charge
drops. Name the monitor and the battery's empty and full voltages:

```json
"battery": { "part": "ina219", "bus": "/dev/i2c-1", "empty_volts": 11.8, "full_volts": 12.7 }
```

The charge is read as a straight line between the two voltages. A monitor that does not
answer is taken as a critical battery, so a fault makes the node sample less rather than
more. Without a `battery`, the node is on mains and samples at the profile's active
interval.

## Parts

The parts the runner reads, the quantities each measures as a profile's `reads` names
them, and where each is found. The [sensor drivers guide](guides/sensors.md) has the
datasheet behind each one.

<!-- table: runner parts -->
| `part` | Measures, as `reads` names it | On | Usual address |
| --- | --- | --- | --- |
| `bme280` | `temperature` in `celsius`, `relative_humidity` in `percent`, `pressure` in `hectopascal` | an I2C `bus` | `0x76` |
| `bmp280` | `temperature` in `celsius`, `pressure` in `hectopascal` | an I2C `bus` | `0x76` |
| `sht3x` | `temperature` in `celsius`, `relative_humidity` in `percent` | an I2C `bus` | `0x44` |
| `hdc1080` | `temperature` in `celsius`, `relative_humidity` in `percent` | an I2C `bus` | `0x40` |
| `tmp117` | `temperature` in `celsius` | an I2C `bus` | `0x48` |
| `scd4x` | `co2` in `ppm`, `temperature` in `celsius`, `relative_humidity` in `percent` | an I2C `bus` | `0x62` |
| `opt3001` | `illuminance` in `lux` | an I2C `bus` | `0x44` |
| `ina219` | `voltage` in `volt` | an I2C `bus` | `0x40` |
| `ina226` | `voltage` in `volt` | an I2C `bus` | `0x40` |
| `ds18b20` | `temperature` in `celsius` | the kernel's 1-Wire files, by `serial` | - |
| `replay` | whatever the profile reads, from its `readings` | nothing | - |
<!-- end -->

## What it refuses

Both files are checked before any part is opened, and each refusal says what to change:

- A profile that reads what the part does not measure: *the profile `soil-moisture-valve`
  reads soil_moisture, and a bme280 measures temperature, relative_humidity, or pressure*.
- A misspelled field, with the nearest one it knows: *unknown field `outptu`, did you
  mean `output`?*
- A profile that switches an output with no `output` wired, or an `output` for a profile
  that switches nothing.
- A unit the runner does not convert, an address past `0x7f`, a password without a
  username, a client certificate without its key, and a battery whose full voltage is
  not above its empty one.
- A profile whose control kind is not built in. A
  [kind of your own](guides/profile.md#profile-custom) is decided by code, so it runs in
  a program of its own that registers that code, as the
  [profile guide](guides/profile.md) shows.

## Every field

The wiring file has a published [JSON Schema](https://pamoja.molex.cloud/schema/wiring-1.json),
named by its `$schema` field, so an editor completes the fields and marks a wrong one as
it is typed. The runner's own check is the last word, since it also knows which parts
measure what.

<!-- table: schema wiring -->
### The wiring file {#wiring-fields}

One site's wiring: the part that reads what a profile reads, the line its output drives, the link its readings go over, and the battery it runs from. pamoja-node reads it with the profile it names and runs the node.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `$schema` | text | no | The format the file is written in: this schema's address, or a copy of it by the same file name. An editor reads it to check the file as it is typed. |
| `site` | text | yes | The site's name, such as coop-2, which the node's logs and its MQTT client id carry. |
| `profile` | text | yes | The profile to run, as a path relative to this file, such as brooder-heater.json. |
| `sensor` | object, see [sensor](#wiring-sensor) | yes | The part that takes the readings. It must measure the quantity the profile reads. |
| `output` | object, [gpio](#wiring-gpio) or [print](#wiring-print-output) | no | The output a setpoint profile switches: a GPIO line, or a printed line for trying a profile with nothing wired. Leave it out for a profile that switches nothing. |
| `link` | object, [mqtt](#wiring-mqtt) or [print](#wiring-print-link) | yes | The link each reading is published over. |
| `battery` | object, see [battery](#wiring-battery) | no | The battery the node runs from, read as a voltage through a power monitor to set how often the node samples. Leave it out on mains power, and the node samples at the profile's active cadence. |

### sensor {#wiring-sensor}

The part that takes the readings. It must measure the quantity the profile reads.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `part` | one of `bme280`, `bmp280`, `sht3x`, `hdc1080`, `tmp117`, `scd4x`, `opt3001`, `ina219`, `ina226`, `ds18b20`, `replay` | yes | The part, as the table of parts names it. |
| `bus` | text | no | The I2C bus the part is on, such as /dev/i2c-1, or sim for the part's simulated twin, which answers with one fixed measurement. Every part but a ds18b20 and a replay needs one. |
| `address` | whole number, at least 0, at most 127, or text | no | The part's I2C address, as a number or as hexadecimal text such as "0x77", when it is not the part's usual one. |
| `serial` | text | no | A ds18b20's 1-Wire serial, such as 28-0316a2795cff, as the kernel names its folder under /sys/bus/w1/devices. |
| `readings` | list of number | no | The readings a replay plays back in turn, in the profile's unit, for trying a profile with nothing wired. |
| `offset` | number | no, `0` | Added to each reading once it is in the profile's unit, to correct a probe that reads high or low. |
| `scale` | number | no, `1` | Multiplied into each reading before the offset is added. |

### output, gpio {#wiring-gpio}

A GPIO line, such as the input of a relay board.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `gpio` | text | yes | The GPIO chip, such as /dev/gpiochip0. |
| `line` | whole number, at least 0 | yes | The line on the chip, which on a Raspberry Pi is the GPIO number, such as 17. |
| `active_low` | true or false | no, `false` | true when the line is driven low to switch the output on, as most relay boards want. |

### output, print {#wiring-print-output}

Names the output in each tick's line instead of driving a line.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `print` | text | yes | What the output is called in the text, such as heat lamp. |

### link, mqtt {#wiring-mqtt}

An MQTT broker.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `mqtt` | text | yes | The broker's host name or address. |
| `port` | whole number, at least 1, at most 65535 | no | The broker's port: 1883 unless given, or 8883 with tls. |
| `client_id` | text | no | The client id; the site's name unless given. |
| `username` | text | no | The username to sign in with. |
| `password` | text | no | The password to sign in with, which goes with a username. |
| `tls` | object, see [link, tls](#wiring-tls) | no | TLS to the broker. An empty object trusts the system's certificate authorities. |

### link, print {#wiring-print-link}

Prints each reading with its topic instead of publishing it.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `print` | `true` | yes | true. |

### link, tls {#wiring-tls}

TLS to the broker. An empty object trusts the system's certificate authorities.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `ca` | text | no | The certificate authority to trust, as a PEM file; the system's own unless given. |
| `certificate` | text | no | The client certificate to present, as a PEM file, for a broker that asks for one. |
| `key` | text | no | The client certificate's private key, as a PEM file, which goes with a certificate. |

### battery {#wiring-battery}

The battery the node runs from, read as a voltage through a power monitor to set how often the node samples. Leave it out on mains power, and the node samples at the profile's active cadence.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `part` | one of `ina219`, `ina226` | yes | The power monitor across the battery. |
| `bus` | text | yes | The I2C bus the monitor is on, or sim. |
| `address` | whole number, at least 0, at most 127, or text | no | The monitor's I2C address, when it is not 0x40. |
| `empty_volts` | number | yes | The voltage the battery reads when it is empty. |
| `full_volts` | number | yes | The voltage the battery reads when it is full, above empty_volts. |
<!-- end -->

## Where next

- [Profiles](profiles.md): the catalog of profiles to run.
- [Device profiles](guides/profile.md): what a profile holds, and a node in your own
  program in any of the four languages.
- [Raspberry Pi](boards/raspberry-pi.md): the board, its buses, and the wiring.
- [Sensor drivers](guides/sensors.md): every part, from every language.
