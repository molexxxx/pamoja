//! A PCA9685 that is not there.
//!
//! [`part`] holds the power-on registers of the datasheet's Table 4 and keeps the rules its
//! register descriptions set out, so a driver's mistakes show up in what the part holds:
//!
//! - a write to PRE_SCALE is blocked while MODE1's SLEEP bit is clear, and never loads less
//!   than [`PRE_SCALE_MIN`];
//! - the register pointer moves on only while MODE1's auto-increment bit is set, wrapping
//!   from LED15_OFF_H and from PRE_SCALE to MODE1;
//! - a write to the ALL_LED registers loads that byte into every channel, and the ALL_LED
//!   registers read back as zero;
//! - writing a logic 1 to RESTART clears it, sleeping with a channel still running sets it,
//!   and loading a channel clears it again; EXTCLK, once set, stays set;
//! - reserved bits read zero, and reserved registers take no writes.
//!
//! The general-call software reset goes to address 0x00, which no simulated part holds.

use pamoja_hal::sim::{I2cPart, Rules, REGISTERS};

use super::{
    mode1, mode2, register, ALLCALL_ADDR_RESET, CHANNELS, LED_OFF_H_RESET, MODE1_RESET,
    MODE2_RESET, PRE_SCALE_MIN, PRE_SCALE_RESET, SUBADR1_RESET, SUBADR2_RESET, SUBADR3_RESET,
};

/// The last channel register, LED15_OFF_H.
const LAST_CHANNEL_REGISTER: u8 = register::LED0_ON_L + 4 * CHANNELS - 1;

/// The PCA9685's rules for writes, reads, and its register pointer.
pub const RULES: Rules = Rules { write, read, next };

/// A PCA9685 at an address, as it powers up: asleep at 200 Hz with every output off.
///
/// # Arguments
///
/// * `address` - the address its A5 to A0 pins select;
///   [`DEFAULT_I2C_ADDRESS`](super::DEFAULT_I2C_ADDRESS) with all six low.
///
/// # Returns
///
/// The part, keeping [`RULES`].
///
/// # Examples
///
/// ```
/// use pamoja_actuators::pca9685::{register, sim, Pwm, DEFAULT_I2C_ADDRESS};
/// use pamoja_hal::i2c::I2c;
///
/// let mut part = sim::part(DEFAULT_I2C_ADDRESS);
/// assert_eq!(part.register(register::PRE_SCALE), 0x1E, "200 Hz");
///
/// // Awake, the part keeps its prescale whatever is written to it.
/// part.write(DEFAULT_I2C_ADDRESS, &[register::MODE1, 0x00]).unwrap();
/// part.write(DEFAULT_I2C_ADDRESS, &[register::PRE_SCALE, 121]).unwrap();
/// assert_eq!(part.register(register::PRE_SCALE), 0x1E);
///
/// // One write to the ALL_LED registers loads every channel.
/// let mut all = [register::ALL_LED_ON_L, 0, 0, 0, 0];
/// all[1..].copy_from_slice(&Pwm::duty(2048).bytes());
/// part.write(DEFAULT_I2C_ADDRESS, &[register::MODE1, 0x20]).unwrap();
/// part.write(DEFAULT_I2C_ADDRESS, &all).unwrap();
/// assert_eq!(part.register(0x09), 0x08, "LED0_OFF_H");
/// assert_eq!(part.register(0x45), 0x08, "LED15_OFF_H");
/// ```
#[must_use]
pub fn part(address: u8) -> I2cPart {
    let mut part = I2cPart::new(address)
        .holding(register::MODE1, &[MODE1_RESET, MODE2_RESET])
        .holding(
            register::SUBADR1,
            &[
                SUBADR1_RESET,
                SUBADR2_RESET,
                SUBADR3_RESET,
                ALLCALL_ADDR_RESET,
            ],
        )
        .holding(register::PRE_SCALE, &[PRE_SCALE_RESET]);
    for channel in 0..CHANNELS {
        part.load(register::LED0_ON_L + 4 * channel + 3, &[LED_OFF_H_RESET]);
    }
    part.following(RULES)
}

