//! The no-JavaScript floor: a server-rendered, meta-refreshing status table.
//!
//! The smallest tier serves the embedded [`lite.html`](../web/lite.html), which renders the
//! fleet with a tiny script. When scripting is off entirely, that page bounces to `GET
//! /lite`, served from here: the same readable status table built once on the device and
//! refreshed by a `<meta http-equiv="refresh">`, with no script at all. It is plain, but it
//! is legible and it works on any browser. This is the only place the device formats HTML at
//! runtime, kept to a single small table on purpose.

use crate::state::{Reading, State, Status};

/// How often the no-script page reloads itself, in seconds.
const REFRESH_SECS: u32 = 15;

/// Renders the fleet snapshot as a complete, self-contained, no-script HTML page.
///
/// The page carries its own styles inline and a `<meta http-equiv="refresh">`, so it needs
/// no other asset and stays current without any client code.
///
/// # Arguments
///
/// * `state` - the fleet snapshot to render.
///
/// # Returns
///
/// A full HTML document as a string.
pub(crate) fn render_lite(state: &State) -> String {
    let (word, sym, class) = status_bits(state.status);
    let mut out = String::with_capacity(2048);
    out.push_str(&page_head());
    out.push_str(&format!(
        "<header><h1>pamoja</h1><p class=\"status {class}\">{sym} {word}</p></header>\n"
    ));

    let mut any = false;
    for org in &state.orgs {
        any = true;
        let org_status = org
            .groups
            .iter()
            .map(|group| group.status)
            .fold(Status::Ok, Status::worst);
        let (oword, osym, oclass) = status_bits(org_status);
        let groups = org.groups.len();
        out.push_str(&format!(
            "<section class=\"org\">\n\
             <h2>{name} <span class=\"badge {oclass}\">{osym} {oword}</span> \
             <span class=\"count\">{groups} group{gp}</span></h2>\n",
            name = esc(&org.name),
            gp = if groups == 1 { "" } else { "s" },
        ));
        for group in &org.groups {
            let (gword, gsym, gclass) = status_bits(group.status);
            let online = if group.link.online {
                "online"
            } else {
                "offline"
            };
            let sensors = group.sensors.iter().filter(|s| !s.reading.stat).count();
            out.push_str(&format!(
                "<section class=\"group\">\n\
                 <h3>{name} <span class=\"badge {gclass}\">{gsym} {gword}</span></h3>\n\
                 <p class=\"muted\">{kind} <span class=\"bars\">{bars}</span> \u{b7} {online} \u{b7} {sensors} sensor{sp}</p>\n",
                name = esc(&group.name),
                kind = esc(link_kind(group.link.kind)),
                bars = bars(group.link.strength),
                sp = if sensors == 1 { "" } else { "s" },
            ));
            if sensors > 0 {
                out.push_str(
                    "<table><thead><tr><th>Sensor</th><th class=\"reading\">Reading</th><th class=\"state\">Status</th></tr></thead><tbody>\n",
                );
                for sensor in group.sensors.iter().filter(|s| !s.reading.stat) {
                    let (sword, ssym, sclass) = status_bits(sensor.reading.status);
                    out.push_str(&format!(
                        "<tr><th scope=\"row\">{id}</th><td class=\"reading\">{reading}</td><td class=\"state {sclass}\">{ssym} {sword}</td></tr>\n",
                        id = esc(&sensor.id),
                        reading = reading_cell(&sensor.reading),
                    ));
                }
                out.push_str("</tbody></table>\n");
            }
            let stats: Vec<String> = group
                .sensors
                .iter()
                .filter(|s| s.reading.stat)
                .map(|s| {
                    let name = s
                        .reading
                        .label
                        .clone()
                        .unwrap_or_else(|| label(&s.reading.key));
                    format!("{} {}", esc(&name), stat_text(&s.reading))
                })
                .collect();
            if !stats.is_empty() {
                out.push_str(&format!(
                    "<p class=\"muted stats\">{}</p>\n",
                    stats.join(" \u{b7} ")
                ));
            }
            out.push_str("</section>\n");
        }
        out.push_str("</section>\n");
    }
    if !any {
        out.push_str("<p class=\"muted\">No sensors are reporting yet.</p>\n");
    }

    out.push_str("<footer><a href=\"./\">Full dashboard</a></footer>\n</body></html>\n");
    out
}

