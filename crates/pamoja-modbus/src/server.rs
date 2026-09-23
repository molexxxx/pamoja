//! A Modbus server: the device's side of the conversation.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::adu::Adu;
use crate::error::ModbusError;
use crate::function::Exception;
use crate::pdu::Pdu;
use crate::request::Request;

/// The unit address every server acts on and none answers.
pub const BROADCAST: u8 = 0;

/// The lowest and highest address a server may have; 248 to 255 are reserved.
pub const UNITS: core::ops::RangeInclusive<u8> = 1..=247;

/// A Modbus device: a unit address and the four tables it serves, coils, discrete inputs,
/// holding registers, and input registers, each holding only the addresses it was given.
///
/// [`answer`](Server::answer) takes one RTU frame and returns the frame the device sends
/// back, following the Modbus over Serial Line specification: a frame that fails its CRC,
/// is for another unit, or is a broadcast gets no answer, and a broadcast write is still
/// carried out. [`serve`](Server::serve) is the same at the PDU level, for a transport with
/// its own framing. A request the device cannot serve is answered with the exception the
/// application protocol's state diagrams give: an unknown function, a quantity or value out
/// of range, then an address the device does not have.
///
/// # Examples
///
/// ```
/// use pamoja_modbus::{Adu, Pdu, Server};
///
/// // A meter at unit 17 holding voltage, current, and a fault word from register 107.
/// let mut meter = Server::new(17)?.with_holding_registers(107, &[2301, 418, 0]);
///
/// let request = Pdu::read_holding_registers(107, 3).to_adu(17);
/// let reply = meter.answer(request.as_bytes()).expect("the meter answers its own unit");
/// let registers: Vec<u16> = reply.response().registers()?.collect();
/// assert_eq!(registers, [2301, 418, 0]);
///
/// // Register 110 is not in its table.
/// let past = Pdu::read_holding_registers(108, 3).to_adu(17);
/// let refused = meter.answer(past.as_bytes()).expect("an exception is an answer");
/// assert_eq!(refused.exception(), Some(pamoja_modbus::Exception::IllegalDataAddress));
/// # Ok::<(), pamoja_modbus::ModbusError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    unit: u8,
    coils: BTreeMap<u16, bool>,
    discrete_inputs: BTreeMap<u16, bool>,
    holding_registers: BTreeMap<u16, u16>,
    input_registers: BTreeMap<u16, u16>,
    served: usize,
}

fn fill<T: Copy>(table: &mut BTreeMap<u16, T>, start: u16, values: &[T]) {
    for (address, &value) in (start..=u16::MAX).zip(values) {
        table.insert(address, value);
    }
}

fn read<T: Copy>(table: &BTreeMap<u16, T>, start: u16, count: usize) -> Vec<T> {
    (start..=u16::MAX)
        .take(count)
        .filter_map(|address| table.get(&address).copied())
        .collect()
}

fn holds<T>(table: &BTreeMap<u16, T>, start: u16, count: usize) -> bool {
    let last = usize::from(start) + count - 1;
    last <= usize::from(u16::MAX) && (start..=last as u16).all(|a| table.contains_key(&a))
}

impl Server {
    /// Makes a device at a unit address, with every table empty.
    ///
    /// # Arguments
    ///
    /// * `unit` - its address, 1 to 247.
    ///
    /// # Returns
    ///
    /// The device.
    ///
    /// # Errors
    ///
    /// [`ModbusError::UnitOutOfRange`] for 0, the broadcast address, or 248 to 255, which the
    /// specification reserves.
    pub fn new(unit: u8) -> Result<Server, ModbusError> {
        if !UNITS.contains(&unit) {
            return Err(ModbusError::UnitOutOfRange { unit });
        }
        Ok(Server {
            unit,
            coils: BTreeMap::new(),
            discrete_inputs: BTreeMap::new(),
            holding_registers: BTreeMap::new(),
            input_registers: BTreeMap::new(),
            served: 0,
        })
    }