fn write(registers: &mut [u8; REGISTERS], register: u8, value: u8) {
    match register {
        register::MODE1 => write_mode1(registers, value),
        register::MODE2 => registers[register::MODE2 as usize] = value & 0x1F,
        register::SUBADR1..=register::ALLCALL_ADDR => registers[register as usize] = value & !1,
        register::LED0_ON_L..=LAST_CHANNEL_REGISTER => {
            registers[register as usize] = channel_byte(register - register::LED0_ON_L, value);
            loaded(registers, register - register::LED0_ON_L);
        }
        register::ALL_LED_ON_L..=register::ALL_LED_OFF_H => {
            let offset = register - register::ALL_LED_ON_L;
            for channel in 0..CHANNELS {
                let at = register::LED0_ON_L + 4 * channel + offset;
                registers[at as usize] = channel_byte(offset, value);
            }
            loaded(registers, offset);
        }
        register::PRE_SCALE if registers[register::MODE1 as usize] & mode1::SLEEP != 0 => {
            registers[register::PRE_SCALE as usize] = value.max(PRE_SCALE_MIN);
        }
        _ => {}
    }
}

// MODE1: RESTART clears on a written 1 and is set by sleeping with a channel running;
// EXTCLK is sticky until a power cycle or a software reset.
fn write_mode1(registers: &mut [u8; REGISTERS], value: u8) {
    let held = registers[register::MODE1 as usize];
    let falling_asleep = held & mode1::SLEEP == 0 && value & mode1::SLEEP != 0;
    let restart = if value & mode1::RESTART != 0 {
        0
    } else if falling_asleep && running(registers) {
        mode1::RESTART
    } else {
        held & mode1::RESTART
    };
    let extclk = (held | value) & mode1::EXTCLK;
    registers[register::MODE1 as usize] =
        (value & !(mode1::RESTART | mode1::EXTCLK)) | restart | extclk;
}

// The high bytes of a channel's counts keep bits 7 to 5 reserved and reading zero.
fn channel_byte(offset: u8, value: u8) -> u8 {
    if offset % 2 == 1 {
        value & 0x1F
    } else {
        value
    }
}

// Loading a channel clears RESTART: at any register when outputs change on STOP, and at the
// last of the four when they change on the acknowledge.
fn loaded(registers: &mut [u8; REGISTERS], offset: u8) {
    let on_ack = registers[register::MODE2 as usize] & mode2::OCH != 0;
    if !on_ack || offset % 4 == 3 {
        registers[register::MODE1 as usize] &= !mode1::RESTART;
    }
}

fn running(registers: &[u8; REGISTERS]) -> bool {
    (0..CHANNELS).any(|channel| {
        let off_high = register::LED0_ON_L + 4 * channel + 3;
        registers[off_high as usize] & 0x10 == 0
    })
}

fn read(registers: &[u8; REGISTERS], register: u8) -> u8 {
    match register {
        register::ALL_LED_ON_L..=register::ALL_LED_OFF_H => 0,
        _ => registers[register as usize],
    }
}

