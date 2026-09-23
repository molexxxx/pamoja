//! The CAN and J1939 guide example; see docs/guides/can.md.
//!
//! Run: `cargo run -p pamoja-examples --example can`

use std::error::Error;

/// A standby generator's J1939 bus: the engine controller broadcasting its speed, a monitoring
/// gateway keeping only that, a service laptop hearing everything, and a coolant sensor speaking
/// plain CAN beside them, all on one simulated bus.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_can::bus::{CanBus, Filter};
    use pamoja_can::{priority, CanError, CanId, Frame, J1939Id, Signals, NOT_AVAILABLE};

    // The nodes by the address each answers to, and the two parameter groups in play.
    const ENGINE: u8 = 0;
    const GATEWAY: u8 = 1;
    const ENGINE_CONTROLLER_1: u32 = 61_444; // carries engine speed
    const REQUEST: u32 = 59_904; // asks another node for a parameter group

    // Where engine speed sits inside that group, and the scale the standard fixes for it.
    const ENGINE_SPEED_AT: usize = 3;
    const RPM_PER_BIT: f64 = 0.125;

    // J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group,
    // and the sender's address. A broadcast names no destination.
    let speed_id = J1939Id::broadcast(priority::CONTROL, ENGINE_CONTROLLER_1, ENGINE);
    println!(
        "engine speed 0x{:08X}: pgn {} at priority {}, from node {ENGINE} to every node",
        speed_id.to_id().raw(),
        speed_id.pgn(),
        speed_id.priority()
    );

    // A reading starts with every signal marked not available, and the engine writes only its
    // speed.
    let reading = |rpm: f64| -> Result<Frame, CanError> {
        let mut signals = Signals::new();
        signals.set_u16(ENGINE_SPEED_AT, (rpm / RPM_PER_BIT) as u16);
        Frame::new(speed_id.to_id(), signals.as_bytes())
    };
    let rpm_of = |frame: &Frame| {
        let raw = frame
            .signals()
            .and_then(|signals| signals.u16(ENGINE_SPEED_AT));
        raw.map(|raw| f64::from(raw) * RPM_PER_BIT)
    };
    let first = reading(1500.0)?;
    let unreported = first
        .data()
        .iter()
        .filter(|&&byte| byte == NOT_AVAILABLE)
        .count();
    println!(
        "payload      {:.1} rpm in bytes {} and {}, the other {unreported} not available",
        rpm_of(&first).unwrap_or_default(),
        ENGINE_SPEED_AT + 1,
        ENGINE_SPEED_AT + 2
    );

    // Four nodes on one bus with nothing plugged in. On a Linux board each is
    // CanBus::open("can0"), and nothing after this statement changes.
    let engine = CanBus::simulated();
    let gateway = engine.join()?;
    let laptop = engine.join()?;
    let sensor = engine.join()?;

    // The gateway keeps engine speed and nothing else; the laptop keeps everything.
    gateway.set_filters(&[Filter::pgn(ENGINE_CONTROLLER_1)])?;

    // Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
    // level in percent on the 11-bit identifier 0x120.
    engine.send(&first)?;
    sensor.send(&Frame::new(CanId::standard(0x120), &[87])?)?;
    engine.send(&reading(1512.5)?)?;

    // Every node hears every frame but its own, and keeps what its filters pass.
    while let Some(frame) = gateway.receive(Duration::from_millis(10))? {
        let from = J1939Id::from_id(frame.id()).map(|id| id.source());
        println!(
            "gateway      {:.1} rpm from node {}",
            rpm_of(&frame).unwrap_or_default(),
            from.unwrap_or_default()
        );
    }
    let on_the_bus = engine.sent() + sensor.sent();
    println!(
        "gateway      kept {} of the {on_the_bus} frames on the bus",
        gateway.received()
    );
    let mut heard = Vec::new();
    while let Some(frame) = laptop.receive(Duration::from_millis(10))? {
        heard.push(frame);
    }
    if let Some(plain) = heard
        .iter()
        .find(|frame| J1939Id::from_id(frame.id()).is_none())
    {
        println!(
            "laptop       heard {}, among them 0x{:03X}, an 11-bit identifier and no J1939 message",
            heard.len(),
            plain.id().raw()
        );
    }

    // A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
    // identifier name the node it is for.
    let request_id = J1939Id::from_parts(priority::DEFAULT, REQUEST, GATEWAY, ENGINE);
    println!(
        "request      pgn {} from node {} to node {}",
        request_id.pgn(),
        request_id.source(),
        request_id.destination().unwrap_or_default()
    );

    // The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus
    // it returns at once and counts the wait instead of sleeping through it.
    let before = gateway.waited_micros();
    let quiet = gateway.receive(Duration::from_millis(500))?;
    println!(
        "silent       {} frames in {} ms, counted and not slept",
        usize::from(quiet.is_some()),
        (gateway.waited_micros() - before) / 1_000
    );

    // Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth
    // byte.
    let wide = Frame::fd(speed_id.to_id(), &[0; 32])?;
    println!(
        "fd           32 bytes travel at data length code {}",
        wide.dlc()
    );
    if let Err(error) = Frame::new(speed_id.to_id(), &[0; 9]) {
        println!("classic      refused nine bytes: {error}");
    }
    // ANCHOR_END: example

    assert_eq!(speed_id.to_id().raw(), 0x0CF0_0400);
    assert_eq!(unreported, 6);
    assert_eq!(gateway.received(), 2);
    assert_eq!(heard.len(), 3);
    assert_eq!(request_id.destination(), Some(ENGINE));
    assert!(quiet.is_none());
    assert_eq!(wide.dlc(), 13);
    assert_eq!(
        Frame::new(speed_id.to_id(), &[0; 9]),
        Err(CanError::DataTooLong)
    );

    Ok(())
}
