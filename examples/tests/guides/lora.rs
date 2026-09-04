//! The LoRa airtime guide example; see docs/guides/lora.md.

/// What one long-range reading costs on the European band, checked against the published
/// time on air and the duty cycle the sub-band carrying it imposes, so both are pinned to
/// the plan rather than round-tripped against themselves.
#[test]
fn what_one_reading_costs_on_a_european_band() {
    // ANCHOR: example
    use pamoja_lora::region::Region;

    // EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
    // setting that reaches furthest and holds the channel longest.
    let plan = Region::Eu868.plan();
    assert_eq!(plan.name, "EU863-870");
    let link = plan.link_settings(0).expect("DR0 is a LoRa data rate");
    assert_eq!(link.spreading_factor(), 12);

    // The published time on air for SF12 at 125 kHz, coding rate 4/5, an eight-symbol
    // preamble, an explicit header and CRC on, carrying ten bytes.
    let airtime = link.airtime_us(10);
    assert_eq!(airtime, 991_232);

    // 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
    // transmission buys ninety-nine times its own length in silence.
    let permille = plan
        .duty_cycle_permille(868_100_000)
        .expect("868.1 MHz is inside a limited sub-band");
    assert_eq!(permille, 10);
    assert_eq!(plan.max_eirp_dbm(868_100_000), 16);
    let off_time = link.min_off_time_us(10, permille);
    assert_eq!(off_time, airtime * 99);

    // The airtime plus that silence is what one reading really costs, which is the
    // budget a deployment plans against: at SF12, thirty-six readings an hour.
    assert_eq!(3_600_000_000 / (airtime + off_time), 36);

    // A frequency in no sub-band the plan describes has no duty cycle to budget
    // against. That is a limit published elsewhere, not permission to transmit.
    assert_eq!(plan.duty_cycle_permille(700_000_000), None);
    // ANCHOR_END: example
}
