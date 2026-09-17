//! Driving the SX1261 that listens beside a concentrator.
//!
//! [`sx1261`](super::sx1261) and [`lbt`](super::lbt) build the commands this radio takes;
//! this sends them over its own SPI device. The radio is not behind the concentrator: it has
//! its own chip select, its own reset line, and no BUSY pin on any concentrator card, so every
//! command waits a millisecond before it goes out, as Semtech's `sx1261_spi_w` and
//! `sx1261_spi_r` do, rather than watching a pin nobody wired.
//!
//! Bringing it up is four steps, and [`Sx1261::bring_up`] takes them in the reference's
//! order: reset, load the patch that adds the carrier check and the spectral scan, calibrate
//! the image for the band, and set the receiver up. After that a gateway points it at a
//! channel with [`listen`](Sx1261::listen) and starts a [`check`](Sx1261::check) before each
//! transmission on a channel that needs one; the concentrator reads the answer at the moment
//! the packet would leave.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{self, OutputPin};
use embedded_hal::spi::SpiDevice;

use super::lbt::{self, ScanTime};
use super::sx1261::{
    self, Bandwidth, Frame, Level, ScanStatus, Status, OP_GET_DEVICE_ERRORS, OP_GET_STATUS,
    OP_READ_REGISTER, PRAM_WORDS, REG_SCAN_STATUS, SCAN_LEVELS, SCAN_TRANSFER,
};

/// How long the reset line is held low, and then how long the radio is given after it, in
/// microseconds: the tenth of a second Semtech's `reset_lgw.sh` waits after every edge.
pub const LISTENER_RESET_US: u32 = 100_000;

/// How long every command waits before it goes out, in microseconds, in place of the BUSY pin
/// no concentrator card wires.
pub const COMMAND_WAIT_US: u32 = 1_000;

/// The longest exchange this driver makes: a spectral scan's counts, and the opcode before them.
const LONGEST: usize = 1 + SCAN_TRANSFER;

/// Why the listening radio could not be driven.
#[derive(Debug)]
pub enum ListenerError<E> {
    /// The SPI device failed.
    Spi(E),
    /// The reset line could not be driven.
    Pin(digital::ErrorKind),
    /// The radio did not answer as a parked radio does, so it is not there or not powered.
    NotParked {
        /// The status byte it answered with.
        status: u8,
    },
    /// The patch was written and the radio still does not report running it.
    Unpatched,
    /// A carrier outside every band the radio calibrates its image for.
    UnsupportedBand {
        /// The carrier asked for, in hertz.
        hertz: u32,
    },
    /// The radio reported its image calibration failed.
    Calibration,
    /// The scan status register held a value that is none of the four states.
    ScanStatus {
        /// What it held.
        byte: u8,
    },
}

impl<E: core::fmt::Debug> core::error::Error for ListenerError<E> {}

impl<E: core::fmt::Debug> core::fmt::Display for ListenerError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ListenerError::Spi(error) => write!(f, "the SX1261's SPI device failed: {error:?}"),
            ListenerError::Pin(kind) => write!(f, "the SX1261's reset line failed: {kind:?}"),
            ListenerError::NotParked { status } => write!(
                f,
                "the SX1261 answered {status:#04x} rather than {:#04x}, so it is not answering",
                sx1261::STANDBY_READY
            ),
            ListenerError::Unpatched => {
                f.write_str("the SX1261 does not report running the patch that was written")
            }
            ListenerError::UnsupportedBand { hertz } => write!(
                f,
                "{hertz} Hz is outside every band the SX1261 calibrates over"
            ),
            ListenerError::Calibration => {
                f.write_str("the SX1261 reported its image calibration failed")
            }
            ListenerError::ScanStatus { byte } => {
                write!(
                    f,
                    "the SX1261's scan status read {byte:#04x}, which is no state"
                )
            }
        }
    }
}

/// The SX1261 beside a concentrator, on its own SPI device with its reset line.
pub struct Sx1261<SPI, RESET, D> {
    spi: SPI,
    reset: RESET,
    delay: D,
}