/// Renders a minimal page for the rare case the snapshot cannot be read.
///
/// # Returns
///
/// A full HTML document reporting that the status is briefly unavailable.
pub(crate) fn render_unavailable() -> String {
    format!(
        "{}<header><h1>pamoja</h1><p class=\"status warn\">\u{25B2} Status unavailable</p></header>\n</body></html>\n",
        page_head()
    )
}

// The shared document head: doctype, meta-refresh, and the inline styles. The styles mirror
// lite.html so the scripted and no-script floors read the same.
fn page_head() -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\" dir=\"ltr\">\n<head>\n\
         <meta charset=\"utf-8\" />\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
         <meta name=\"color-scheme\" content=\"light dark\" />\n\
         <meta http-equiv=\"refresh\" content=\"{REFRESH_SECS}\" />\n\
         <title>pamoja status</title>\n<style>{STYLE}</style>\n</head>\n<body>\n",
    )
}

// Word, shape, and color class for a status: redundant encoding so it never relies on color.
fn status_bits(status: Status) -> (&'static str, &'static str, &'static str) {
    match status {
        Status::Alarm => ("ALARM", "\u{2715}", "alarm"),
        Status::Warn => ("WARN", "\u{25B2}", "warn"),
        Status::Ok => ("OK", "\u{2713}", "ok"),
    }
}

// The reading cell: the leaf of a discrete state code, or a rounded value with its unit
// abbreviated to a short symbol and set in a muted span, so the value reads first.
fn reading_cell(reading: &Reading) -> String {
    if let Some(state) = &reading.state {
        let leaf = state.rsplit('.').next().unwrap_or(state);
        return esc(leaf);
    }
    let rounded = (reading.value as f64 * 100.0).round() / 100.0;
    let unit = unit_symbol(&reading.unit);
    if unit.is_empty() {
        format!("{rounded}")
    } else {
        format!("{rounded} <span class=\"unit\">{}</span>", esc(unit))
    }
}

// Abbreviates a canonical unit name to its conventional short symbol, so a reading reads
// like an instrument ("6.79 °C", "5.14 L/min") rather than carrying a long wire label. An
// unmapped unit falls back to its raw name; a non-physical unit (count, record, state)
// renders as no symbol at all.
fn unit_symbol(unit: &str) -> &str {
    match unit {
        "celsius" => "\u{b0}C",
        "fahrenheit" => "\u{b0}F",
        "kelvin" => "K",
        "percent" => "%",
        "volt" => "V",
        "millivolt" => "mV",
        "ampere" => "A",
        "milliampere" => "mA",
        "watt" => "W",
        "kilowatt" => "kW",
        "watt_hour" => "Wh",
        "kilowatt_hour" => "kWh",
        "hectopascal" => "hPa",
        "pascal" => "Pa",
        "kilopascal" => "kPa",
        "meter" => "m",
        "millimeter" => "mm",
        "centimeter" => "cm",
        "kilometer" => "km",
        "meter_per_second" => "m/s",
        "kilometer_per_hour" => "km/h",
        "liter" => "L",
        "liter_per_minute" => "L/min",
        "liter_per_hour" => "L/h",
        "lux" => "lx",
        "decibel" => "dB",
        "hertz" => "Hz",
        "degree" => "\u{b0}",
        "count" | "record" | "state" => "",
        other => other,
    }
}

// A signal-strength meter drawn as filled and empty dots over 0..=4, so it reads without
// color and does not look like a sensor count the way a bare "3/4" can.
fn bars(strength: u8) -> String {
    let filled = usize::from(strength.min(4));
    (0..4)
        .map(|i| if i < filled { '\u{25CF}' } else { '\u{25CB}' })
        .collect()
}

// A node or network stat as short text for the stats line: the state leaf, or the bare
// rounded value with no unit, since a stat like a neighbor count reads better plain.
fn stat_text(reading: &Reading) -> String {
    if let Some(state) = &reading.state {
        let leaf = state.rsplit('.').next().unwrap_or(state);
        return esc(leaf);
    }
    let rounded = (reading.value as f64 * 100.0).round() / 100.0;
    format!("{rounded}")
}

// Turns a stable reading key into a short label for the stats line, such as
// `"messages_relayed"` into `"messages relayed"`.
fn label(key: &str) -> String {
    key.replace('_', " ")
}