    /// Gives the device coils at consecutive addresses.
    ///
    /// # Arguments
    ///
    /// * `start` - the first coil's address.
    /// * `values` - their states, in address order.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub fn with_coils(mut self, start: u16, values: &[bool]) -> Server {
        fill(&mut self.coils, start, values);
        self
    }

    /// Gives the device discrete inputs at consecutive addresses.
    ///
    /// # Arguments
    ///
    /// * `start` - the first input's address.
    /// * `values` - their states, in address order.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub fn with_discrete_inputs(mut self, start: u16, values: &[bool]) -> Server {
        fill(&mut self.discrete_inputs, start, values);
        self
    }

    /// Gives the device holding registers at consecutive addresses.
    ///
    /// # Arguments
    ///
    /// * `start` - the first register's address.
    /// * `values` - their values, in address order.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub fn with_holding_registers(mut self, start: u16, values: &[u16]) -> Server {
        fill(&mut self.holding_registers, start, values);
        self
    }

    /// Gives the device input registers at consecutive addresses.
    ///
    /// # Arguments
    ///
    /// * `start` - the first register's address.
    /// * `values` - their values, in address order.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub fn with_input_registers(mut self, start: u16, values: &[u16]) -> Server {
        fill(&mut self.input_registers, start, values);
        self
    }

    /// Sets holding registers from an address on, adding any the device did not have, as its
    /// own measurements change.
    ///
    /// # Arguments
    ///
    /// * `start` - the first register's address.
    /// * `values` - their values, in address order.
    pub fn set_holding_registers(&mut self, start: u16, values: &[u16]) {
        fill(&mut self.holding_registers, start, values);
    }

    /// Sets input registers from an address on, adding any the device did not have.
    ///
    /// # Arguments
    ///
    /// * `start` - the first register's address.
    /// * `values` - their values, in address order.
    pub fn set_input_registers(&mut self, start: u16, values: &[u16]) {
        fill(&mut self.input_registers, start, values);
    }

    /// Sets coils from an address on, adding any the device did not have.
    ///
    /// # Arguments
    ///
    /// * `start` - the first coil's address.
    /// * `values` - their states, in address order.
    pub fn set_coils(&mut self, start: u16, values: &[bool]) {
        fill(&mut self.coils, start, values);
    }

    /// Sets discrete inputs from an address on, adding any the device did not have.
    ///
    /// # Arguments
    ///
    /// * `start` - the first input's address.
    /// * `values` - their states, in address order.
    pub fn set_discrete_inputs(&mut self, start: u16, values: &[bool]) {
        fill(&mut self.discrete_inputs, start, values);
    }

    /// Returns a coil's state, or `None` when the device has no coil there.
    pub fn coil(&self, address: u16) -> Option<bool> {
        self.coils.get(&address).copied()
    }

    /// Returns a discrete input's state, or `None` when the device has no input there.
    pub fn discrete_input(&self, address: u16) -> Option<bool> {
        self.discrete_inputs.get(&address).copied()
    }

    /// Returns a holding register's value, or `None` when the device has no register there.
    pub fn holding_register(&self, address: u16) -> Option<u16> {
        self.holding_registers.get(&address).copied()
    }

    /// Returns an input register's value, or `None` when the device has no register there.
    pub fn input_register(&self, address: u16) -> Option<u16> {
        self.input_registers.get(&address).copied()
    }

    /// Returns the device's unit address.
    pub fn unit(&self) -> u8 {
        self.unit
    }

    /// Returns how many requests the device has carried out, broadcasts included and
    /// refusals not.
    pub fn served(&self) -> usize {
        self.served
    }

    /// Serves one request PDU.
    ///
    /// # Arguments
    ///
    /// * `pdu` - the request: a function code followed by its data.
    ///
    /// # Returns
    ///
    /// The reply PDU: the normal response, or the exception the request earned.
    pub fn serve(&mut self, pdu: &[u8]) -> Pdu {
        let function = pdu.first().copied().unwrap_or(0);
        let request = match Request::parse(pdu) {
            Ok(request) => request,
            Err(exception) => return Pdu::exception(function, exception),
        };
        let (start, count) = request.span();
        let present = match request {
            Request::ReadCoils { .. }
            | Request::WriteSingleCoil { .. }
            | Request::WriteMultipleCoils { .. } => holds(&self.coils, start, count),
            Request::ReadDiscreteInputs { .. } => holds(&self.discrete_inputs, start, count),
            Request::ReadHoldingRegisters { .. }
            | Request::WriteSingleRegister { .. }
            | Request::WriteMultipleRegisters { .. } => {
                holds(&self.holding_registers, start, count)
            }
            Request::ReadInputRegisters { .. } => holds(&self.input_registers, start, count),
        };
        if !present {
            return Pdu::exception(function, Exception::IllegalDataAddress);
        }
        self.served += 1;
        let quantity = count as u16;
        let reply = match request {
            Request::ReadCoils { .. } => Pdu::read_coils_reply(&read(&self.coils, start, count)),
            Request::ReadDiscreteInputs { .. } => {
                Pdu::read_discrete_inputs_reply(&read(&self.discrete_inputs, start, count))
            }
            Request::ReadHoldingRegisters { .. } => {
                Pdu::read_holding_registers_reply(&read(&self.holding_registers, start, count))
            }
            Request::ReadInputRegisters { .. } => {
                Pdu::read_input_registers_reply(&read(&self.input_registers, start, count))
            }
            Request::WriteSingleCoil { address, on } => {
                self.coils.insert(address, on);
                Ok(Pdu::write_single_coil(address, on))
            }
            Request::WriteSingleRegister { address, value } => {
                self.holding_registers.insert(address, value);
                Ok(Pdu::write_single_register(address, value))
            }
            Request::WriteMultipleCoils { values, .. } => {
                for (address, on) in (start..=u16::MAX).zip(values) {
                    self.coils.insert(address, on);
                }
                Ok(Pdu::write_multiple_coils_reply(start, quantity))
            }
            Request::WriteMultipleRegisters { values, .. } => {
                for (address, value) in (start..=u16::MAX).zip(values) {
                    self.holding_registers.insert(address, value);
                }
                Ok(Pdu::write_multiple_registers_reply(start, quantity))
            }
        };
        reply.unwrap_or_else(|_| Pdu::exception(function, Exception::ServerDeviceFailure))
    }

    /// Answers one RTU frame, as a device on the line does.
    ///
    /// # Arguments
    ///
    /// * `frame` - the frame as it came off the line, CRC included.
    ///
    /// # Returns
    ///
    /// The frame to send back, or `None` when the device stays silent: the frame failed its
    /// CRC, is for another unit, or is a broadcast, whose write the device still carries out.
    pub fn answer(&mut self, frame: &[u8]) -> Option<Adu> {
        let adu = Adu::parse(frame).ok()?;
        if adu.address() == BROADCAST {
            if Request::parse(adu.pdu()).is_ok_and(|request| !request.reads()) {
                self.serve(adu.pdu());
            }
            return None;
        }
        if adu.address() != self.unit {
            return None;
        }
        Some(self.serve(adu.pdu()).to_adu(self.unit))
    }
}

