//! The LoRa radio guide example; see docs/guides/radios.md.
//!
//! Run: `cargo run -p pamoja-examples --example radios`

use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    a_reading_from_an_sx1262_on_the_bench()?;
    the_same_reading_from_an_rfm95w()?;
    a_radio_on_a_linux_board()?;
    Ok(())
}

/// A reading sent from a simulated SX1262 through the same driver a real one runs: the power
/// a regional ceiling allows behind a whip, what the chip was tuned to, the silence the duty
/// cycle owes, and what the chip hands over when a frame arrives and when none does.
fn a_reading_from_an_sx1262_on_the_bench() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use pamoja_lora::region::Region;
    use pamoja_radios::duty::DutyCycle;
    use pamoja_radios::radio::{RadioConfig, Reception};
    use pamoja_radios::sim::Chip;
    use pamoja_radios::sx126x::config::{PowerAmplifier, TxPower};
    use pamoja_radios::sx126x::Board;

    // An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
    // 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
    // and pigtail decide how hard the amplifier may drive under that cap.
    let eu868 = Region::Eu868.plan();
    let frequency = 868_100_000;
    let link = eu868.link_settings(3).expect("DR3 is a LoRa data rate");
    let whip = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };
    let ceiling = eu868.max_eirp_dbm(frequency);
    let power = TxPower::under_ceiling(
        PowerAmplifier::HighPower,
        &whip,
        Decibels::from_db(ceiling.into()),
    );
    println!(
        "power     {} dBm under a {ceiling} dBm EIRP ceiling",
        power.setting_dbm
    );

    // A simulated SX1262 stands in for the chip on the node's board, driven by the same code
    // that drives a real one, and it reports what that code told it.
    let chip = Chip::sx126x(Board::new(PowerAmplifier::HighPower));
    let mut radio = chip.radio();
    radio.init()?;
    radio.configure(RadioConfig::new(frequency, link, power.setting_dbm))?;
    let tuned = chip.tuning();
    println!(
        "tuned     {:.1} MHz, SF{} at {} kHz, {} dBm",
        f64::from(tuned.frequency_hz) / 1e6,
        tuned.link.spreading_factor(),
        tuned.link.bandwidth_hz() / 1000,
        tuned.output_dbm
    );

    // The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band
    // that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long
    // in silence before the next.
    let reading = b"level=0.42";
    let airtime = radio.transmit(reading)?;
    println!(
        "sent      {} bytes, {airtime} us on air",
        chip.sent()[0].payload.len()
    );
    let permille = eu868
        .duty_cycle_permille(frequency)
        .expect("868.1 MHz is in a sub-band");
    let mut guard = DutyCycle::new(permille);
    guard.transmitted(0, &link, reading.len());
    println!(
        "silence   the next frame starts {} us after this one did",
        guard.wait_us(0)
    );

    // A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
    chip.hear(b"ack", Decibels::from_db(-109), Decibels::from_tenths(-25));
    let mut buffer = [0u8; 255];
    if let Reception::Frame { len, levels } = radio.receive(&mut buffer, 1_000_000)? {
        println!(
            "received  {} at {} dBm, SNR {} dB",
            String::from_utf8_lossy(&buffer[..len]),
            levels.rssi_dbm,
            levels.snr_db
        );
    }

    // With nothing on the air the reception times out, and a frame whose CRC fails is dropped
    // rather than handed over.
    let quiet = radio.receive(&mut buffer, 1_000_000)?;
    chip.hear_corrupt(Decibels::from_db(-121), Decibels::from_db(-12));
    let broken = radio.receive(&mut buffer, 1_000_000)?;
    println!("then      {quiet:?}, then {broken:?}");
    // ANCHOR_END: example

    assert_eq!(power.setting_dbm, 14);
    assert_eq!(tuned.frequency_hz, frequency);
    assert_eq!(airtime, link.airtime_us(reading.len()));
    assert_eq!(chip.sent()[0].payload, reading);
    assert_eq!(guard.wait_us(0), airtime * 100);
    assert_eq!(quiet, Reception::Timeout);
    assert_eq!(broken, Reception::Corrupt);

    Ok(())
}