fn next(registers: &[u8; REGISTERS], register: u8) -> u8 {
    if registers[register::MODE1 as usize] & mode1::AUTO_INCREMENT == 0 {
        return register;
    }
    match register {
        LAST_CHANNEL_REGISTER | register::PRE_SCALE => register::MODE1,
        _ => register.wrapping_add(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pca9685::{channel_register, Pca9685, Pwm, DEFAULT_I2C_ADDRESS};
    use pamoja_hal::i2c::I2c;
    use pamoja_hal::script::DelayLog;

    const PART: u8 = DEFAULT_I2C_ADDRESS;

    #[test]
    fn it_powers_up_with_the_datasheet_register_values() {
        let part = part(PART);
        assert_eq!(
            part.register(register::MODE1),
            0x11,
            "Table 5: sleeping, All Call on"
        );
        assert_eq!(part.register(register::MODE2), 0x04, "Table 6: totem pole");
        assert_eq!(part.register(register::SUBADR1), 0xE2, "Table 9");
        assert_eq!(part.register(register::SUBADR2), 0xE4);
        assert_eq!(part.register(register::SUBADR3), 0xE8);
        assert_eq!(part.register(register::ALLCALL_ADDR), 0xE0, "Table 10");
        assert_eq!(part.register(register::PRE_SCALE), 0x1E, "Table 8: 200 Hz");
        for channel in 0..CHANNELS {
            let first = channel_register(channel);
            let bytes = [
                part.register(first),
                part.register(first + 1),
                part.register(first + 2),
                part.register(first + 3),
            ];
            assert_eq!(
                bytes,
                [0, 0, 0, 0x10],
                "Table 7: channel {channel} full off"
            );
            assert_eq!(Pwm::from_bytes(&bytes), Pwm::full_off());
        }
    }

    #[test]
    fn prescale_takes_a_write_only_while_asleep_and_never_below_three() {
        let mut part = part(PART);
        part.write(PART, &[register::PRE_SCALE, 121]).unwrap();
        assert_eq!(
            part.register(register::PRE_SCALE),
            121,
            "asleep at power-up"
        );
        part.write(PART, &[register::PRE_SCALE, 1]).unwrap();
        assert_eq!(
            part.register(register::PRE_SCALE),
            3,
            "the hardware minimum"
        );
        part.write(PART, &[register::MODE1, 0x00]).unwrap();
        part.write(PART, &[register::PRE_SCALE, 121]).unwrap();
        assert_eq!(part.register(register::PRE_SCALE), 3, "blocked while awake");
    }

    #[test]
    fn the_pointer_moves_on_only_with_auto_increment() {
        let mut part = part(PART);
        let first = channel_register(0);
        part.write(PART, &[first, 1, 2, 3, 4]).unwrap();
        assert_eq!(part.register(first), 4, "every byte lands on one register");
        assert_eq!(part.register(first + 1), 0);

        part.write(PART, &[register::MODE1, mode1::AUTO_INCREMENT])
            .unwrap();
        part.write(PART, &[first, 1, 2, 3, 4]).unwrap();
        assert_eq!(
            [
                part.register(first),
                part.register(first + 1),
                part.register(first + 2),
                part.register(first + 3)
            ],
            [1, 2, 3, 4]
        );

        part.write(PART, &[LAST_CHANNEL_REGISTER, 0x10, mode1::AUTO_INCREMENT])
            .unwrap();
        assert_eq!(
            part.register(register::MODE1),
            mode1::AUTO_INCREMENT,
            "wrapped to MODE1"
        );
        let mut around = [0u8; 2];
        part.write_read(PART, &[register::PRE_SCALE], &mut around)
            .unwrap();
        assert_eq!(
            around,
            [PRE_SCALE_RESET, mode1::AUTO_INCREMENT],
            "PRE_SCALE rolls over to MODE1"
        );
    }

    #[test]
    fn all_led_loads_every_channel_and_reads_zero() {
        let mut part = part(PART);
        part.write(PART, &[register::MODE1, mode1::AUTO_INCREMENT])
            .unwrap();
        let [on_l, on_h, off_l, off_h] = Pwm::from_counts(410, 3686).bytes();
        part.write(PART, &[register::ALL_LED_ON_L, on_l, on_h, off_l, off_h])
            .unwrap();
        for channel in 0..CHANNELS {
            let first = channel_register(channel);
            let bytes = [
                part.register(first),
                part.register(first + 1),
                part.register(first + 2),
                part.register(first + 3),
            ];
            assert_eq!(Pwm::from_bytes(&bytes), Pwm::from_counts(410, 3686));
        }
        let mut all = [0xFFu8; 4];
        part.write_read(PART, &[register::ALL_LED_ON_L], &mut all)
            .unwrap();
        assert_eq!(all, [0; 4], "Table 4: the ALL_LED registers read zero");
    }

    #[test]
    fn restart_follows_section_7_3_1_1() {
        let mut part = part(PART);
        let awake = mode1::AUTO_INCREMENT;
        part.write(PART, &[register::MODE1, awake]).unwrap();
        part.write(PART, &[channel_register(3), 0, 0, 0x00, 0x08])
            .unwrap();

        part.write(PART, &[register::MODE1, awake | mode1::SLEEP])
            .unwrap();
        assert_ne!(
            part.register(register::MODE1) & mode1::RESTART,
            0,
            "asleep with a channel running"
        );

        part.write(PART, &[register::MODE1, awake]).unwrap();
        assert_ne!(
            part.register(register::MODE1) & mode1::RESTART,
            0,
            "a written 0 has no effect"
        );
        part.write(PART, &[register::MODE1, awake | mode1::RESTART])
            .unwrap();
        assert_eq!(
            part.register(register::MODE1) & mode1::RESTART,
            0,
            "a written 1 clears it"
        );

        part.write(PART, &[register::MODE1, awake | mode1::SLEEP])
            .unwrap();
        part.write(PART, &[channel_register(5), 0, 0, 0, 0x08])
            .unwrap();
        assert_eq!(
            part.register(register::MODE1) & mode1::RESTART,
            0,
            "loading a channel clears it"
        );

        let mut stopped = super::part(PART);
        stopped.write(PART, &[register::MODE1, 0x00]).unwrap();
        stopped
            .write(PART, &[register::MODE1, mode1::SLEEP])
            .unwrap();
        assert_eq!(
            stopped.register(register::MODE1) & mode1::RESTART,
            0,
            "every channel off"
        );
    }

    #[test]
    fn extclk_stays_set_and_reserved_bits_read_zero() {
        let mut part = part(PART);
        part.write(PART, &[register::MODE1, mode1::SLEEP | mode1::EXTCLK])
            .unwrap();
        part.write(PART, &[register::MODE1, mode1::SLEEP]).unwrap();
        assert_ne!(part.register(register::MODE1) & mode1::EXTCLK, 0);
        part.write(PART, &[register::MODE2, 0xFF]).unwrap();
        assert_eq!(part.register(register::MODE2), 0x1F);
        part.write(PART, &[register::SUBADR1, 0xFF]).unwrap();
        assert_eq!(
            part.register(register::SUBADR1),
            0xFE,
            "the LSB is read-only 0"
        );
        part.write(PART, &[0x50, 0xAA]).unwrap();
        assert_eq!(part.register(0x50), 0, "a reserved register takes nothing");
    }

    #[test]
    fn the_driver_leaves_the_part_as_its_datasheet_asks() {
        let mut pwm = Pca9685::new(part(PART), PART, DelayLog::new()).with_frequency(50);
        pwm.set_channel(0, Pwm::servo(1500, 50)).unwrap();
        pwm.set_channel(15, Pwm::duty(1024)).unwrap();
        let (part, delay) = pwm.release();
        assert_eq!(
            part.register(register::PRE_SCALE),
            121,
            "round(25 MHz / 4096 / 50) - 1"
        );
        assert_eq!(
            part.register(register::MODE1),
            mode1::AUTO_INCREMENT,
            "awake, RESTART cleared"
        );
        assert_eq!(part.register(register::MODE2), MODE2_RESET);
        let first = channel_register(0);
        let servo = [
            part.register(first),
            part.register(first + 1),
            part.register(first + 2),
            part.register(first + 3),
        ];
        assert_eq!(Pwm::from_bytes(&servo), Pwm::servo(1500, 50));
        assert_eq!(delay.total_micros(), 500, "the oscillator's start-up");
    }

    #[test]
    fn a_channel_reads_back_whatever_mode1_holds() {
        let mut fresh = Pca9685::new(part(PART), PART, DelayLog::new());
        assert_eq!(
            fresh.channel(7).unwrap(),
            Pwm::full_off(),
            "every channel powers up full off, and auto-increment is off until init"
        );
        let (part, _) = fresh.release();
        assert_eq!(
            part.register(register::MODE1),
            MODE1_RESET,
            "a read changes nothing"
        );

        let mut pwm = Pca9685::new(part, PART, DelayLog::new()).with_frequency(50);
        pwm.set_channel(7, Pwm::duty(1024)).unwrap();
        assert_eq!(pwm.channel(7).unwrap(), Pwm::duty(1024));
    }

    #[test]
    fn a_driver_that_writes_the_prescale_awake_leaves_the_part_at_200_hz() {
        let mut part = part(PART);
        part.write(PART, &[register::MODE1, mode1::AUTO_INCREMENT])
            .unwrap();
        part.write(PART, &[register::PRE_SCALE, 121]).unwrap();
        assert_eq!(part.register(register::PRE_SCALE), PRE_SCALE_RESET);
    }
}