/// A server on the far end of a simulated port answers each write as one frame.
#[cfg(feature = "port")]
impl pamoja_hal::port::Peer for Server {
    fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.answer(bytes)
            .map(|adu| adu.as_bytes().to_vec())
            .unwrap_or_default()
    }
}

/// Several servers on one simulated line, as devices share an RS485 pair: every frame reaches
/// all of them, the one it is addressed to answers, and each carries out a broadcast write.
///
/// A line is a [`Peer`](pamoja_hal::port::Peer), so a
/// [`SerialPort::simulated`](pamoja_hal::port::SerialPort::simulated) port with a line on
/// the far end is a bus of devices a [`Client`](crate::Client) polls. Each server is kept
/// behind a shared handle, so a program looks at what a write did to it afterward.
///
/// # Examples
///
/// ```
/// use pamoja_hal::port::{Parity, SerialPort, Settings};
/// use pamoja_modbus::{Client, Line, Server};
///
/// let mut line = Line::new();
/// let meter = line.attach(Server::new(17)?.with_input_registers(0, &[2301]));
/// let pump = line.attach(Server::new(18)?.with_coils(0, &[false]));
///
/// let settings = Settings::new(19_200).with_parity(Parity::Even);
/// let mut client = Client::new(SerialPort::simulated(settings, line));
/// assert_eq!(client.read_input_registers(17, 0, 1)?, [2301]);
/// client.write_single_coil(18, 0, true)?;
///
/// assert_eq!(pump.lock().unwrap().coil(0), Some(true));
/// assert_eq!(meter.lock().unwrap().served(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(feature = "port")]
#[derive(Clone, Debug, Default)]
pub struct Line {
    servers: Vec<std::sync::Arc<std::sync::Mutex<Server>>>,
}