/// The same reading from a simulated RFM95W, whose SX1276 takes registers where the SX1262
/// takes commands: the power on PA_BOOST, the carrier its synthesizer can reach, a packet's
/// levels, and whether an LLCC68 could carry the same data rates.
fn the_same_reading_from_an_rfm95w() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: rfm95w
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use pamoja_lora::region::Region;
    use pamoja_radios::radio::{RadioConfig, Reception};
    use pamoja_radios::sim::Chip;
    use pamoja_radios::sx126x::config::{llcc68_supports, LoraModulation as Sx126xModulation};
    use pamoja_radios::sx127x::config::{PaOutput, TxPower};
    use pamoja_radios::sx127x::Board;

    // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
    // same 16 dBm ceiling leave it the same 14 dBm.
    let band = Region::Eu868.plan();
    let channel = 868_100_000;
    let dr3 = band.link_settings(3).expect("DR3 is a LoRa data rate");
    let antenna = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };
    let limit = Decibels::from_db(band.max_eirp_dbm(channel).into());
    let rfm95w = TxPower::under_ceiling(PaOutput::PaBoost, &antenna, limit);

    // The same driver calls tune it, through registers this time. Its synthesizer steps in
    // 61 Hz, so the carrier lands on the step nearest the one asked for.
    let chip = Chip::sx127x(Board::new(PaOutput::PaBoost));
    let mut radio = chip.radio();
    radio.init()?;
    radio.configure(RadioConfig::new(channel, dr3, rfm95w.output_dbm))?;
    let tuned = chip.tuning();
    println!(
        "rfm95w    {} dBm on PA_BOOST, carrier {} Hz, {} Hz from {channel}",
        tuned.output_dbm,
        tuned.frequency_hz,
        channel.abs_diff(tuned.frequency_hz)
    );

    // The SX1276 gives a packet's strength in whole decibels, and works out the strength of
    // the signal itself from the SNR when it arrived under the noise.
    chip.hear(b"ack", Decibels::from_db(-109), Decibels::from_tenths(-25));
    let mut buffer = [0u8; 255];
    if let Reception::Frame { levels, .. } = radio.receive(&mut buffer, 1_000_000)? {
        println!(
            "received  RSSI {} dBm, SNR {} dB, signal {} dBm",
            levels.rssi_dbm, levels.snr_db, levels.signal_rssi_dbm
        );
    }

    // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
    let carries = |data_rate: u8| {
        let fits = band
            .link_settings(data_rate)
            .and_then(|link| Sx126xModulation::from_link(&link))
            .is_some_and(|modulation| {
                llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
            });
        if fits {
            "carries"
        } else {
            "cannot carry"
        }
    };
    println!("llcc68    {} DR3 and {} DR2", carries(3), carries(2));
    // ANCHOR_END: rfm95w

    assert_eq!(rfm95w.output_dbm, 14);
    assert_eq!(tuned.output_dbm, 14);
    assert!(channel.abs_diff(tuned.frequency_hz) <= 61);
    assert_eq!(carries(3), "carries");

    Ok(())
}

/// The same radio opened on a Linux board, over the kernel's spidev and GPIO character
/// devices, which says what it found where no radio is wired.
fn a_radio_on_a_linux_board() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: hardware
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use pamoja_lora::region::Region;
    use pamoja_radios::linux::{self, Wiring};
    use pamoja_radios::radio::RadioConfig;
    use pamoja_radios::sx127x::config::{PaOutput, TxPower};
    use pamoja_radios::sx127x::Board;

    // An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin
    // on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
    let wiring = Wiring::new("/dev/spidev0.0", "/dev/gpiochip0", 25);
    println!(
        "radio     an RFM95W on {}, reset on GPIO{}",
        wiring.spi.display(),
        wiring.reset_line
    );

    // The channel and the power the same whip leaves under the same ceiling, now as the number
    // the radio is set to rather than the registers it goes into.
    let band = Region::Eu868.plan();
    let channel = 868_100_000;
    let dr3 = band.link_settings(3).expect("DR3 is a LoRa data rate");
    let antenna = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };
    let ceiling = Decibels::from_db(i32::from(band.max_eirp_dbm(channel)));
    let rfm95w = TxPower::under_ceiling(PaOutput::PaBoost, &antenna, ceiling);
    println!(
        "plan      {channel} Hz at DR3, {} dBm on PA_BOOST",
        rfm95w.output_dbm
    );

    // Opening resets the chip and reads its version back, so a wiring mistake is caught here
    // rather than on the first frame. With no radio wired, this is the line that prints.
    match linux::open_sx127x(&wiring, Board::new(PaOutput::PaBoost)) {
        Ok(mut radio) => {
            radio
                .configure(RadioConfig::new(channel, dr3, rfm95w.output_dbm))
                .expect("the chip takes the settings");
            let airtime_us = radio.transmit(b"21.5").expect("the frame goes out");
            println!("sent      a reading in {airtime_us} us on air");
        }
        Err(_) => println!("absent    no radio answered, so nothing went out"),
    }
    // ANCHOR_END: hardware

    assert_eq!(rfm95w.output_dbm, 14);

    Ok(())
}