// A short, stable English label for a link kind. The floor page is single-locale by design.
fn link_kind(kind: crate::state::LinkKind) -> &'static str {
    use crate::state::LinkKind::*;
    match kind {
        Lora => "LoRa",
        Wifi => "Wi-Fi",
        Cellular => "Cellular",
        NbIot => "NB-IoT",
        Satellite => "Satellite",
        Ethernet => "Ethernet",
        Mesh => "Mesh",
    }
}

// Escapes the characters that would otherwise break out of HTML text content.
fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const STYLE: &str = "\
:root{--bg:#faf5eb;--tint:#ede3d2;--card:#faf5eb;--line:#d4cdc1;--frame:#141414;--text:#141414;--muted:#585856;--ok:#0f5f56;--warn:#8a5a00;--alarm:#a5321a;color-scheme:light dark}\
@media(prefers-color-scheme:dark){:root{--bg:#191a17;--tint:#22231f;--card:#191a17;--line:#3a3b35;--frame:#82857b;--text:#eae8e1;--muted:#a8a69d;--ok:#5ec6b0;--warn:#e8b44a;--alarm:#ef8368}}\
*{box-sizing:border-box}\
body{margin:0;padding:1rem;background:var(--bg);color:var(--text);font:16px/1.6 system-ui,-apple-system,Segoe UI,Roboto,sans-serif}\
header{display:flex;align-items:baseline;justify-content:space-between;gap:1rem;flex-wrap:wrap;padding-bottom:.7rem;margin-bottom:1.2rem;border-bottom:1.5px solid var(--frame)}\
h1{margin:0;font-size:1.3rem;letter-spacing:-.01em}\
h2{margin:2rem 0 .6rem;font-size:1.1rem;display:flex;align-items:center;gap:.5rem;flex-wrap:wrap;border-top:1.5px solid var(--frame);padding-top:.5rem}\
h3{margin:1rem 0 .4rem;font-size:.95rem;display:flex;align-items:center;gap:.5rem;flex-wrap:wrap}\
.status{font-weight:700;font-size:1.05rem}\
.badge{font-size:.7rem;font-weight:700;text-transform:uppercase;letter-spacing:.09em;padding:.1rem .45rem;border:1px solid currentColor}\
.ok{color:var(--ok)}.warn{color:var(--warn)}.alarm{color:var(--alarm)}\
.muted{color:var(--muted);font-size:.85rem;margin:.1rem 0}\
.count{color:var(--muted);font-size:.78rem;font-weight:400}\
.bars{letter-spacing:.12em;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}\
.stats{font-size:.8rem;margin-top:.35rem;color:var(--muted)}\
section.group{margin:.2rem 0 1.1rem}\
table{width:100%;table-layout:fixed;border-collapse:collapse;background:var(--card);border-top:1.5px solid var(--frame);border-bottom:1.5px solid var(--frame)}\
th,td{text-align:start;padding:.45rem .6rem;border-bottom:1px solid var(--line)}\
th.reading,td.reading,th.state,td.state{text-align:end}\
th.reading{width:34%}th.state{width:22%}\
.unit{color:var(--muted)}\
thead th{font-size:.7rem;text-transform:uppercase;letter-spacing:.09em;color:var(--muted);background:var(--tint);border-bottom:1px solid var(--frame)}\
tbody tr:last-child th,tbody tr:last-child td{border-bottom:0}\
tbody th{font-weight:600}\
td.ok,td.warn,td.alarm{font-weight:700;white-space:nowrap;font-variant-numeric:tabular-nums}\
footer{margin-top:2rem;padding-top:.7rem;border-top:1.5px solid var(--frame);color:var(--muted);font-size:.85rem}a{color:inherit}";

#[cfg(test)]
mod tests {
    use super::*;
    // The demo fleet is used only by the two tests below, so its import and they are behind
    // the `mock` feature; the direct-`State` render tests run in the default build.
    #[cfg(feature = "mock")]
    use crate::{Mock, Scenario, StateSource};

