//! Every capability error works with the `?` operator in an ordinary program.
//!
//! `fn main() -> Result<(), Box<dyn Error>>` is the first thing most people write, and it
//! only compiles if each error type implements `Error` rather than just `Display`. Eight
//! crates implemented only `Display` until this was checked, so the failure showed up on
//! a reader's first line rather than anywhere the tests looked.

use std::error::Error;

use pamoja_can::{CanId, Frame as CanFrame};
use pamoja_gpio::i2c::Address;
use pamoja_lorawan::{Device, JoinGrant};
use pamoja_mesh::Frame as MeshFrame;
use pamoja_modbus::Adu;
use pamoja_sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};
use pamoja_serial::slip;
use pamoja_session::{AgreementKey, Role, Session};

/// Carries an error from every capability through `?` into one boxed error type.
#[test]
fn every_capability_error_crosses_the_question_mark_operator() -> Result<(), Box<dyn Error>> {
    let mesh = MeshFrame::broadcast(0x1234_5678, 1, b"level=high")?;
    let can = CanFrame::new(CanId::extended(0x0CF0_0400), &[0xFF; 8])?;
    let modbus = Adu::from_pdu(17, &[0x03, 0x02, 0x02, 0x2B])?;
    let sensor = Scratchpad::new(
        temperature_from_celsius(25.0625, Resolution::Bits12),
        Resolution::Bits12,
        75,
        -10,
    );
    let decoded = Scratchpad::parse(&sensor.to_bytes())?;
    let address = Address::seven_bit(0x76)?;

    let app_key = [7u8; 16];
    let device = Device::new([0u8; 8], [0u8; 8], app_key);
    let accept = JoinGrant::new(2, 19, 0x2601_2E43).accept(&app_key, 1);
    let joined = device.accept_join(accept.as_bytes(), 1)?;

    let mut framed = [0u8; 32];
    let framed = slip::encode(b"ok", &mut framed)?;

    let node = AgreementKey::from_seed(&[7u8; 32]);
    let gateway = AgreementKey::from_seed(&[9u8; 32]);
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).expect("the system random source");
    let mut uplink = Session::establish(&node, &gateway.public(), &salt, Role::Initiator);
    let mut downlink = Session::establish(&gateway, &node.public(), &salt, Role::Responder);
    let mut buffer = *b"flow=41.2";
    let sealed = uplink.seal(&mut buffer, b"pump-3");
    downlink.open(&sealed, &mut buffer, b"pump-3")?;

    assert_eq!(mesh.payload(), b"level=high");
    assert_eq!(can.dlc(), 8);
    assert_eq!(modbus.address(), 17);
    assert_eq!(decoded.temperature_celsius(), 25.0625);
    assert!(!address.is_reserved());
    assert_eq!(framed, 3, "two payload bytes and the end delimiter");
    assert_eq!(&buffer, b"flow=41.2");
    assert_eq!(joined.dev_addr(), 0x2601_2E43);
    Ok(())
}
