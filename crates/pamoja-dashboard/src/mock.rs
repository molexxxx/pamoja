//! A scenario-driven mock fleet, so the whole dashboard runs with no hardware.
//!
//! The mock is the heart of the hardware-free development workflow. It implements
//! [`StateSource`] and serves a believable, deterministic multi-organization fleet, so
//! every state the UI must handle can be reproduced on demand instead of waited for in
//! the field. Readings drift on slow sine waves (real sensors wander gently rather than
//! jumping), and a [`Scenario`] injects a condition - an alarm, a sensor fault, a flat
//! battery, a dropped link, a cold start - into the otherwise healthy fleet so each
//! state is one click away.

use pamoja_profile::{ElementSpec, Presentation, Scope, Viz};

use crate::catalog::Catalog;
use crate::command::{Command, CommandError};
use crate::source::StateSource;
use crate::state::{
    EventLevel, EventRecord, Group, Link, LinkKind, Mode, Org, Reading, Sensor, State, Status,
    Trend,
};

/// A reproducible condition injected into the fleet for the dashboard to render.
///
/// Selectable from the command line and switchable live with `?scenario=`, so one
/// running dev server covers every case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// The whole fleet is healthy.
    Normal,
    /// A cold-chain fridge has drifted out of its safe band.
    Alarm,
    /// A silo probe has failed and reads an impossible value.
    SensorFault,
    /// A solar microgrid's batteries are nearly flat.
    LowBattery,
    /// A river-watch group has lost its uplink.
    LinkLost,
    /// The fleet has just booted, with little history yet.
    ColdStart,
}

impl Scenario {
    /// Every scenario, in a stable order for menus and test sweeps.
    pub const ALL: [Scenario; 6] = [
        Scenario::Normal,
        Scenario::Alarm,
        Scenario::SensorFault,
        Scenario::LowBattery,
        Scenario::LinkLost,
        Scenario::ColdStart,
    ];