impl<SPI, RESET, D> Sx1261<SPI, RESET, D> {
    /// Takes the radio's bus, reset line and delay.
    ///
    /// # Arguments
    ///
    /// * `spi` - the radio's own SPI device, `/dev/spidev0.1` on Semtech's reference card.
    /// * `reset` - its reset line, which is held low to reset, the opposite of the
    ///   concentrator's.
    /// * `delay` - a blocking delay.
    ///
    /// # Returns
    ///
    /// The driver. Nothing is sent until a method is called.
    pub const fn new(spi: SPI, reset: RESET, delay: D) -> Sx1261<SPI, RESET, D> {
        Sx1261 { spi, reset, delay }
    }

    /// Gives the bus, the line and the delay back.
    ///
    /// # Returns
    ///
    /// What [`new`](Self::new) was given.
    pub fn release(self) -> (SPI, RESET, D) {
        (self.spi, self.reset, self.delay)
    }
}

impl<SPI, RESET, D> Sx1261<SPI, RESET, D>
where
    SPI: SpiDevice,
    RESET: OutputPin,
    D: DelayNs,
{
    /// Brings the radio up for carrier checks and spectral scans.
    ///
    /// # Arguments
    ///
    /// * `patch` - the words of Semtech's `sx1261_pram.var`, read with
    ///   [`read_patch`](super::sx1261::read_patch).
    /// * `hertz` - a carrier in the band the gateway works in, which the image is calibrated
    ///   for; the reference uses the first front end's.
    ///
    /// # Returns
    ///
    /// `Ok(())` with the radio patched, calibrated and parked.
    ///
    /// # Errors
    ///
    /// Returns what [`reset`](Self::reset), [`load_patch`](Self::load_patch),
    /// [`calibrate`](Self::calibrate) or [`set_up`](Self::set_up) returns.
    pub fn bring_up(
        &mut self,
        patch: &[u32; PRAM_WORDS],
        hertz: u32,
    ) -> Result<(), ListenerError<SPI::Error>> {
        self.reset()?;
        self.load_patch(patch)?;
        self.calibrate(hertz)?;
        self.set_up()
    }

    /// Resets the radio: its line low, then high, each held a tenth of a second.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Pin`] if the line cannot be driven.
    pub fn reset(&mut self) -> Result<(), ListenerError<SPI::Error>> {
        self.reset.set_low().map_err(pin)?;
        self.delay.delay_us(LISTENER_RESET_US);
        self.reset.set_high().map_err(pin)?;
        self.delay.delay_us(LISTENER_RESET_US);
        Ok(())
    }

    /// Sends one command.
    ///
    /// # Arguments
    ///
    /// * `frame` - the command and its payload.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn command(&mut self, frame: &Frame) -> Result<(), ListenerError<SPI::Error>> {
        self.send(frame.opcode(), frame.payload())
    }

    /// Sends an opcode and its payload as one transfer.
    ///
    /// # Arguments
    ///
    /// * `opcode` - the command.
    /// * `payload` - what follows it.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn send(&mut self, opcode: u8, payload: &[u8]) -> Result<(), ListenerError<SPI::Error>> {
        let mut out = [0u8; LONGEST];
        let len = 1 + payload.len().min(LONGEST - 1);
        out[0] = opcode;
        out[1..len].copy_from_slice(&payload[..len - 1]);
        self.delay.delay_us(COMMAND_WAIT_US);
        self.spi.write(&out[..len]).map_err(ListenerError::Spi)
    }

    /// Sends an opcode with a payload and reads back what the radio answers over it.
    ///
    /// # Arguments
    ///
    /// * `opcode` - the command.
    /// * `exchange` - the payload going out, replaced by the bytes that came back over it.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn query(
        &mut self,
        opcode: u8,
        exchange: &mut [u8],
    ) -> Result<(), ListenerError<SPI::Error>> {
        let mut buffer = [0u8; LONGEST];
        let len = 1 + exchange.len().min(LONGEST - 1);
        buffer[0] = opcode;
        buffer[1..len].copy_from_slice(&exchange[..len - 1]);
        self.delay.delay_us(COMMAND_WAIT_US);
        self.spi
            .transfer_in_place(&mut buffer[..len])
            .map_err(ListenerError::Spi)?;
        exchange[..len - 1].copy_from_slice(&buffer[1..len]);
        Ok(())
    }

    /// Asks the radio what it is doing.
    ///
    /// # Returns
    ///
    /// The status it answered with.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn status(&mut self) -> Result<Status, ListenerError<SPI::Error>> {
        let mut answer = [0u8; 1];
        self.query(OP_GET_STATUS, &mut answer)?;
        Ok(Status::read(answer[0]))
    }

    /// Reads bytes from the radio's memory.
    ///
    /// # Arguments
    ///
    /// * `address` - where to start.
    /// * `exchange` - the address, a byte the radio answers over, then room for the data. The
    ///   first three bytes are filled in here, and the data lands after them.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn read_memory(
        &mut self,
        address: u16,
        exchange: &mut [u8],
    ) -> Result<(), ListenerError<SPI::Error>> {
        if let Some(head) = exchange.get_mut(..3) {
            head.copy_from_slice(&[(address >> 8) as u8, address as u8, 0]);
        }
        self.query(OP_READ_REGISTER, exchange)
    }

    /// Loads the patch that adds the carrier check and the spectral scan, and proves it took.
    ///
    /// Semtech's `sx1261_load_pram`: park the radio and confirm it is parked, open the patch
    /// area, write each word, close it, commit, and read back a version string that ends in
    /// [`PRAM_VERSION`](super::sx1261::PRAM_VERSION).
    ///
    /// # Arguments
    ///
    /// * `patch` - the patch's words.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::NotParked`] if the radio does not answer as a parked radio,
    /// [`ListenerError::Unpatched`] if the version it reports afterward is not the patch's,
    /// and [`ListenerError::Spi`] if a transfer fails.
    pub fn load_patch(
        &mut self,
        patch: &[u32; PRAM_WORDS],
    ) -> Result<(), ListenerError<SPI::Error>> {
        self.command(&sx1261::standby(sx1261::Standby::Rc))?;
        self.parked()?;

        // The reference reads the version it is about to replace as well, which costs one
        // transfer and keeps this sequence the one a capture of it shows.
        let (address, len) = sx1261::version_read();
        let mut before = [0u8; sx1261::VERSION_TRANSFER];
        self.read_memory(address, &mut before[..len])?;

        self.command(&sx1261::open_patch())?;
        for (index, word) in patch.iter().enumerate() {
            self.command(&sx1261::patch_word(index, *word))?;
        }
        self.command(&sx1261::close_patch())?;
        self.command(&sx1261::commit())?;

        let (address, len) = sx1261::version_read();
        let mut answer = [0u8; sx1261::VERSION_TRANSFER];
        self.read_memory(address, &mut answer[..len])?;
        if sx1261::patched(&answer) {
            Ok(())
        } else {
            Err(ListenerError::Unpatched)
        }
    }

    /// Calibrates the radio's image rejection for the band a carrier falls in.
    ///
    /// # Arguments
    ///
    /// * `hertz` - a carrier in the band.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::UnsupportedBand`] for a carrier outside every band,
    /// [`ListenerError::Calibration`] if the radio reports the calibration failed, and
    /// [`ListenerError::Spi`] if a transfer fails.
    pub fn calibrate(&mut self, hertz: u32) -> Result<(), ListenerError<SPI::Error>> {
        self.status()?;
        let frame = sx1261::calibrate(hertz).ok_or(ListenerError::UnsupportedBand { hertz })?;
        self.command(&frame)?;
        self.delay.delay_ms(sx1261::CALIBRATION_WAIT_MS);

        let mut errors = [0u8; 3];
        self.query(OP_GET_DEVICE_ERRORS, &mut errors)?;
        if sx1261::calibration_failed(errors) {
            return Err(ListenerError::Calibration);
        }
        Ok(())
    }

    /// Sets the receiver up for carrier checks: parked, its buffers placed, and its
    /// sensitivity trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::NotParked`] if the radio does not settle parked, and
    /// [`ListenerError::Spi`] if a transfer fails.
    pub fn set_up(&mut self) -> Result<(), ListenerError<SPI::Error>> {
        let [park, buffers, sensitivity] = sx1261::setup();
        self.command(&park)?;
        self.parked()?;
        self.command(&buffers)?;
        self.command(&sensitivity)
    }

    /// Points the receiver at a channel and leaves it listening.
    ///
    /// # Arguments
    ///
    /// * `hertz` - the channel's carrier.
    /// * `bandwidth` - the channel's width.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if a transfer fails.
    pub fn listen(
        &mut self,
        hertz: u32,
        bandwidth: Bandwidth,
    ) -> Result<(), ListenerError<SPI::Error>> {
        for frame in sx1261::receive(hertz, bandwidth) {
            self.command(&frame)?;
        }
        Ok(())
    }

    /// Starts a carrier check on the channel the receiver listens on, and waits out its scan.
    ///
    /// The radio then holds its answer on the pin the concentrator reads when the packet is
    /// due to leave, so the transmission that needs the check is armed after this returns.
    ///
    /// # Arguments
    ///
    /// * `scan_time` - how long to listen.
    /// * `threshold_dbm` - the level above which the channel counts as busy, with the board's
    ///   RSSI offset already added.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn check(
        &mut self,
        scan_time: ScanTime,
        threshold_dbm: i8,
    ) -> Result<(), ListenerError<SPI::Error>> {
        let (opcode, payload) = lbt::start(scan_time, threshold_dbm);
        self.send(opcode, &payload)?;
        self.delay.delay_us(u32::from(scan_time.micros()));
        Ok(())
    }

    /// Ends a carrier check and parks the receiver on frequency.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if a transfer fails.
    pub fn stop(&mut self) -> Result<(), ListenerError<SPI::Error>> {
        let ((release, released), (park, parked)) = lbt::stop();
        self.send(release, &released)?;
        self.send(park, &parked)
    }

    /// Starts a spectral scan of a 125 kHz channel.
    ///
    /// # Arguments
    ///
    /// * `hertz` - the channel's carrier.
    /// * `scans` - how many times to sample it.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if a transfer fails.
    pub fn start_scan(&mut self, hertz: u32, scans: u16) -> Result<(), ListenerError<SPI::Error>> {
        self.listen(hertz, Bandwidth::Khz125)?;
        let (opcode, payload) = lbt::spectral_scan(scans);
        self.send(opcode, &payload)
    }

    /// How far along a spectral scan is.
    ///
    /// # Returns
    ///
    /// The scan's state.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::ScanStatus`] for a value that is no state, and
    /// [`ListenerError::Spi`] if the transfer fails.
    pub fn scan_status(&mut self) -> Result<ScanStatus, ListenerError<SPI::Error>> {
        let mut answer = [0u8; 4];
        self.read_memory(REG_SCAN_STATUS, &mut answer)?;
        ScanStatus::of(answer[3]).ok_or(ListenerError::ScanStatus { byte: answer[3] })
    }

    /// Reads what a finished spectral scan counted.
    ///
    /// # Arguments
    ///
    /// * `rssi_offset` - the board's correction, in dB.
    ///
    /// # Returns
    ///
    /// The count at each level, strongest first.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn scan_counts(
        &mut self,
        rssi_offset: i8,
    ) -> Result<[Level; SCAN_LEVELS], ListenerError<SPI::Error>> {
        let mut answer = [0u8; SCAN_TRANSFER];
        self.read_memory(sx1261::REG_SCAN_RESULTS, &mut answer)?;
        Ok(sx1261::scan_counts(&answer, rssi_offset)
            .unwrap_or([Level { dbm: 0, count: 0 }; SCAN_LEVELS]))
    }

    /// Abandons a spectral scan, which frees the radio for a carrier check.
    ///
    /// # Errors
    ///
    /// Returns [`ListenerError::Spi`] if the transfer fails.
    pub fn abort_scan(&mut self) -> Result<(), ListenerError<SPI::Error>> {
        self.command(&sx1261::write_register(sx1261::REG_RSSI_WINDOW, 0x00))
    }

    fn parked(&mut self) -> Result<(), ListenerError<SPI::Error>> {
        let status = self.status()?;
        if status.parked() {
            Ok(())
        } else {
            Err(ListenerError::NotParked {
                status: status.byte,
            })
        }
    }
}