#[cfg(feature = "port")]
impl Line {
    /// Makes a line with no devices on it.
    pub fn new() -> Line {
        Line::default()
    }

    /// Puts a device on the line.
    ///
    /// # Arguments
    ///
    /// * `server` - the device.
    ///
    /// # Returns
    ///
    /// A shared handle to it, for looking at it or changing its tables later.
    pub fn attach(&mut self, server: Server) -> std::sync::Arc<std::sync::Mutex<Server>> {
        let shared = std::sync::Arc::new(std::sync::Mutex::new(server));
        self.servers.push(std::sync::Arc::clone(&shared));
        shared
    }

    /// Puts a device the program already shares on the line.
    ///
    /// # Arguments
    ///
    /// * `server` - the shared device.
    pub fn attach_shared(&mut self, server: std::sync::Arc<std::sync::Mutex<Server>>) {
        self.servers.push(server);
    }

    /// Returns the unit addresses on the line, in the order they were attached.
    pub fn units(&self) -> Vec<u8> {
        self.servers
            .iter()
            .map(|server| lock(server).unit())
            .collect()
    }
}

#[cfg(feature = "port")]
fn lock(server: &std::sync::Mutex<Server>) -> std::sync::MutexGuard<'_, Server> {
    server
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(feature = "port")]
impl pamoja_hal::port::Peer for Line {
    fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut replies = Vec::new();
        for server in &self.servers {
            if let Some(reply) = lock(server).answer(bytes) {
                replies.extend_from_slice(reply.as_bytes());
            }
        }
        replies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Response;

    fn reply(server: &mut Server, request: Pdu) -> Pdu {
        server.serve(request.as_bytes())
    }

