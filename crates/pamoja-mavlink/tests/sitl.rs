//! Interop tests against a real ArduPilot or PX4 SITL autopilot.
//!
//! Unlike the in-process [`SitlAutopilot`](pamoja_mavlink::link::SitlAutopilot) tests, which run
//! everywhere, this drives a [`Vehicle`] over a real link against an actual autopilot's MAVLink
//! stack, so interop is proven against ArduPilot and PX4 rather than against our own mock. It is
//! `#[ignore]`d by default because it needs a running SITL; `cargo xtask sitl <ardupilot|px4>`
//! builds the autopilot in Docker, launches it, and runs this with the endpoint set:
//!
//! - `PAMOJA_SITL_TCP` (for example `127.0.0.1:5760`) connects over TCP, as ArduPilot SITL serves.
//! - `PAMOJA_SITL_UDP` (for example `0.0.0.0:14550`) binds and learns the peer, as PX4 SITL sends.
//!
//! The assertions are at the protocol level: a heartbeat is received, a command is acknowledged,
//! and a mission downloads. Mission upload is asserted to round-trip on PX4 (whose mission storage
//! is in RAM); ArduPilot SITL advertises no mission storage in a headless build and answers
//! `MAV_MISSION_NO_SPACE`, a simulator provisioning limitation rather than a protocol failure, so
//! it is tolerated there (the upload path itself is covered by the in-process round-trip test).
//! No flight outcome is asserted, since a SITL without full sensor simulation may reject arming.

#![cfg(feature = "std")]

use std::time::Duration;

use pamoja_core::Device;
use pamoja_mavlink::dialect::{
    mav_autopilot, mav_cmd, mav_frame, AutopilotVersion, Message, MissionItemInt,
};
use pamoja_mavlink::link::ByteLink;
use pamoja_mavlink::vehicle::GCS_COMPONENT;
use pamoja_mavlink::{Report, TcpLink, UdpLink, Vehicle};

// The overall budget for the whole exchange, generous enough for a cold SITL still booting.
const BUDGET: Duration = Duration::from_secs(60);

fn waypoint(command: u16, lat: i32, lon: i32, alt: f32) -> MissionItemInt {
    MissionItemInt {
        param1: 0.0,
        param2: 0.0,
        param3: 0.0,
        param4: 0.0,
        x: lat,
        y: lon,
        z: alt,
        seq: 0,
        command,
        target_system: 0,
        target_component: 0,
        frame: mav_frame::GLOBAL_RELATIVE_ALT_INT,
        current: 0,
        autocontinue: 1,
        mission_type: 0,
    }
}

// A small, ordinary plan: take off, then two waypoints.
fn sample_plan() -> [MissionItemInt; 3] {
    [
        waypoint(mav_cmd::NAV_TAKEOFF, -353_632_610, 1_491_652_300, 10.0),
        waypoint(mav_cmd::NAV_WAYPOINT, -353_631_000, 1_491_653_000, 20.0),
        waypoint(mav_cmd::NAV_WAYPOINT, -353_630_000, 1_491_654_000, 15.0),
    ]
}

#[tokio::test]
#[ignore = "needs a running ArduPilot or PX4 SITL; run via `cargo xtask sitl <ardupilot|px4>`"]
async fn a_real_autopilot_completes_the_mission_and_command_exchanges() {
    if let Ok(addr) = std::env::var("PAMOJA_SITL_TCP") {
        let link = TcpLink::connect(&addr)
            .await
            .unwrap_or_else(|err| panic!("connecting to SITL over TCP at {addr}: {err}"));
        run(Vehicle::new(link, 255, GCS_COMPONENT)).await;
    } else if let Ok(addr) = std::env::var("PAMOJA_SITL_UDP") {
        let link = UdpLink::bind(&addr)
            .await
            .unwrap_or_else(|err| panic!("binding {addr} for SITL UDP: {err}"));
        run(Vehicle::new(link, 255, GCS_COMPONENT)).await;
    } else {
        panic!("set PAMOJA_SITL_TCP or PAMOJA_SITL_UDP to a SITL endpoint");
    }
}

async fn run<L: ByteLink + Send>(mut vehicle: Vehicle<L>) {
    tokio::time::timeout(BUDGET, drive(&mut vehicle))
        .await
        .expect("the SITL interop exchange overran its time budget");
}

async fn drive<L: ByteLink + Send>(vehicle: &mut Vehicle<L>) {
    // The vehicle announces itself; connecting learns its MAVLink address.
    vehicle
        .connect()
        .await
        .expect("no heartbeat from the autopilot");
    println!("connected to {}", vehicle.id());

    // Some autopilots only act on a ground station they can see, so make ourselves known.
    vehicle
        .send_heartbeat()
        .await
        .expect("sending a GCS heartbeat");

    // Read telemetry until a heartbeat, and report which autopilot answered.
    let autopilot = read_heartbeat_autopilot(vehicle).await;
    println!("heartbeat autopilot id: {autopilot}");

    // Ask for the autopilot's version; any acknowledgment is a completed command exchange.
    let result = vehicle
        .request_message(AutopilotVersion::ID)
        .await
        .expect("requesting AUTOPILOT_VERSION");
    println!("AUTOPILOT_VERSION request result: {result}");

    // Download the current plan, exercising the receiver state machine against the real autopilot.
    let existing = vehicle
        .download_mission()
        .await
        .expect("downloading the mission");
    println!("downloaded {} existing mission items", existing.len());

    // Upload a plan and read it back. PX4 stores plans in RAM and must accept it; ArduPilot SITL
    // advertises no mission storage in this headless build and answers NO_SPACE, which is
    // tolerated (see the module docs).
    let plan = sample_plan();
    match vehicle.upload_mission(&plan).await {
        Ok(()) => {
            let downloaded = vehicle
                .download_mission()
                .await
                .expect("downloading after upload");
            println!(
                "uploaded {} items, downloaded {}",
                plan.len(),
                downloaded.len()
            );
            assert_eq!(
                downloaded.len(),
                plan.len(),
                "the autopilot stored a different item count than was uploaded"
            );
        }
        Err(err)
            if autopilot == mav_autopilot::ARDUPILOTMEGA
                && err.to_string().contains("result 4") =>
        {
            println!(
                "ArduPilot SITL declined the upload ({err}); its mission storage is \
                 unprovisioned in this headless build, so upload is tolerated here"
            );
        }
        Err(err) => panic!("uploading the mission: {err}"),
    }

    // Arm; whether the autopilot accepts or denies it, a well-formed COMMAND_ACK is a completed
    // command exchange, which is what interop is proving here.
    let arm_result = vehicle.arm(true).await.expect("arming command");
    println!("arm command result: {arm_result}");
}

// Reads reports until a heartbeat and returns its autopilot id. Bounded by time rather than a
// message count, since an autopilot can stream other telemetry far faster than its 1 Hz
// heartbeat.
async fn read_heartbeat_autopilot<L: ByteLink>(vehicle: &mut Vehicle<L>) -> u8 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), vehicle.recv()).await {
            Ok(Ok(Report::Heartbeat(heartbeat))) => return heartbeat.autopilot,
            Ok(Ok(_)) => continue,
            Ok(Err(err)) => panic!("reading telemetry: {err}"),
            Err(_) => panic!("no telemetry arrived within the read window"),
        }
    }
    panic!("no heartbeat seen among the telemetry");
}