/// Turns a pin failure into an error, keeping only what the trait promises.
fn pin<P: digital::Error, E>(error: P) -> ListenerError<E> {
    ListenerError::Pin(error.kind())
}

#[cfg(test)]
mod tests {
    use embedded_hal::digital::PinState;
    use pamoja_hal::script::{DelayLog, PinScript, SpiScript, SpiStep};

    use super::*;

    /// A radio whose bus answers with exactly these steps.
    fn driven(steps: Vec<SpiStep>) -> Sx1261<SpiScript, PinScript, DelayLog> {
        Sx1261::new(SpiScript::new(steps), PinScript::new([]), DelayLog::new())
    }

    /// Semtech's `sx1261_get_status`: the opcode and one byte the status comes back over.
    fn status(answer: u8) -> SpiStep {
        SpiStep::transfer(vec![0xc0, 0x00], vec![0x00, answer])
    }

    /// `sx1261_pram_get_version`: a register read of fifteen characters at 0x0320.
    fn version(text: &[u8; 15]) -> SpiStep {
        let mut out = vec![0x1d, 0x03, 0x20, 0x00];
        out.extend([0u8; 15]);
        let mut reply = vec![0x00; 4];
        reply.extend(text);
        SpiStep::transfer(out, reply)
    }

    /// A patch of recognizable words, since the real one is Semtech's to distribute.
    fn patch() -> [u32; PRAM_WORDS] {
        core::array::from_fn(|index| 0x0033_7f00 | index as u32)
    }