    /// Returns the stable query-parameter key for this scenario.
    ///
    /// # Returns
    ///
    /// The lowercase identifier used in `?scenario=` and on the command line.
    pub fn key(self) -> &'static str {
        match self {
            Scenario::Normal => "normal",
            Scenario::Alarm => "alarm",
            Scenario::SensorFault => "sensor-fault",
            Scenario::LowBattery => "low-battery",
            Scenario::LinkLost => "link-lost",
            Scenario::ColdStart => "cold-start",
        }
    }

    /// Parses a scenario from its [`key`](Scenario::key).
    ///
    /// # Arguments
    ///
    /// * `key` - the scenario identifier, as used in `?scenario=`.
    ///
    /// # Returns
    ///
    /// The matching scenario, or `None` if `key` names none.
    pub fn from_key(key: &str) -> Option<Scenario> {
        Scenario::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// A deterministic, hardware-free fleet that serves a [`Scenario`].
///
/// Create one with [`Mock::new`], poll it through [`StateSource::snapshot`], and flip
/// scenarios live with [`Mock::set_scenario`]. Readings drift smoothly and repeatably,
/// because they are sine waves of the tick rather than real randomness.
///
/// # Examples
///
/// ```
/// use pamoja_dashboard::{Mock, Scenario, StateSource, Status};
///
/// let mut fleet = Mock::new(Scenario::Alarm);
/// let state = fleet.snapshot();
/// assert_eq!(state.status, Status::Alarm);
/// assert!(!state.orgs.is_empty());
/// ```
pub struct Mock {
    scenario: Scenario,
    tick: u64,
    slot: u32,
    // Actuator overrides applied to each snapshot, keyed by `"groupId/sensorId"`, so a
    // commanded valve stays where it was set across the deterministic drift.
    overrides: std::collections::HashMap<String, String>,
    // Values commanded onto numeric actuators, keyed the same way.
    settings: std::collections::HashMap<String, f32>,
    // Provisioning overlay: groups and sensors added or removed by authenticated commands,
    // so the device is the shared source of truth every client sees.
    added_groups: Vec<(String, Group)>,
    added_sensors: Vec<(String, Sensor)>,
    removed_groups: Vec<String>,
    removed_sensors: Vec<String>,
}

// How many history samples each sensor carries.
const HISTORY: usize = 32;

impl Mock {
    /// Creates a mock running `scenario` at tick zero.
    ///
    /// # Arguments
    ///
    /// * `scenario` - the condition to inject into the fleet.
    ///
    /// # Returns
    ///
    /// A mock with a deterministic drift sequence.
    pub fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            tick: 0,
            slot: 0,
            overrides: std::collections::HashMap::new(),
            settings: std::collections::HashMap::new(),
            added_groups: Vec::new(),
            added_sensors: Vec::new(),
            removed_groups: Vec::new(),
            removed_sensors: Vec::new(),
        }
    }

    /// Switches the scenario served from the next snapshot on.
    ///
    /// # Arguments
    ///
    /// * `scenario` - the new condition to inject.
    pub fn set_scenario(&mut self, scenario: Scenario) {
        self.scenario = scenario;
    }

    /// Returns the scenario currently being served.
    ///
    /// # Returns
    ///
    /// The active scenario.
    pub fn scenario(&self) -> Scenario {
        self.scenario
    }

    // A smooth value plus its recent history, deterministic for the current tick. Each
    // call advances a per-snapshot slot so sensors get distinct phases.
    fn series(&mut self, base: f32, amp: f32) -> (f32, Vec<f32>) {
        self.slot = self.slot.wrapping_add(1);
        let offset = self.slot as f32 * 1.7;
        let freq = 0.18;
        let samples = if self.scenario == Scenario::ColdStart {
            (self.tick as usize).min(HISTORY)
        } else {
            HISTORY
        };
        let mut history = Vec::with_capacity(samples);
        for i in 0..samples {
            let tk = self.tick as f32 - (samples as f32 - 1.0 - i as f32);
            history.push(base + amp * (tk * freq + offset).sin());
        }
        let current = *history.last().unwrap_or(&base);
        (current, history)
    }

    // Builds a sensor, deriving status and trend from the value and band.
    #[allow(clippy::too_many_arguments)]
    fn sensor(
        &mut self,
        id: &str,
        key: &str,
        unit: &str,
        base: f32,
        amp: f32,
        band: (f32, f32),
        battery: Option<f32>,
    ) -> Sensor {
        let (value, history) = self.series(base, amp);
        let (lo, hi) = band;
        let margin = (hi - lo) * 0.18;
        let status = if value < lo - margin || value > hi + margin {
            Status::Alarm
        } else if value < lo || value > hi {
            Status::Warn
        } else {
            Status::Ok
        };
        let trend = match history.as_slice() {
            [.., a, b] if b - a > amp * 0.08 => Trend::Rising,
            [.., a, b] if a - b > amp * 0.08 => Trend::Falling,
            _ => Trend::Steady,
        };
        let mut events = Vec::new();
        if status != Status::Ok {
            events.push(EventRecord {
                level: if status == Status::Alarm {
                    EventLevel::Error
                } else {
                    EventLevel::Warn
                },
                code: event_for(key),
                value: Some(value),
                age_secs: Some(5),
            });
        }
        events.push(EventRecord {
            level: EventLevel::Info,
            code: "reading.ok".to_owned(),
            value: Some(value),
            age_secs: Some(12),
        });
        Sensor {
            id: id.to_owned(),
            reading: Reading::new(key, value, unit)
                .with_status(status)
                .with_band(lo, hi)
                .with_trend(trend),
            battery,
            mode: if battery.is_some_and(|b| b < 0.2) {
                Mode::Critical
            } else if battery.is_some_and(|b| b < 0.5) {
                Mode::Saver
            } else {
                Mode::Active
            },
            history,
            events,
            peer: None,
            lat: None,
            lon: None,
        }
    }

    // A discrete (non-numeric) sensor rendered as a labeled chip, such as a valve or a
    // pump-health state. Carries a state code the page localizes, no band or history.
    fn chip_sensor(&self, id: &str, key: &str, state_code: &str, status: Status) -> Sensor {
        Sensor {
            id: id.to_owned(),
            reading: Reading::new(
                key,
                if state_code.ends_with("open") {
                    1.0
                } else {
                    0.0
                },
                "state",
            )
            .with_status(status)
            .with_state(state_code),
            battery: None,
            mode: Mode::Active,
            history: Vec::new(),
            events: vec![EventRecord {
                level: EventLevel::Info,
                code: "reading.ok".to_owned(),
                value: None,
                age_secs: Some(8),
            }],
            peer: None,
            lat: None,
            lon: None,
        }
    }

    // A mesh-map sensor (key "mesh_relay"): its value is the node count the preview/modal
    // draws. The same sensor type applies to any mesh group.
    fn mesh_sensor(&self, id: &str, nodes: f32) -> Sensor {
        Sensor {
            id: id.to_owned(),
            reading: Reading::new("mesh_relay", nodes, "state")
                .with_status(Status::Ok)
                .with_state("mesh.optimized"),
            battery: None,
            mode: Mode::Active,
            history: Vec::new(),
            events: vec![EventRecord {
                level: EventLevel::Info,
                code: "reading.ok".to_owned(),
                value: None,
                age_secs: Some(8),
            }],
            peer: None,
            lat: None,
            lon: None,
        }
    }

    fn group(
        &self,
        id: &str,
        name: &str,
        kind: LinkKind,
        strength: u8,
        sensors: Vec<Sensor>,
    ) -> Group {
        let mut group = Group {
            id: id.to_owned(),
            name: name.to_owned(),
            link: Link {
                kind,
                strength,
                online: true,
            },
            status: Status::Ok,
            sensors,
            lat: None,
            lon: None,
        };
        group.recompute_status();
        group
    }

    // Applies the provisioning overlay (added and removed groups and sensors) onto a
    // freshly built fleet, so authenticated provisioning persists and every client sees it.
    fn apply_edits(&self, state: &mut State) {
        if self.added_groups.is_empty()
            && self.added_sensors.is_empty()
            && self.removed_groups.is_empty()
            && self.removed_sensors.is_empty()
        {
            return;
        }
        for org in &mut state.orgs {
            org.groups.retain(|g| !self.removed_groups.contains(&g.id));
            for (target_org, group) in &self.added_groups {
                if *target_org == org.id && !self.removed_groups.contains(&group.id) {
                    org.groups.push(group.clone());
                }
            }
            for group in &mut org.groups {
                group.sensors.retain(|s| {
                    !self
                        .removed_sensors
                        .contains(&format!("{}/{}", group.id, s.id))
                });
                for (target_group, sensor) in &self.added_sensors {
                    let path = format!("{target_group}/{}", sensor.id);
                    if *target_group == group.id && !self.removed_sensors.contains(&path) {
                        group.sensors.push(sensor.clone());
                    }
                }
                group.recompute_status();
            }
        }
    }

    // Applies the recorded actuator overrides and settings onto a freshly built fleet, so
    // a commanded valve or pump holds what it was set to across the deterministic drift.
    fn apply_overrides(&self, state: &mut State) {
        if self.overrides.is_empty() && self.settings.is_empty() {
            return;
        }
        for org in &mut state.orgs {
            for group in &mut org.groups {
                for sensor in &mut group.sensors {
                    let target = format!("{}/{}", group.id, sensor.id);
                    if let Some(action) = self.overrides.get(&target) {
                        let allowed = sensor
                            .reading
                            .actions
                            .as_ref()
                            .is_some_and(|actions| actions.iter().any(|a| a == action));
                        if allowed {
                            sensor.reading.state = Some(format!("state.{action}"));
                            sensor.reading.value = if action == "open" { 1.0 } else { 0.0 };
                        }
                    }
                    if let Some(value) = self.settings.get(&target) {
                        if sensor.reading.range.is_some() {
                            sensor.reading.value = *value;
                            sensor.history.push(*value);
                        }
                    }
                }
                group.recompute_status();
            }
        }
    }

    /// The presentation catalog of the demo fleet: the field-kit elements the page ships
    /// no presets for, and where its network map places the demo sites.
    ///
    /// The dev server serves it at `GET /catalog` and the static showcase carries it in
    /// its first frame, so the demo fleet's own elements never have to be baked into the
    /// page. A real deployment serves its own from its profiles instead.
    ///
    /// # Returns
    ///
    /// The demo catalog.
    pub fn catalog() -> Catalog {
        let mesh = || Scope::Links(vec!["mesh".to_owned()]);
        let presentation = Presentation::new()
            .with_element(
                ElementSpec::new("well_level", "percent", "Well level", Viz::Bar)
                    .with_band(20.0, 100.0),
            )
            .with_element(
                ElementSpec::new("soil_trend", "percent", "Soil trend", Viz::Spark)
                    .with_band(0.0, 100.0),
            )
            .with_element(
                ElementSpec::new("drip_valve", "state", "Drip valve", Viz::Valve)
                    .with_state("state.closed"),
            )
            .with_element(
                ElementSpec::new("pump_speed", "percent", "Pump speed", Viz::Bar)
                    .with_band(0.0, 100.0),
            )
            .with_element(
                ElementSpec::new("fridge_temp", "celsius", "Fridge", Viz::Thermometer)
                    .with_band(2.0, 8.0),
            )
            .with_element(
                ElementSpec::new("ward_power", "percent", "Ward power", Viz::Bar)
                    .with_band(50.0, 100.0),
            )
            .with_element(
                ElementSpec::new("oxygen_stock", "percent", "Oxygen stock", Viz::Bar)
                    .with_band(30.0, 100.0),
            )
            .with_element(
                ElementSpec::new("uplink", "state", "Uplink", Viz::Switch)
                    .with_state("state.synced")
                    .as_stat(),
            )
            .with_element(
                ElementSpec::new("tamper_log", "record", "Tamper log", Viz::Chain)
                    .with_value(1000.0)
                    .as_stat(),
            )
            .with_element(
                ElementSpec::new("storage_tank", "percent", "Storage tank", Viz::Bar)
                    .with_band(20.0, 100.0),
            )
            .with_element(
                ElementSpec::new("flow_trend", "liter_per_minute", "Flow trend", Viz::Spark)
                    .with_band(0.0, 16.0),
            )
            .with_element(
                ElementSpec::new("pump_health", "state", "Pump health", Viz::Switch)
                    .with_state("state.nominal"),
            )
            .with_element(
                ElementSpec::new("acoustic", "decibel", "Acoustic", Viz::Wave)
                    .with_band(0.0, 120.0),
            )
            .with_element(
                ElementSpec::new("relay_status", "state", "Relay status", Viz::Switch)
                    .with_state("state.online")
                    .as_stat()
                    .on(mesh()),
            )
            .with_element(
                ElementSpec::new("routing", "state", "Routing", Viz::Switch)
                    .with_state("mesh.optimized")
                    .as_stat()
                    .on(mesh()),
            )
            .with_element(
                ElementSpec::new("mesh_relay", "state", "Mesh relay", Viz::Mesh)
                    .with_state("mesh.optimized")
                    .with_value(5.0)
                    .on(mesh()),
            );
        Catalog::from_presentations(&[&presentation])
            .with_site_position("__gateway", 0.47, 0.45)
            .with_site_position("cold-chain", 0.29, 0.15)
            .with_site_position("maternity", 0.17, 0.27)
            .with_site_position("silo-3", 0.72, 0.20)
            .with_site_position("weather", 0.88, 0.36)
            .with_site_position("solar", 0.66, 0.78)
            .with_site_position("river", 0.36, 0.69)
    }
}

