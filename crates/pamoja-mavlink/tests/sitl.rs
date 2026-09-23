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
//! An autopilot answers MAVLink long before it can fly: ArduPilot SITL does not even start its
//! boot until a ground station connects on TCP, and until it has, it reports room for no mission
//! items and refuses to arm. So the test first asks for `SYS_STATUS` and waits for the
//! `PREARM_CHECK` health bit, which both autopilots set once every pre-arm check passes. After
//! that nothing is tolerated: the plan must be stored and read back item for item, the arm must
//! be accepted and seen in the heartbeat, and the disarm must be accepted.

#![cfg(feature = "std")]

use std::time::Duration;

use pamoja_core::Device;
use pamoja_mavlink::dialect::{
    mav_autopilot, mav_cmd, mav_frame, mav_mode_flag, mav_result, mav_sys_status_sensor,
    AutopilotVersion, Message, MissionItemInt, SysStatus,
};
use pamoja_mavlink::link::ByteLink;
use pamoja_mavlink::vehicle::GCS_COMPONENT;
use pamoja_mavlink::{Report, TcpLink, UdpLink, Vehicle};

const BUDGET: Duration = Duration::from_secs(300);

const READY_WITHIN: Duration = Duration::from_secs(180);

const ARMED_WITHIN: Duration = Duration::from_secs(10);

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

fn sample_plan() -> [MissionItemInt; 3] {
    [
        waypoint(mav_cmd::NAV_TAKEOFF, -353_632_610, 1_491_652_300, 10.0),
        waypoint(mav_cmd::NAV_WAYPOINT, -353_631_000, 1_491_653_000, 20.0),
        waypoint(mav_cmd::NAV_WAYPOINT, -353_630_000, 1_491_654_000, 15.0),
    ]
}

#[tokio::test]
#[ignore = "needs a running ArduPilot or PX4 SITL; run via `cargo xtask sitl <ardupilot|px4>`"]
async fn a_real_autopilot_stores_a_plan_and_arms() {
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
    vehicle
        .connect()
        .await
        .expect("no heartbeat from the autopilot");
    println!("connected to {}", vehicle.id());
    vehicle
        .send_heartbeat()
        .await
        .expect("sending a GCS heartbeat");

    let autopilot = read_heartbeat_autopilot(vehicle).await;
    let name = match autopilot {
        mav_autopilot::ARDUPILOTMEGA => "ArduPilot",
        mav_autopilot::PX4 => "PX4",
        _ => "an unrecognized autopilot",
    };
    println!("the heartbeat names {name} (autopilot id {autopilot})");

    let result = vehicle
        .request_message(AutopilotVersion::ID)
        .await
        .expect("requesting AUTOPILOT_VERSION");
    assert_eq!(result, mav_result::ACCEPTED, "AUTOPILOT_VERSION request");

    wait_until_ready(vehicle).await;

    let existing = vehicle
        .download_mission()
        .await
        .expect("downloading the mission");
    println!("downloaded {} existing mission items", existing.len());

    let plan = sample_plan();
    vehicle
        .upload_mission(&plan)
        .await
        .expect("the autopilot refused the plan");
    let stored = vehicle
        .download_mission()
        .await
        .expect("downloading after upload");
    println!(
        "uploaded {} items and read back {}",
        plan.len(),
        stored.len()
    );
    assert_eq!(stored.len(), plan.len(), "the stored plan's item count");
    // ArduPilot keeps sequence 0 as its home position whatever is uploaded there, so the
    // items after it are the ones both autopilots must hand back unchanged.
    for (sent, back) in plan.iter().zip(&stored).skip(1) {
        assert_eq!(back.command, sent.command, "item {} command", back.seq);
        assert_eq!(
            (back.x, back.y),
            (sent.x, sent.y),
            "item {} position",
            back.seq
        );
    }

    let armed = vehicle.arm(true).await.expect("sending the arm command");
    assert_eq!(armed, mav_result::ACCEPTED, "the arm command");
    wait_for_armed_heartbeat(vehicle).await;
    println!("armed");

    let disarmed = vehicle
        .arm(false)
        .await
        .expect("sending the disarm command");
    assert_eq!(disarmed, mav_result::ACCEPTED, "the disarm command");
    println!("disarmed");
}

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

async fn wait_until_ready<L: ByteLink + Send>(vehicle: &mut Vehicle<L>) {
    let result = vehicle
        .set_message_interval(SysStatus::ID, 500_000)
        .await
        .expect("asking for SYS_STATUS");
    assert_eq!(result, mav_result::ACCEPTED, "the SYS_STATUS interval");

    let started = tokio::time::Instant::now();
    let mut said = Vec::new();
    while started.elapsed() < READY_WITHIN {
        vehicle
            .send_heartbeat()
            .await
            .expect("sending a GCS heartbeat");
        let Ok(report) = tokio::time::timeout(Duration::from_secs(1), vehicle.recv()).await else {
            continue;
        };
        match report.expect("reading telemetry") {
            Report::SysStatus(status)
                if status.onboard_control_sensors_health & mav_sys_status_sensor::PREARM_CHECK
                    != 0 =>
            {
                println!(
                    "pre-arm checks passed after {:.0} s",
                    started.elapsed().as_secs_f32()
                );
                return;
            }
            Report::Statustext(status) => {
                let end = status
                    .text
                    .iter()
                    .position(|&byte| byte == 0)
                    .unwrap_or(status.text.len());
                let text = String::from_utf8_lossy(&status.text[..end]).into_owned();
                println!("autopilot: {text}");
                said.push(text);
            }
            _ => {}
        }
    }
    panic!(
        "the pre-arm checks did not pass within {} s; the autopilot said: {said:?}",
        READY_WITHIN.as_secs()
    );
}

async fn wait_for_armed_heartbeat<L: ByteLink>(vehicle: &mut Vehicle<L>) {
    let deadline = tokio::time::Instant::now() + ARMED_WITHIN;
    while tokio::time::Instant::now() < deadline {
        let Ok(report) = tokio::time::timeout(Duration::from_secs(2), vehicle.recv()).await else {
            continue;
        };
        if let Report::Heartbeat(heartbeat) = report.expect("reading telemetry") {
            if heartbeat.base_mode & mav_mode_flag::SAFETY_ARMED != 0 {
                return;
            }
        }
    }
    panic!("the arm was accepted but no heartbeat reported the vehicle armed");
}