    /// Every transfer `sx1261_load_pram`, `sx1261_calibrate` and `sx1261_setup` make, in their
    /// order, for a gateway at 922.1 MHz.
    fn bring_up_transcript(patch: &[u32; PRAM_WORDS], answered: &[u8; 15]) -> Vec<SpiStep> {
        let mut steps = vec![
            SpiStep::write([0x80, 0x00]),
            status(0x22),
            version(b"sx1261_v1.10000"),
            SpiStep::write([0x0d, 0x06, 0x10, 0x10]),
        ];
        for (index, word) in patch.iter().enumerate() {
            let address = 0x8000 + 4 * index as u16;
            let [a, b, c, d] = word.to_be_bytes();
            steps.push(SpiStep::write([
                0x0d,
                (address >> 8) as u8,
                address as u8,
                a,
                b,
                c,
                d,
            ]));
        }
        steps.extend([
            SpiStep::write([0x0d, 0x06, 0x10, 0x00]),
            SpiStep::write([0xd9]),
            version(answered),
            status(0x22),
            SpiStep::write([0x98, 0xe1, 0xe9]),
            SpiStep::transfer([0x17, 0x00, 0x00, 0x00], [0x00, 0x00, 0x00, 0x00]),
            SpiStep::write([0x80, 0x00]),
            status(0x22),
            SpiStep::write([0x8f, 0x80, 0x80]),
            SpiStep::write([0x0d, 0x08, 0xac, 0xcb]),
        ]);
        steps
    }