// Finds a sensor by its `"groupId/sensorId"` path in a fleet snapshot.
fn find_sensor<'a>(state: &'a State, target: &str) -> Option<&'a Sensor> {
    let (group_id, sensor_id) = target.split_once('/')?;
    state
        .orgs
        .iter()
        .flat_map(|org| &org.groups)
        .filter(|group| group.id == group_id)
        .flat_map(|group| &group.sensors)
        .find(|sensor| sensor.id == sensor_id)
}

// Marks a sensor as a node or network stat rather than a real measurement.
fn stat(mut sensor: Sensor) -> Sensor {
    sensor.reading.stat = true;
    sensor
}

// Maps a reading key to a stable, localizable event code for an out-of-band reading.
fn event_for(key: &str) -> String {
    match key {
        "temperature" | "ambient_temp" => "temperature.out_of_range",
        k if k.starts_with("grain_temp") => "spoilage.risk",
        "battery_voltage" | "state_of_charge" => "battery.low",
        _ => "reading.out_of_range",
    }
    .to_owned()
}

impl StateSource for Mock {
    fn snapshot(&mut self) -> State {
        self.tick += 1;
        self.slot = 0;
        let uptime = self.tick * 5;
        let s = self.scenario;

        // Health authority: cold chain + ward power.
        let fridge1 = self.sensor(
            "fridge-1",
            "temperature",
            "celsius",
            5.0,
            0.5,
            (2.0, 8.0),
            None,
        );
        let fridge2_base = if s == Scenario::Alarm { 11.0 } else { 4.6 };
        let fridge2 = self.sensor(
            "fridge-2",
            "temperature",
            "celsius",
            fridge2_base,
            0.5,
            (2.0, 8.0),
            None,
        );
        let ward_humidity = self.sensor(
            "ward-rh",
            "humidity",
            "percent",
            48.0,
            4.0,
            (30.0, 60.0),
            None,
        );
        let cold_chain = self.group(
            "cold-chain",
            "Kano cold chain",
            LinkKind::Cellular,
            3,
            vec![fridge1, fridge2, ward_humidity],
        );

        let ward_power = self.sensor(
            "ward-soc",
            "state_of_charge",
            "percent",
            78.0,
            6.0,
            (40.0, 100.0),
            Some(0.78),
        );
        let ward_load = self.sensor(
            "ward-load",
            "load_power",
            "watt",
            210.0,
            40.0,
            (0.0, 600.0),
            None,
        );
        let maternity = self.group(
            "maternity",
            "Maternity ward power",
            LinkKind::Wifi,
            4,
            vec![ward_power, ward_load],
        );

        let health = Org {
            id: "kano-health".to_owned(),
            name: "Kano Health Authority".to_owned(),
            groups: vec![cold_chain, maternity],
        };

        // Farmers co-op: silo, weather, solar, river.
        let silo_top = self.sensor(
            "silo-top",
            "grain_temp_top",
            "celsius",
            22.0,
            0.6,
            (0.0, 30.0),
            None,
        );
        let silo_mid = self.sensor(
            "silo-mid",
            "grain_temp_lower",
            "celsius",
            24.0,
            0.6,
            (0.0, 30.0),
            None,
        );
        let silo_floor_base = if s == Scenario::SensorFault {
            -127.0
        } else {
            26.0
        };
        let silo_floor = self.sensor(
            "silo-floor",
            "grain_temp_floor",
            "celsius",
            silo_floor_base,
            0.6,
            (0.0, 30.0),
            None,
        );
        let silo_rh = self.sensor(
            "silo-rh",
            "humidity",
            "percent",
            58.0,
            5.0,
            (0.0, 65.0),
            None,
        );
        let silo = self.group(
            "silo-3",
            "Co-op silo 3",
            LinkKind::Lora,
            2,
            vec![silo_top, silo_mid, silo_floor, silo_rh],
        );

        let w_temp = self.sensor(
            "wx-temp",
            "temperature",
            "celsius",
            18.0,
            3.0,
            (3.0, 38.0),
            None,
        );
        let w_press = self.sensor(
            "wx-press",
            "pressure",
            "hectopascal",
            1009.0,
            4.0,
            (980.0, 1040.0),
            None,
        );
        let w_wind = self.sensor(
            "wx-wind",
            "wind_speed",
            "meter_per_second",
            5.5,
            2.5,
            (0.0, 20.0),
            None,
        );
        let w_lux = self.sensor(
            "wx-lux",
            "illuminance",
            "lux",
            42000.0,
            12000.0,
            (0.0, 100000.0),
            None,
        );
        let weather = self.group(
            "weather",
            "Village weather station",
            LinkKind::Lora,
            3,
            vec![w_temp, w_press, w_wind, w_lux],
        );

        let solar_low = s == Scenario::LowBattery;
        let pv = self.sensor(
            "pv",
            "pv_power",
            "watt",
            if solar_low { 40.0 } else { 320.0 },
            50.0,
            (0.0, 400.0),
            None,
        );
        let soc = self.sensor(
            "soc",
            "state_of_charge",
            "percent",
            if solar_low { 12.0 } else { 66.0 },
            4.0,
            (20.0, 100.0),
            Some(if solar_low { 0.12 } else { 0.66 }),
        );
        let vbatt = self.sensor(
            "vbatt",
            "battery_voltage",
            "volt",
            if solar_low { 11.6 } else { 12.9 },
            0.15,
            (11.8, 14.6),
            Some(if solar_low { 0.12 } else { 0.66 }),
        );
        let solar = self.group(
            "solar",
            "Off-grid solar microgrid",
            LinkKind::Ethernet,
            4,
            vec![pv, soc, vbatt],
        );

        let river_level = self.sensor(
            "river-1",
            "river_level",
            "millimeter",
            1800.0,
            120.0,
            (0.0, 3000.0),
            Some(0.71),
        );
        let mut river = self.group(
            "river",
            "River watch mesh",
            LinkKind::Mesh,
            3,
            vec![river_level],
        );
        if s == Scenario::LinkLost {
            river.link.online = false;
            river.link.strength = 0;
            river.sensors[0].events.insert(
                0,
                EventRecord {
                    level: EventLevel::Error,
                    code: "link.lost".to_owned(),
                    value: None,
                    age_secs: Some(40),
                },
            );
            river.recompute_status();
        }

        let coop = Org {
            id: "meru-coop".to_owned(),
            name: "Meru Farmers Co-op".to_owned(),
            groups: vec![silo, weather, solar, river],
        };

        // Pamoja Field Kits: bespoke kit/recipe groups for crucial-area deployments. The
        // farm node waters on soil moisture, so the drip valve opens when the soil is dry.
        let soil = self.sensor(
            "soil",
            "soil_moisture",
            "percent",
            48.0,
            9.0,
            (36.0, 100.0),
            None,
        );
        let soil_dry = soil.reading.value < 40.0;
        let well = self.sensor(
            "well",
            "well_level",
            "percent",
            64.0,
            6.0,
            (20.0, 100.0),
            None,
        );
        let mut valve = self.chip_sensor(
            "valve",
            "drip_valve",
            if soil_dry {
                "state.open"
            } else {
                "state.closed"
            },
            Status::Ok,
        );
        valve.reading = valve.reading.with_actions(["open", "closed"]);
        let farm_batt = self.sensor(
            "farm-batt",
            "battery_voltage",
            "volt",
            4.0,
            0.18,
            (3.5, 4.3),
            Some(0.74),
        );
        let soil_trend = self.sensor(
            "soil-trend",
            "soil_trend",
            "percent",
            48.0,
            11.0,
            (0.0, 100.0),
            None,
        );
        // The pump is a numeric actuator: its duty is whatever it was last set to.
        let pump_speed = Sensor {
            id: "pump".to_owned(),
            reading: Reading::new("pump_speed", 55.0, "percent").with_range(0.0, 100.0),
            battery: None,
            mode: Mode::Active,
            history: vec![55.0; 12],
            events: Vec::new(),
            peer: None,
            lat: None,
            lon: None,
        };
        let farm = self.group(
            "farm-node",
            "Farm node",
            LinkKind::Lora,
            3,
            vec![soil, well, valve, pump_speed, farm_batt, soil_trend],
        );

        // Health post: a cold-chain clinic kit on NB-IoT whose readings are written to a
        // tamper-evident, hash-chained log so a record cannot be altered after the fact.
        let fridge_base = if s == Scenario::Alarm { 11.5 } else { 4.2 };
        let fridge = self.sensor(
            "fridge",
            "fridge_temp",
            "celsius",
            fridge_base,
            1.2,
            (2.0, 8.0),
            None,
        );
        let ward_pwr = self.sensor(
            "ward-pwr",
            "ward_power",
            "percent",
            90.0,
            5.0,
            (50.0, 100.0),
            None,
        );
        let oxygen = self.sensor(
            "o2",
            "oxygen_stock",
            "percent",
            72.0,
            7.0,
            (30.0, 100.0),
            None,
        );
        let uplink = stat(self.chip_sensor("uplink", "uplink", "state.synced", Status::Ok));
        let tamper = Sensor {
            id: "tamper".to_owned(),
            reading: Reading::new("tamper_log", 1041.0 + self.tick as f32, "record")
                .with_status(Status::Ok)
                .as_stat(),
            battery: None,
            mode: Mode::Active,
            history: Vec::new(),
            events: vec![
                EventRecord {
                    level: EventLevel::Info,
                    code: "log.signed".to_owned(),
                    value: None,
                    age_secs: Some(6),
                },
                EventRecord {
                    level: EventLevel::Info,
                    code: "log.chained".to_owned(),
                    value: None,
                    age_secs: Some(22),
                },
            ],
            peer: None,
            lat: None,
            lon: None,
        };
        let health_post = self.group(
            "health-post",
            "Health post",
            LinkKind::NbIot,
            3,
            vec![fridge, ward_pwr, oxygen, uplink, tamper],
        );

        // Water point: a borehole/standpipe kit on LoRa - flow rate and tank level with a
        // pump-health read derived from pressure trend, plus a day-long flow trace.
        let flow = self.sensor(
            "flow",
            "flow_rate",
            "liter_per_minute",
            9.0,
            4.0,
            (0.0, 16.0),
            None,
        );
        let wp_well = self.sensor(
            "wp-well",
            "well_level",
            "percent",
            58.0,
            7.0,
            (20.0, 100.0),
            None,
        );
        let tank = self.sensor(
            "tank",
            "storage_tank",
            "percent",
            64.0,
            8.0,
            (20.0, 100.0),
            None,
        );
        let pump = self.chip_sensor("pump", "pump_health", "state.nominal", Status::Ok);
        let flow_trend = self.sensor(
            "flow-trend",
            "flow_trend",
            "liter_per_minute",
            9.0,
            5.0,
            (0.0, 16.0),
            None,
        );
        let mut water_point = self.group(
            "water-point",
            "Water point",
            LinkKind::Lora,
            3,
            vec![flow, wp_well, tank, pump, flow_trend],
        );
        if s == Scenario::LinkLost {
            water_point.link.online = false;
            water_point.link.strength = 0;
            water_point.sensors[0].events.insert(
                0,
                EventRecord {
                    level: EventLevel::Error,
                    code: "link.lost".to_owned(),
                    value: None,
                    age_secs: Some(40),
                },
            );
            water_point.recompute_status();
        }

        // Ranger relay: a conservation mesh node that listens for threats (chainsaws,
        // gunshots) on an acoustic monitor and relays alerts to the ranger post.
        // The relay's neighbours are real sensor stations with their own positions; two
        // sensors share the riverside post to show a peer that hosts more than one sensor.
        let rr_river = self
            .sensor(
                "rr-river",
                "river_level",
                "millimeter",
                1700.0,
                120.0,
                (0.0, 3000.0),
                None,
            )
            .on_peer("River post")
            .at(-0.4503, 36.8702);
        let rr_low = s == Scenario::LowBattery;
        let rr_batt = self
            .sensor(
                "rr-batt",
                "battery_level",
                "percent",
                if rr_low { 14.0 } else { 90.0 },
                5.0,
                (20.0, 100.0),
                Some(if rr_low { 0.14 } else { 0.9 }),
            )
            .on_peer("River post")
            .at(-0.4503, 36.8702);
        let relay = stat(self.chip_sensor("relay", "relay_status", "state.online", Status::Ok));
        let rr_temp_base = if s == Scenario::SensorFault {
            -127.0
        } else {
            27.0
        };
        let rr_temp = self
            .sensor(
                "rr-temp",
                "ambient_temp",
                "celsius",
                rr_temp_base,
                4.0,
                (0.0, 45.0),
                None,
            )
            .on_peer("Ridge camera")
            .at(-0.3902, 36.9461);
        // Most of the time it is quiet ambient; briefly and periodically a chainsaw is
        // heard, spiking the monitor so the threat state is visible without any input. The
        // sound class drives the hot waveform; the node stays OK (it heard and relayed).
        let chainsaw = (self.tick % 40) < 3;
        let mut acoustic = self
            .sensor(
                "acoustic",
                "acoustic",
                "decibel",
                if chainsaw { 84.0 } else { 41.0 },
                6.0,
                (0.0, 120.0),
                None,
            )
            .on_peer("Acoustic post")
            .at(-0.4288, 36.9303);
        acoustic.reading.state = Some(
            if chainsaw {
                "acoustic.abnormal"
            } else {
                "acoustic.ambient"
            }
            .to_owned(),
        );
        let relay_mesh = self.mesh_sensor("relay-mesh", 5.0);
        let ranger_relay = self
            .group(
                "ranger-relay",
                "Ranger relay",
                LinkKind::Mesh,
                3,
                vec![rr_river, rr_batt, relay, rr_temp, acoustic, relay_mesh],
            )
            .at(-0.4216, 36.9123);

        // Mesh node: a routing peer in the mesh - it draws its neighbour graph with live
        // packets and reports neighbours, hops to the gateway, routing state and traffic.
        let neighbour_mesh = self.mesh_sensor("mesh", 6.0);
        let neighbours =
            stat(self.sensor("neigh", "neighbours", "count", 5.0, 0.0, (1.0, 12.0), None));
        let hops = stat(self.sensor("hops", "hops", "count", 3.0, 0.0, (1.0, 8.0), None));
        let routing = stat(self.chip_sensor("routing", "routing", "mesh.optimized", Status::Ok));
        let relayed = stat(self.sensor(
            "relayed",
            "messages_relayed",
            "count",
            318.0 + self.tick as f32,
            0.0,
            (0.0, 99999.0),
            None,
        ));
        let mesh_node = self.group(
            "mesh-node",
            "Neighborhood Mesh",
            LinkKind::Mesh,
            4,
            vec![neighbour_mesh, neighbours, hops, routing, relayed],
        );

        let field_kits = Org {
            id: "pamoja-kits".to_owned(),
            name: "Pamoja Field Kits".to_owned(),
            groups: vec![farm, health_post, water_point, ranger_relay, mesh_node],
        };

        let mut state = State {
            orgs: vec![field_kits, health, coop],
            status: Status::Ok,
            uptime_secs: Some(uptime),
            demo: true,
        };
        self.apply_edits(&mut state);
        self.apply_overrides(&mut state);
        state.recompute_status();
        state
    }