    #[cfg(feature = "mock")]
    #[test]
    fn renders_a_complete_document_with_meta_refresh() {
        let html = render_lite(&Mock::new(Scenario::Normal).snapshot());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("http-equiv=\"refresh\""));
        assert!(html.trim_end().ends_with("</html>"));
        // No script tag at all: this is the no-JavaScript floor.
        assert!(!html.contains("<script"));
    }

    #[cfg(feature = "mock")]
    #[test]
    fn an_alarm_fleet_shows_the_alarm_word_not_just_a_color() {
        let html = render_lite(&Mock::new(Scenario::Alarm).snapshot());
        assert!(
            html.contains("ALARM"),
            "the status word must be present, not color alone"
        );
    }

    #[test]
    fn a_reading_shows_its_rounded_value_and_unit() {
        let mut state = State {
            orgs: vec![crate::Org {
                id: "o".into(),
                name: "Clinic".into(),
                groups: vec![crate::Group {
                    id: "g".into(),
                    name: "Cold chain".into(),
                    link: crate::Link {
                        kind: crate::LinkKind::Lora,
                        strength: 3,
                        online: true,
                    },
                    status: Status::Ok,
                    sensors: vec![crate::Sensor::new(
                        "fridge-1",
                        Reading::new("temperature", 6.789, "celsius"),
                    )],
                    lat: None,
                    lon: None,
                }],
            }],
            status: Status::Ok,
            uptime_secs: None,
            demo: false,
        };
        state.recompute_status();
        let html = render_lite(&state);
        assert!(html.contains("Clinic"), "the org is a section header");
        assert!(html.contains("fridge-1"));
        assert!(
            html.contains("6.79") && html.contains("\u{b0}C"),
            "value rounds to two places with an abbreviated unit: {html}"
        );
        // The link line names the kind and the sensor count, drawn without a bare "3/4".
        assert!(html.contains("LoRa"));
        assert!(html.contains("online"));
        assert!(
            html.contains("1 sensor"),
            "one measurement is counted: {html}"
        );
    }

    #[test]
    fn stats_are_listed_apart_from_sensors_and_not_counted() {
        // A group with one real sensor and two node/network stats: the count is one sensor,
        // and the stats appear on their own line rather than as sensor rows.
        let mut state = State {
            orgs: vec![crate::Org {
                id: "o".into(),
                name: "Mesh org".into(),
                groups: vec![crate::Group {
                    id: "g".into(),
                    name: "Relay".into(),
                    link: crate::Link {
                        kind: crate::LinkKind::Mesh,
                        strength: 4,
                        online: true,
                    },
                    status: Status::Ok,
                    sensors: vec![
                        crate::Sensor::new(
                            "river-1",
                            Reading::new("river_level", 1800.0, "millimeter"),
                        ),
                        crate::Sensor::new(
                            "neigh",
                            Reading::new("neighbors", 5.0, "count").as_stat(),
                        ),
                        crate::Sensor::new("hops", Reading::new("hops", 3.0, "count").as_stat()),
                    ],
                    lat: None,
                    lon: None,
                }],
            }],
            status: Status::Ok,
            uptime_secs: None,
            demo: false,
        };
        state.recompute_status();
        let html = render_lite(&state);
        assert!(html.contains("1 sensor"), "only the measurement is counted");
        assert!(html.contains("river-1"), "the measurement is a sensor row");
        // The stats show on the stats line by their key, not as sensor rows.
        assert!(html.contains("class=\"muted stats\""));
        assert!(html.contains("neighbors 5"));
        assert!(html.contains("hops 3"));
        assert!(
            !html.contains("scope=\"row\">neigh<"),
            "a stat is not a sensor row: {html}"
        );
    }

    #[test]
    fn names_are_html_escaped() {
        let html = render_lite(&State {
            orgs: vec![crate::Org {
                id: "o".into(),
                name: "Org".into(),
                groups: vec![crate::Group {
                    id: "g".into(),
                    name: "Acme & Co <x>".into(),
                    link: crate::Link {
                        kind: crate::LinkKind::Wifi,
                        strength: 4,
                        online: true,
                    },
                    status: Status::Ok,
                    sensors: Vec::new(),
                    lat: None,
                    lon: None,
                }],
            }],
            status: Status::Ok,
            uptime_secs: None,
            demo: false,
        });
        assert!(!html.contains("Acme & Co <x>"));
        assert!(html.contains("Acme &amp; Co &lt;x&gt;"));
    }

    #[test]
    fn a_discrete_state_shows_its_leaf() {
        let reading = Reading::new("valve", 0.0, "").with_state("state.open");
        assert_eq!(reading_cell(&reading), "open");
    }

    #[test]
    fn a_unit_is_abbreviated_to_its_symbol() {
        assert_eq!(
            reading_cell(&Reading::new("t", 5.0, "celsius")),
            "5 <span class=\"unit\">\u{b0}C</span>"
        );
        assert_eq!(
            reading_cell(&Reading::new("f", 5.14, "liter_per_minute")),
            "5.14 <span class=\"unit\">L/min</span>"
        );
    }
}