    #[test]
    fn bringing_the_radio_up_follows_the_reference_transfer_for_transfer() {
        let patch = patch();
        let mut radio = driven(bring_up_transcript(&patch, b"sx1261_v1.12D06"));
        radio
            .bring_up(&patch, 922_100_000)
            .expect("every transfer as the reference makes it");

        let (spi, reset, delay) = radio.release();
        assert!(spi.done(), "{} transfers left unmade", spi.remaining());
        assert_eq!(
            reset.driven(),
            [PinState::Low, PinState::High],
            "held low to reset, the opposite of the concentrator"
        );
        assert!(delay.total_micros() >= 2 * u64::from(LISTENER_RESET_US));
    }

    #[test]
    fn a_patch_the_radio_does_not_report_running_is_refused() {
        let patch = patch();
        let mut steps = bring_up_transcript(&patch, b"sx1261_v1.10000");
        steps.truncate(steps.len() - 7);
        let mut radio = driven(steps);
        assert!(matches!(
            radio.bring_up(&patch, 922_100_000),
            Err(ListenerError::Unpatched)
        ));
    }

    #[test]
    fn a_radio_that_does_not_answer_parked_goes_no_further() {
        let mut radio = driven(vec![SpiStep::write([0x80, 0x00]), status(0x00)]);
        assert!(matches!(
            radio.load_patch(&patch()),
            Err(ListenerError::NotParked { status: 0x00 })
        ));
    }

