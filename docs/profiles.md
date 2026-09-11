# Profiles

A profile is a node written down as data: what it publishes on, which policy it
applies to each reading, how often it samples as its battery drains, and how a
dashboard should draw it. The [device profiles guide](guides/profile.md) loads one
and runs it in the four languages. This page is the catalog of the profiles the
project and its community share. Each is a JSON file under `profiles/` in the
repository, read by the same parser a device uses, checked in CI for the mistakes a
hand-written manifest makes, and kept in the form the library itself writes, so a
change to one shows as a change of meaning and nothing else.

## Using one

Download the file, or copy its text, and load it where the node runs. In Rust,
`Profile::from_json` reads it and `Node` runs it over a sensor, an actuator, and a
transport of your own. In TypeScript, Python, and C#, `fromJson`, `from_json`, and
`FromJson` read it, and its controller decides each reading while the program
drives the hardware, the loop the guide shows. The numbers are a starting point: a
manifest is meant to be edited to the bed, the fridge, or the river in front of
you, and shared back once it has run.

## Sharing yours

A profile that has run on a real node is worth sharing. The
[community page](community.md#share-a-profile) says how: one file, one pull
request, no Rust. `cargo xtask profiles` checks it the way CI will, and the four
presets the library ships are here in the same form, so a manifest and the code
that would have built it cannot drift apart.

<!-- table: profiles -->
<div class="pkgs">
<div class="pkg stack" id="profile-brooder-heater">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/brooder-heater.json">brooder-heater</a><code class="pkg-import">profiles/brooder-heater.json</code><p>Keeps a poultry brooder at 32 C by switching a heat lamp on below 31.5 C and off above 32.5 C, and alerts when the chicks are more than 4 C from target. Samples every two minutes on a healthy battery and eases off as it drains.</p><ul class="pkg-proves"><li>Publishes on <code>poultry/brooder/temperature</code>.</li><li>Holds 32 by switching the output on below 31.5 and off above 32.5, and alerts outside 28 to 36.</li><li>Samples every 2 min, every 10 min below 50 % charge, and every 30 min below 20 %.</li><li>Draws Brooder temperature as a thermometer in celsius with a safe band of 28 to 36; Heat lamp as a switch in state.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/brooder-heater.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/brooder-heater.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-flood-sensor">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/flood-sensor.json">flood-sensor</a><code class="pkg-import">profiles/flood-sensor.json</code><p>Watches a river gauge and warns when the level rises more than 0.3 m in one sample, the signature of a flash flood. Samples every minute, since a flood gives little warning.</p><ul class="pkg-proves"><li>Publishes on <code>water/river/level</code>.</li><li>Warns when the reading rises by more than 0.3 in one sample.</li><li>Samples every 1 min, every 5 min below 50 % charge, and every 15 min below 20 %.</li><li>Draws River level as a wave in meter with a safe band of 0.2 to 2.5; Rainfall as a bar in millimeter with a safe band of 0 to 25.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/flood-sensor.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/flood-sensor.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-grain-store-humidity">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/grain-store-humidity.json">grain-store-humidity</a><code class="pkg-import">profiles/grain-store-humidity.json</code><p>Reports the relative humidity in a grain store without driving anything, drawn as a droplet with a safe band up to 65 %, above which stored cereal takes on the moisture that mold grows in. Samples every 10 minutes, since a store changes slowly.</p><ul class="pkg-proves"><li>Publishes on <code>storage/grain/humidity</code>.</li><li>Reports readings and drives nothing.</li><li>Samples every 10 min, every 1 hour below 50 % charge, and every 2 hours below 20 %.</li><li>Draws Store humidity as a droplet in percent with a safe band of 30 to 65; Store temperature as a thermometer in celsius with a safe band of 5 to 30.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/grain-store-humidity.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/grain-store-humidity.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-irrigation-node">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/irrigation-node.json">irrigation-node</a><code class="pkg-import">profiles/irrigation-node.json</code><p>Opens an irrigation valve when soil moisture falls below 30 % and closes it again above 40 %, and alerts when the soil dries below 10 % or is waterlogged above 60 %. Samples slowly, since soil changes over hours and the battery has to last.</p><ul class="pkg-proves"><li>Publishes on <code>farm/irrigation/soil-moisture</code>.</li><li>Holds 35 by switching the output on below 30 and off above 40, and alerts outside 10 to 60.</li><li>Samples every 5 min, every 30 min below 50 % charge, and every 1 hour below 20 %.</li><li>Draws Soil moisture as a droplet in percent with a safe band of 10 to 60; Drip valve as a valve in state.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/irrigation-node.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/irrigation-node.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-pipeline-pressure-drop">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/pipeline-pressure-drop.json">pipeline-pressure-drop</a><code class="pkg-import">profiles/pipeline-pressure-drop.json</code><p>Watches the pressure in a water main and warns when it falls by more than 0.5 bar between samples, the signature of a burst or a valve left open downstream. Samples every 30 s, since a burst empties a tank in minutes.</p><ul class="pkg-proves"><li>Publishes on <code>water/main/pressure</code>.</li><li>Warns when the reading falls by more than 0.5 in one sample.</li><li>Samples every 30 s, every 2 min below 40 % charge, and every 10 min below 15 %.</li><li>Draws Main pressure as a dial in bar with a safe band of 2 to 6.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/pipeline-pressure-drop.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/pipeline-pressure-drop.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-soil-moisture-valve">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/soil-moisture-valve.json">soil-moisture-valve</a><code class="pkg-import">profiles/soil-moisture-valve.json</code><p>Waters a raised bed through a solenoid valve, opening it when the volumetric water content falls below 25 % and closing it above 35 %, and alerts when the bed is drier than 15 % or wetter than 45 %. The dashboard draws the bed as a droplet and the valve as a valve, with French and Swahili labels.</p><ul class="pkg-proves"><li>Publishes on <code>garden/bed/moisture</code>.</li><li>Holds 30 by switching the output on below 25 and off above 35, and alerts outside 15 to 45.</li><li>Samples every 5 min, every 30 min below 50 % charge, and every 1 hour below 20 %.</li><li>Draws Soil moisture as a droplet in percent with a safe band of 25 to 45; Bed valve as a valve in state.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/soil-moisture-valve.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/soil-moisture-valve.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-vaccine-fridge-monitor">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/vaccine-fridge-monitor.json">vaccine-fridge-monitor</a><code class="pkg-import">profiles/vaccine-fridge-monitor.json</code><p>Holds a vaccine fridge at 5 C by switching its cooler, and raises an alert the moment the temperature leaves the 2 to 8 C safe range. Keeps sampling often as the battery drains, since an unnoticed excursion costs more than a flat battery.</p><ul class="pkg-proves"><li>Publishes on <code>cold-chain/fridge/temperature</code>.</li><li>Holds 5 by switching the output on above 5.5 and off below 4.5, and alerts outside 2 to 8.</li><li>Samples every 1 min, every 5 min below 50 % charge, and every 15 min below 20 %.</li><li>Draws Fridge temperature as a thermometer in celsius with a safe band of 2 to 8; Cooler as a switch in state; Compressor duty as a bar in percent with a safe band of 0 to 60, a node stat.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/vaccine-fridge-monitor.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/vaccine-fridge-monitor.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
<div class="pkg stack" id="profile-well-level">
<div class="pkg-head">
<div class="pkg-what"><a class="pkg-title" href="https://github.com/molexxxx/pamoja/blob/main/profiles/well-level.json">well-level</a><code class="pkg-import">profiles/well-level.json</code><p>Reports a well's water level and warns once the level is on course to reach the dry mark within six more samples, so a pump is stopped before it runs dry.</p><ul class="pkg-proves"><li>Publishes on <code>water/well/level</code>.</li><li>Watches a falling level and warns once it is on course to reach 0.5 within 6 more samples.</li><li>Samples every 10 min, every 30 min below 50 % charge, and every 1 hour below 20 %.</li><li>Draws Well level as a bar in meter with a safe band of 1 to 6; Battery as a battery in volt with a safe band of 3.5 to 4.3, a node stat.</li></ul></div>
<div class="pkg-get"><code class="cmd">curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/well-level.json</code><button class="copy" type="button" data-copy="curl -O https://raw.githubusercontent.com/molexxxx/pamoja/main/profiles/well-level.json" aria-label="Copy the install command">copy</button></div>
</div>
</div>
</div>
<!-- end -->
