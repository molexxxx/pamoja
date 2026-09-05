//! The LoRa airtime guide example; see docs/guides/lora.md.

/// What one reading really costs on a European LoRa deployment: the time it holds the
/// channel, the silence the regulator then requires, and how many readings an hour that
/// leaves room for.
#[test]
fn what_one_reading_costs_on_a_european_band() {
    // ANCHOR: example
    use pamoja_lora::region::Region;

    // EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
    // setting that reaches furthest and holds the channel longest.
    let plan = Region::Eu868.plan();
    let link = plan.link_settings(0).expect("DR0 is a LoRa data rate");
    println!(
        "{} DR0 is SF{} at 125 kHz",
        plan.name,
        link.spreading_factor()
    );

    // The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an
    // explicit header and CRC on, carrying a ten-byte reading.
    let airtime = link.airtime_us(10);
    println!("airtime   {:.2} s for ten bytes", airtime as f64 / 1e6);

    // 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
    // transmission buys ninety-nine times its own length in silence.
    let channel = 868_100_000;
    let permille = plan
        .duty_cycle_permille(channel)
        .expect("868.1 MHz is inside a limited sub-band");
    let power = plan.max_eirp_dbm(channel);
    println!("channel   {permille} per mille duty cycle, {power} dBm");

    let off_time = link.min_off_time_us(10, permille);
    println!(
        "silence   {:.1} s owed after each reading",
        off_time as f64 / 1e6
    );

    // The airtime plus that silence is what one reading really costs, which is the budget
    // a deployment plans against.
    let per_hour = 3_600_000_000 / (airtime + off_time);
    println!("budget    {per_hour} readings an hour at this data rate");

    // A frequency in no sub-band the plan describes has no duty cycle to budget against.
    // That is a limit published elsewhere, not permission to transmit.
    match plan.duty_cycle_permille(700_000_000) {
        Some(limit) => println!("700 MHz reported a {limit} per mille limit, which it has none of"),
        None => println!("700 MHz  is outside this plan, so it budgets nothing"),
    }
    // ANCHOR_END: example

    assert_eq!(plan.name, "EU863-870");
    assert_eq!(link.spreading_factor(), 12);
    assert_eq!(airtime, 991_232);
    assert_eq!(permille, 10);
    assert_eq!(power, 16);
    assert_eq!(off_time, airtime * 99);
    assert_eq!(per_hour, 36);
    assert_eq!(plan.duty_cycle_permille(700_000_000), None);
}