    #[test]
    fn a_failed_or_impossible_calibration_is_reported() {
        let mut radio = driven(vec![
            status(0x22),
            SpiStep::write([0x98, 0xd7, 0xdb]),
            SpiStep::transfer([0x17, 0x00, 0x00, 0x00], [0x00, 0x00, 0x00, 0x10]),
        ]);
        assert!(matches!(
            radio.calibrate(868_100_000),
            Err(ListenerError::Calibration)
        ));

        let mut radio = driven(vec![status(0x22)]);
        assert!(matches!(
            radio.calibrate(2_400_000_000),
            Err(ListenerError::UnsupportedBand {
                hertz: 2_400_000_000
            })
        ));
    }

    #[test]
    fn a_carrier_check_listens_then_waits_out_its_scan() {
        // `sx1261_set_rx_params` for 922.1 MHz at 125 kHz, `sx1261_lbt_start` for 5 ms at
        // -80 dBm, then `sx1261_lbt_stop`.
        let mut radio = driven(vec![
            SpiStep::write([0x0d, 0x08, 0x9b, 0x00]),
            SpiStep::write([0xc1]),
            SpiStep::write([0x86, 0x39, 0xa1, 0x99, 0x99]),
            SpiStep::write([0x0d, 0x08, 0x9b, 0x14]),
            SpiStep::write([0x8a, 0x00]),
            SpiStep::write([0x8b, 0x00, 0x14, 0x00, 0x00, 0x0a, 0x02, 0xe9, 0x0f]),
            SpiStep::write([0x8c, 0x00, 0x20, 0x05, 0x20, 0x00, 0x01, 0xff, 0x00, 0x00]),
            SpiStep::write([0x82, 0xff, 0xff, 0xff]),
            SpiStep::write([0x9a, 11, 0x02, 0xcb, 160, 1]),
            SpiStep::write([0x0d, 0x08, 0x9b, 0x00]),
            SpiStep::write([0xc1]),
        ]);
        radio
            .listen(922_100_000, Bandwidth::Khz125)
            .expect("the receiver is pointed at the channel");
        let before = radio.delay.total_micros();
        radio.check(ScanTime::Long, -80).expect("the check starts");
        assert_eq!(
            radio.delay.total_micros() - before,
            u64::from(COMMAND_WAIT_US) + 5_000,
            "the command's wait, then the whole scan before a transmission is armed"
        );
        radio.stop().expect("the check ends");
        assert!(radio.spi.done());
    }

    #[test]
    fn a_spectral_scan_reports_its_state_and_counts() {
        let mut counts = vec![0x00; 4];
        for level in 0..33u16 {
            counts.extend((level * 10).to_be_bytes());
        }
        let mut read = vec![0x1d, 0x04, 0x01, 0x00];
        read.extend([0u8; 66]);
        let mut status_read = vec![0x00; 3];
        status_read.push(0xff);

        let mut radio = driven(vec![
            SpiStep::transfer([0x1d, 0x07, 0xcd, 0x00, 0x00], {
                let mut reply = vec![0x00];
                reply.extend(&status_read);
                reply
            }),
            SpiStep::transfer(read, counts),
            SpiStep::transfer(
                [0x1d, 0x07, 0xcd, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x42],
            ),
        ]);
        assert_eq!(radio.scan_status().expect("answers"), ScanStatus::Completed);
        let levels = radio.scan_counts(-4).expect("answers");
        assert_eq!(levels[0], Level { dbm: -4, count: 0 });
        assert_eq!(levels[1], Level { dbm: -8, count: 10 });
        assert_eq!(
            levels[32],
            Level {
                dbm: -128,
                count: 320
            }
        );
        assert!(matches!(
            radio.scan_status(),
            Err(ListenerError::ScanStatus { byte: 0x42 })
        ));
    }
}
