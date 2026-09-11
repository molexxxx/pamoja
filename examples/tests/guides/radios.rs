//! The LoRa radio guide example; see docs/guides/radios.md.

/// A reading planned for an SX1262: the power a regional ceiling allows behind a whip, the
/// bytes of each command, the chip's answers decoded, and the silence the duty cycle owes.
#[test]
fn a_reading_planned_for_an_sx1262() {
    // ANCHOR: example
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use pamoja_lora::region::Region;
    use pamoja_radios::duty::DutyCycle;
    use pamoja_radios::sx126x::command;
    use pamoja_radios::sx126x::config::{
        self, LoraModulation, LoraPacket, PacketType, PowerAmplifier, RampTime, StandbyMode,
        TxPower,
    };
    use pamoja_radios::sx126x::irq::Irq;
    use pamoja_radios::sx126x::status::{PacketStatus, Status};

    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

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

    // The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
    // transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
    let airtime = link.airtime_us(10);
    let events = Irq::TX_DONE | Irq::TIMEOUT;
    let modulation = LoraModulation::from_link(&link).expect("125 kHz is an SX126x bandwidth");
    let commands = [
        ("standby", command::set_standby(StandbyMode::Rc)),
        ("packet type", command::set_packet_type(PacketType::Lora)),
        (
            "frequency",
            command::set_rf_frequency(config::frequency_word(frequency)),
        ),
        ("pa config", command::set_pa_config(power.pa)),
        (
            "tx params",
            command::set_tx_params(power.setting_dbm, RampTime::at_least(40)),
        ),
        (
            "modulation",
            command::set_lora_modulation_params(modulation),
        ),
        (
            "packet",
            command::set_lora_packet_params(LoraPacket::from_link(&link, 10, false)),
        ),
        (
            "irq",
            command::set_dio_irq_params(events, events, Irq::NONE, Irq::NONE),
        ),
        (
            "tx",
            command::set_tx(config::timeout_steps(airtime + 1_000_000)),
        ),
    ];
    for (name, bytes) in &commands {
        println!("{name:<12}{}", hex(bytes.as_bytes()));
    }

    // Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
    // chip back in standby.
    let irq = Irq::from_bytes([0x00, 0x01]);
    let sent = irq.contains(Irq::TX_DONE);
    let timed_out = irq.contains(Irq::TIMEOUT);
    println!("sent      tx done {sent}, timed out {timed_out}");
    let status = Status::from_byte(0x2C);
    println!(
        "status    {:?}, {:?}",
        status.chip_mode, status.command_status
    );

    // A frame that arrives later comes with the signal levels it was heard at.
    let heard = PacketStatus::from_bytes([0xDB, 0xF6, 0xE0]);
    let db = |value: Decibels| f64::from(value.hundredths()) / 100.0;
    println!(
        "received  RSSI {} dBm, SNR {} dB",
        db(heard.rssi_dbm),
        db(heard.snr_db)
    );

    // The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
    // ninety-nine times as long in silence before the next.
    let permille = eu868
        .duty_cycle_permille(frequency)
        .expect("868.1 MHz is in a sub-band");
    let mut guard = DutyCycle::new(permille);
    let held = guard.transmitted(0, &link, 10);
    println!(
        "airtime   {held} us, next frame after {} us",
        guard.wait_us(0)
    );
    // ANCHOR_END: example

    // The bytes each command carries are pinned once, in the crate tests and the generated
    // conformance vectors, so a guide asserts behavior instead.
    assert_eq!(power.setting_dbm, 14);
    assert_eq!(commands.len(), 9);
    assert!(sent && !timed_out);
    assert_eq!(held, airtime);
    assert!(!guard.ready(0));
    assert!(guard.ready(held * 100));
}

/// The same reading from an RFM95W, whose SX1276 is driven through registers: the amplifier
/// setting on PA_BOOST, the carrier and modem registers, the transmit mode, a received packet
/// decoded, and whether an LLCC68 could carry the same data rates.
#[test]
fn the_same_reading_from_an_rfm95w() {
    // ANCHOR: rfm95w
    use pamoja_lora::budget::{Decibels, LinkBudget};
    use pamoja_lora::region::Region;
    use pamoja_radios::sx126x::config::{llcc68_supports, LoraModulation as Sx126xModulation};
    use pamoja_radios::sx127x::config::{frequency_word, LoraModulation, PaOutput, TxPower};
    use pamoja_radios::sx127x::irq::IrqFlags;
    use pamoja_radios::sx127x::register::{lora_op_mode, Mode};
    use pamoja_radios::sx127x::status::{PacketStatus, Port};

    // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
    // same 16 dBm ceiling leave it the same 14 dBm, set through three registers.
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
    println!(
        "rfm95w    {} dBm on PA_BOOST: RegPaConfig {:02x}, RegPaDac {:02x}, RegOcp {:02x}",
        rfm95w.output_dbm, rfm95w.pa_config, rfm95w.pa_dac, rfm95w.ocp
    );

    // The carrier and the modem go into registers while the chip stands by, and TX mode sends
    // the frame the FIFO holds.
    let modem = LoraModulation::from_link(&dr3).expect("DR3 fits an SX1276");
    println!("carrier   RegFrf {:06x}", frequency_word(channel));
    println!(
        "modem     RegModemConfig {:02x} {:02x} {:02x}",
        modem.modem_config_1(),
        modem.modem_config_2(0),
        modem.modem_config_3()
    );
    println!("tx mode   RegOpMode {:02x}", lora_op_mode(Mode::Tx));

    // A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give
    // its levels on the high frequency port.
    let flags = IrqFlags::from_bits(0x50);
    let received = flags.contains(IrqFlags::RX_DONE);
    let corrupt = flags.contains(IrqFlags::PAYLOAD_CRC_ERROR);
    println!("irq       rx done {received}, crc error {corrupt}");
    let packet = PacketStatus::from_bytes([0xF6, 0x30], Port::for_frequency(channel));
    let db = |value: Decibels| f64::from(value.hundredths()) / 100.0;
    println!(
        "received  RSSI {} dBm, SNR {} dB, signal {} dBm",
        db(packet.rssi_dbm),
        db(packet.snr_db),
        db(packet.signal_rssi_dbm)
    );

    // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
    let fits = |data_rate: u8| {
        band.link_settings(data_rate)
            .and_then(|link| Sx126xModulation::from_link(&link))
            .is_some_and(|modulation| {
                llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
            })
    };
    println!("llcc68    DR3 {}, DR2 {}", fits(3), fits(2));
    // ANCHOR_END: rfm95w

    assert_eq!(rfm95w.output_dbm, 14);
    assert_eq!(rfm95w.pa_config, 0xFC);
    assert_eq!(modem.modem_config_2(0), 0x94);
    assert!(received && !corrupt);
    assert!(fits(3) && !fits(2));
}