    #[test]
    fn it_answers_each_specification_example_with_the_frame_the_specification_shows() {
        // 6.1: coils 20 to 38, at addresses 19 to 37, answered CD 6B 05.
        let coils = [
            true, false, true, true, false, false, true, true, true, true, false, true, false,
            true, true, false, true, false, true,
        ];
        let mut device = Server::new(1)
            .unwrap()
            .with_coils(0x0013, &coils)
            .with_holding_registers(0x006B, &[0x022B, 0x0000, 0x0064])
            .with_input_registers(0x0008, &[0x000A])
            .with_holding_registers(0x0001, &[0, 0]);
        assert_eq!(
            reply(&mut device, Pdu::read_coils(0x0013, 0x0013)).as_bytes(),
            &[0x01, 0x03, 0xCD, 0x6B, 0x05]
        );
        // 6.2: discrete inputs 197 to 218, at addresses 196 to 217, answered AC DB 35.
        let inputs = [
            false, false, true, true, false, true, false, true, true, true, false, true, true,
            false, true, true, true, false, true, false, true, true,
        ];
        let mut device = device.with_discrete_inputs(0x00C4, &inputs);
        assert_eq!(
            reply(&mut device, Pdu::read_discrete_inputs(0x00C4, 0x0016)).as_bytes(),
            &[0x02, 0x03, 0xAC, 0xDB, 0x35]
        );
        // 6.3 and 6.4.
        assert_eq!(
            reply(&mut device, Pdu::read_holding_registers(0x006B, 3)).as_bytes(),
            &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64]
        );
        assert_eq!(
            reply(&mut device, Pdu::read_input_registers(0x0008, 1)).as_bytes(),
            &[0x04, 0x02, 0x00, 0x0A]
        );
        // 6.5: coil 173 on, answered with an echo.
        let on = Pdu::write_single_coil(0x00AC, true);
        let mut device = device.with_coils(0x00AC, &[false]);
        assert_eq!(reply(&mut device, on).as_bytes(), on.as_bytes());
        assert_eq!(device.coil(0x00AC), Some(true));
        // 6.11: coils 20 to 29 written from CD 01, answered with where and how many.
        let ten = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let many = Pdu::write_multiple_coils(0x0013, &ten).unwrap();
        assert_eq!(
            reply(&mut device, many).as_bytes(),
            &[0x0F, 0x00, 0x13, 0x00, 0x0A]
        );
        // 6.6 and 6.12.
        let single = Pdu::write_single_register(0x0001, 0x0003);
        assert_eq!(reply(&mut device, single).as_bytes(), single.as_bytes());
        let two = Pdu::write_multiple_registers(0x0001, &[0x000A, 0x0102]).unwrap();
        assert_eq!(
            reply(&mut device, two).as_bytes(),
            &[0x10, 0x00, 0x01, 0x00, 0x02]
        );
        assert_eq!(device.holding_register(0x0002), Some(0x0102));
        assert_eq!(device.coil(0x0013 + 9), Some(false));
        assert_eq!(device.served(), 8);
    }

    #[test]
    fn it_refuses_in_the_order_the_state_diagrams_check() {
        let mut device = Server::new(1).unwrap().with_holding_registers(0, &[0; 100]);
        // Section 7: an output that does not exist answers 81 02.
        assert_eq!(
            reply(&mut device, Pdu::read_coils(0x04A1, 1)).as_bytes(),
            &[0x81, 0x02]
        );
        // Exception code 02's definition: of 100 registers, 96 and 4 more is fine, and 96
        // and 5 more is not.
        assert!(
            Response::new(reply(&mut device, Pdu::read_holding_registers(96, 4)).as_bytes())
                .exception()
                .is_none()
        );
        assert_eq!(
            reply(&mut device, Pdu::read_holding_registers(96, 5)).as_bytes(),
            &[0x83, 0x02]
        );
        // A quantity out of range is refused as a value before any address is looked at.
        assert_eq!(
            reply(&mut device, Pdu::read_holding_registers(5_000, 126)).as_bytes(),
            &[0x83, 0x03]
        );
        // A function the device does not serve.
        let diagnostics = Pdu::raw(0x08, &[0x00, 0x00, 0xA5, 0x37]).unwrap();
        assert_eq!(reply(&mut device, diagnostics).as_bytes(), &[0x88, 0x01]);
        // A span that would run past address 0xFFFF.
        assert_eq!(
            reply(&mut device, Pdu::read_holding_registers(0xFFFF, 2)).as_bytes(),
            &[0x83, 0x02]
        );
        assert_eq!(device.served(), 1, "only the one good read was carried out");
    }

    #[test]
    fn a_frame_gets_an_answer_only_when_it_should() {
        let mut device = Server::new(17)
            .unwrap()
            .with_holding_registers(107, &[2301, 418, 0]);
        let request = Pdu::read_holding_registers(107, 1).to_adu(17);
        let answer = device.answer(request.as_bytes()).unwrap();
        assert_eq!(answer.address(), 17);

        // Another unit's frame, and a frame that fails its CRC, get silence.
        let other = Pdu::read_holding_registers(107, 1).to_adu(18);
        assert!(device.answer(other.as_bytes()).is_none());
        let mut mangled = request.as_bytes().to_vec();
        mangled[3] ^= 0x01;
        assert!(device.answer(&mangled).is_none());

        // A broadcast write is carried out and not answered; a broadcast read is neither.
        let write = Pdu::write_single_register(108, 500).to_adu(BROADCAST);
        assert!(device.answer(write.as_bytes()).is_none());
        assert_eq!(device.holding_register(108), Some(500));
        let read_all = Pdu::read_holding_registers(107, 1).to_adu(BROADCAST);
        assert!(device.answer(read_all.as_bytes()).is_none());
        assert_eq!(device.served(), 2);
    }

    #[test]
    fn a_unit_outside_one_to_247_is_refused() {
        for unit in [0, 248, 255] {
            assert_eq!(Server::new(unit), Err(ModbusError::UnitOutOfRange { unit }));
        }
        assert!(Server::new(247).is_ok());
    }

    #[test]
    fn many_coils_and_discrete_inputs_round_trip() {
        let ten = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let mut device = Server::new(1)
            .unwrap()
            .with_coils(0x0013, &[false; 10])
            .with_discrete_inputs(0x00C4, &[true, false, true]);
        let write = Pdu::write_multiple_coils(0x0013, &ten).unwrap();
        assert_eq!(
            reply(&mut device, write).as_bytes(),
            &[0x0F, 0x00, 0x13, 0x00, 0x0A]
        );
        let back = reply(&mut device, Pdu::read_coils(0x0013, 10));
        assert!(Response::new(back.as_bytes()).coils(10).unwrap().eq(ten));
        let inputs = reply(&mut device, Pdu::read_discrete_inputs(0x00C4, 3));
        assert_eq!(inputs.as_bytes(), &[0x02, 0x01, 0x05]);
    }
}