    fn select(&mut self, key: &str) -> bool {
        match Scenario::from_key(key) {
            Some(scenario) => {
                self.set_scenario(scenario);
                true
            }
            None => false,
        }
    }

    fn command(&mut self, command: &Command) -> Result<(), CommandError> {
        // Validate against the current fleet. Probing advances the mock's clock one tick,
        // which is immaterial for a hardware-free source.
        let fleet = self.snapshot();
        match command {
            Command::Actuate { target, action } => {
                match find_sensor(&fleet, target).map(|sensor| &sensor.reading) {
                    None => Err(CommandError::UnknownTarget),
                    Some(reading) => match &reading.actions {
                        Some(actions) if actions.iter().any(|a| a == action) => {
                            self.overrides.insert(target.clone(), action.clone());
                            Ok(())
                        }
                        Some(_) => Err(CommandError::InvalidAction),
                        None => Err(CommandError::Unsupported),
                    },
                }
            }
            Command::Set { target, value } => {
                match find_sensor(&fleet, target).map(|sensor| sensor.reading.range) {
                    None => Err(CommandError::UnknownTarget),
                    Some(Some([low, high])) if (low..=high).contains(value) => {
                        self.settings.insert(target.clone(), *value);
                        Ok(())
                    }
                    Some(Some(_)) => Err(CommandError::OutOfRange),
                    Some(None) => Err(CommandError::Unsupported),
                }
            }
            Command::AddGroup { org, group } => {
                if fleet.orgs.iter().any(|o| o.id == *org) {
                    self.added_groups.push((org.clone(), group.clone()));
                    Ok(())
                } else {
                    Err(CommandError::UnknownTarget)
                }
            }
            Command::RemoveGroup { id } => {
                self.removed_groups.push(id.clone());
                Ok(())
            }
            Command::AddSensor { group, sensor, .. } => {
                let known = fleet
                    .orgs
                    .iter()
                    .flat_map(|o| &o.groups)
                    .any(|g| g.id == *group);
                if known {
                    self.added_sensors.push((group.clone(), sensor.clone()));
                    Ok(())
                } else {
                    Err(CommandError::UnknownTarget)
                }
            }
            Command::RemoveSensor { target } => {
                self.removed_sensors.push(target.clone());
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scenario_round_trips_its_key() {
        for scenario in Scenario::ALL {
            assert_eq!(Scenario::from_key(scenario.key()), Some(scenario));
        }
        assert_eq!(Scenario::from_key("nope"), None);
    }

    #[test]
    fn the_fleet_has_several_orgs_and_groups() {
        let mut fleet = Mock::new(Scenario::Normal);
        let state = fleet.snapshot();
        assert_eq!(state.orgs.len(), 3);
        let groups: usize = state.orgs.iter().map(|o| o.groups.len()).sum();
        assert!(groups >= 7, "expected a rich fleet, got {groups} groups");
        assert_eq!(state.status, Status::Ok);
        assert!(state.demo, "the mock marks its snapshot as demo data");
    }

    #[test]
    fn a_set_value_holds_across_ticks_and_out_of_range_is_refused() {
        let mut fleet = Mock::new(Scenario::Normal);
        fleet
            .command(&Command::Set {
                target: "farm-node/pump".to_owned(),
                value: 80.0,
            })
            .expect("inside the pump's range");
        for _ in 0..3 {
            let state = fleet.snapshot();
            let pump = state
                .orgs
                .iter()
                .flat_map(|o| &o.groups)
                .flat_map(|g| &g.sensors)
                .find(|s| s.id == "pump")
                .expect("the pump");
            assert_eq!(pump.reading.value, 80.0);
            assert_eq!(pump.reading.range, Some([0.0, 100.0]));
        }
        assert_eq!(
            fleet.command(&Command::Set {
                target: "farm-node/pump".to_owned(),
                value: 140.0,
            }),
            Err(CommandError::OutOfRange)
        );
        assert_eq!(
            fleet.command(&Command::Set {
                target: "farm-node/soil".to_owned(),
                value: 40.0,
            }),
            Err(CommandError::Unsupported)
        );
    }

    #[test]
    fn the_demo_catalog_declares_the_field_kit_elements_and_the_map() {
        let json = Mock::catalog().to_json().expect("serialize");
        for key in [
            "well_level",
            "fridge_temp",
            "ward_power",
            "oxygen_stock",
            "pump_speed",
            "mesh_relay",
            "tamper_log",
        ] {
            assert!(
                json.contains(&format!("\"key\":\"{key}\"")),
                "{key} is declared"
            );
        }
        assert!(json.contains("\"sitePositions\""));
        assert!(json.contains("\"river\":[0.36,0.69]"));
        // Every key the demo fleet reports has a preset or is one the page ships.
        let mut fleet = Mock::new(Scenario::Normal);
        let state = fleet.snapshot();
        let shipped = [
            "temperature",
            "humidity",
            "state_of_charge",
            "pv_power",
            "pressure",
            "wind_speed",
            "illuminance",
            "battery_voltage",
            "soil_moisture",
            "battery_level",
            "flow_rate",
            "neighbours",
            "hops",
            "messages_relayed",
            "ambient_temp",
            "load_power",
            "rainfall",
            "river_level",
            "compressor_duty",
            "grain_temp_top",
            "grain_temp_upper",
            "grain_temp_lower",
            "grain_temp_floor",
            "neighbour_mesh",
            "relay_mesh",
        ];
        for sensor in state
            .orgs
            .iter()
            .flat_map(|o| &o.groups)
            .flat_map(|g| &g.sensors)
        {
            let key = sensor.reading.key.as_str();
            assert!(
                shipped.contains(&key) || json.contains(&format!("\"key\":\"{key}\"")),
                "{key} needs a preset in the demo catalog or the page"
            );
        }
    }

    #[test]
    fn the_farm_node_has_a_discrete_valve_reading() {
        let mut fleet = Mock::new(Scenario::Normal);
        let state = fleet.snapshot();
        let valve = state
            .orgs
            .iter()
            .flat_map(|o| &o.groups)
            .flat_map(|g| &g.sensors)
            .find(|s| s.reading.key == "drip_valve")
            .expect("drip valve sensor");
        assert!(valve.reading.state.is_some(), "valve carries a state code");
        assert!(
            valve.reading.actions.is_some(),
            "valve advertises its actions, so it is controllable"
        );
    }

    #[test]
    fn node_stats_are_flagged_and_measurements_are_not() {
        let mut fleet = Mock::new(Scenario::Normal);
        let state = fleet.snapshot();
        let by_key = |key: &str| {
            state
                .orgs
                .iter()
                .flat_map(|o| &o.groups)
                .flat_map(|g| &g.sensors)
                .find(|s| s.reading.key == key)
        };
        for key in [
            "neighbours",
            "hops",
            "routing",
            "messages_relayed",
            "relay_status",
            "uplink",
            "tamper_log",
        ] {
            assert!(
                by_key(key).is_some_and(|s| s.reading.stat),
                "{key} is a node stat"
            );
        }
        for key in ["soil_moisture", "battery_level", "acoustic"] {
            assert!(
                by_key(key).is_some_and(|s| !s.reading.stat),
                "{key} is a measurement, not a stat"
            );
        }
        // A pure relay node carries only its mesh map and node stats - no plain sensors.
        let mesh = state
            .orgs
            .iter()
            .flat_map(|o| &o.groups)
            .find(|g| g.id == "mesh-node")
            .expect("mesh node group");
        let measurements = mesh
            .sensors
            .iter()
            .filter(|s| !s.reading.stat && s.reading.key != "mesh_relay")
            .count();
        assert_eq!(measurements, 0, "the mesh node has no plain sensors");
    }

    #[test]
    fn actuating_the_valve_holds_its_new_state() {
        let mut fleet = Mock::new(Scenario::Normal);
        fleet
            .command(&Command::Actuate {
                target: "farm-node/valve".to_owned(),
                action: "closed".to_owned(),
            })
            .expect("valve accepts a valid action");
        let closed = fleet.snapshot();
        let valve = find_sensor(&closed, "farm-node/valve").expect("valve");
        assert_eq!(valve.reading.state.as_deref(), Some("state.closed"));

        fleet
            .command(&Command::Actuate {
                target: "farm-node/valve".to_owned(),
                action: "open".to_owned(),
            })
            .expect("valve accepts the other action");
        let opened = fleet.snapshot();
        let valve = find_sensor(&opened, "farm-node/valve").expect("valve");
        assert_eq!(valve.reading.state.as_deref(), Some("state.open"));
    }

    #[test]
    fn provisioning_adds_and_removes_in_the_snapshot() {
        let mut fleet = Mock::new(Scenario::Normal);
        let org_id = fleet.snapshot().orgs[0].id.clone();
        let group = Group {
            id: "extra".to_owned(),
            name: "Extra".to_owned(),
            link: Link {
                kind: LinkKind::Lora,
                strength: 3,
                online: true,
            },
            status: Status::Ok,
            sensors: Vec::new(),
            lat: None,
            lon: None,
        };
        fleet
            .command(&Command::AddGroup {
                org: org_id.clone(),
                group,
            })
            .expect("add group to a known org");
        let added = fleet.snapshot();
        let org = added.orgs.iter().find(|o| o.id == org_id).expect("org");
        assert!(org.groups.iter().any(|g| g.id == "extra"), "group added");

        fleet
            .command(&Command::RemoveGroup {
                id: "extra".to_owned(),
            })
            .expect("remove the group");
        let removed = fleet.snapshot();
        let org = removed.orgs.iter().find(|o| o.id == org_id).expect("org");
        assert!(!org.groups.iter().any(|g| g.id == "extra"), "group removed");
    }

    #[test]
    fn adding_a_group_to_an_unknown_org_is_refused() {
        let mut fleet = Mock::new(Scenario::Normal);
        let group = Group {
            id: "x".to_owned(),
            name: "X".to_owned(),
            link: Link {
                kind: LinkKind::Lora,
                strength: 3,
                online: true,
            },
            status: Status::Ok,
            sensors: Vec::new(),
            lat: None,
            lon: None,
        };
        assert_eq!(
            fleet.command(&Command::AddGroup {
                org: "no-such-org".to_owned(),
                group,
            }),
            Err(CommandError::UnknownTarget)
        );
    }

    #[test]
    fn a_command_to_an_unknown_or_invalid_target_is_refused() {
        let mut fleet = Mock::new(Scenario::Normal);
        assert_eq!(
            fleet.command(&Command::Actuate {
                target: "farm-node/nope".to_owned(),
                action: "open".to_owned(),
            }),
            Err(CommandError::UnknownTarget)
        );
        assert_eq!(
            fleet.command(&Command::Actuate {
                target: "farm-node/valve".to_owned(),
                action: "spin".to_owned(),
            }),
            Err(CommandError::InvalidAction)
        );
    }

    #[test]
    fn the_alarm_scenario_pushes_a_fridge_out_of_band() {
        let mut fleet = Mock::new(Scenario::Alarm);
        let state = fleet.snapshot();
        assert_eq!(state.status, Status::Alarm);
        let cold = state
            .orgs
            .iter()
            .flat_map(|o| &o.groups)
            .find(|g| g.id == "cold-chain")
            .expect("cold chain group");
        assert!(cold
            .sensors
            .iter()
            .any(|s| s.reading.status == Status::Alarm));
    }

    #[test]
    fn the_link_lost_scenario_takes_a_group_offline() {
        let mut fleet = Mock::new(Scenario::LinkLost);
        let state = fleet.snapshot();
        let river = state
            .orgs
            .iter()
            .flat_map(|o| &o.groups)
            .find(|g| g.id == "river")
            .expect("river group");
        assert!(!river.link.online);
    }

    #[test]
    fn the_drift_is_deterministic_for_a_tick() {
        let a = Mock::new(Scenario::Normal).snapshot();
        let b = Mock::new(Scenario::Normal).snapshot();
        assert_eq!(a, b);
    }
}
