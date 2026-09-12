//! The LoRa airtime guide example; see docs/guides/lora.md.
//!
//! Run: `cargo run -p pamoja-examples --example lora`

use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    what_one_reading_costs_on_a_european_band()?;
    how_far_a_reading_reaches()?;
    Ok(())
}

/// What one reading really costs on a European LoRa deployment: the time it holds the
/// channel, the silence the regulator then requires, and how many readings an hour that
/// leaves room for.
fn what_one_reading_costs_on_a_european_band() -> std::result::Result<(), Box<dyn Error>> {
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
    println!("budget    {per_hour} readings an hour");

    // A frequency in no sub-band the plan describes has no duty cycle to budget against.
    // That is a limit published elsewhere, not permission to transmit.
    match plan.duty_cycle_permille(700_000_000) {
        Some(limit) => println!("700 MHz reported a {limit} per mille limit, which it has none of"),
        None => println!("700 MHz  is outside this plan, so it budgets nothing: true"),
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

    Ok(())
}

/// How far a reading reaches from a European node: the power the plan leaves the radio
/// behind a real antenna, the weakest signal a gateway still hears, and the margin a path
/// leaves at a few distances.
fn how_far_a_reading_reaches() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: range
    use pamoja_lora::budget::{self, Decibels, Fcc15247, LinkBudget, GATEWAY_NOISE_FIGURE_DB};
    use pamoja_lora::region::Region;

    let eu868 = Region::Eu868.plan();
    let dr0 = eu868.link_settings(0).expect("DR0 is a LoRa data rate");
    let frequency = 868_100_000;

    // A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
    // 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
    let whip = LinkBudget {
        transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
        transmit_cable_loss_db: Decibels::from_tenths(5),
        ..LinkBudget::default()
    };

    // The plan caps what leaves the antenna, so the antenna and cable decide how hard the
    // radio may drive. A radio takes whole decibels, so the setting rounds down.
    let ceiling = Decibels::from_db(eu868.max_eirp_dbm(frequency).into());
    let most = whip.max_transmit_power_dbm(ceiling);
    let node = LinkBudget {
        transmit_power_dbm: Decibels::from_db(most.floor_db()),
        receive_antenna_gain_dbi: Decibels::from_db(6),
        receive_cable_loss_db: Decibels::from_tenths(15),
        noise_figure_db: GATEWAY_NOISE_FIGURE_DB,
        ..whip
    };
    println!(
        "radio     {most} dBm allowed, set to {} dBm",
        node.transmit_power_dbm.round_db()
    );
    println!(
        "eirp      {} dBm under a {} dBm ceiling",
        node.eirp_dbm(),
        ceiling.round_db()
    );

    // The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
    // loss the link survives.
    let sensitivity = node.sensitivity_dbm(dr0);
    let survives = node.max_path_loss_db(dr0);
    println!("gateway   hears down to {sensitivity} dBm, so {survives} dB of path loss");

    // Free space at three distances, and what each path leaves to spare.
    let mut margins = Vec::new();
    for distance_m in [2_000, 5_000, 15_000] {
        let loss = budget::free_space_loss_db(distance_m, frequency);
        let margin = node.margin_db(dr0, loss);
        println!(
            "{:>2} km     {loss} dB lost, {margin} dB to spare",
            distance_m / 1_000
        );
        margins.push(margin);
    }

    // Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
    // diffraction loss, which starts once the clearance falls below 60% of its radius.
    let radius = budget::fresnel_radius_mm(2_500, 2_500, frequency);
    println!(
        "fresnel   {:.1} m at the middle of 5 km, keep {:.1} m clear",
        f64::from(radius) / 1000.0,
        f64::from(radius * 6 / 10) / 1000.0
    );

    // In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
    // decibel an antenna has over 6 dBi.
    let limit = Fcc15247::FrequencyHopping { channels: 64 }
        .max_conducted_dbm(Decibels::from_db(9))
        .expect("64 hopping channels have a limit");
    println!("fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit} dBm");
    // ANCHOR_END: range

    assert_eq!(most, Decibels::from_hundredths(1_435));
    assert_eq!(node.transmit_power_dbm, Decibels::from_db(14));
    assert_eq!(node.eirp_dbm(), Decibels::from_hundredths(1_565));
    assert_eq!(sensitivity, Decibels::from_hundredths(-14_003));
    assert_eq!(survives, Decibels::from_hundredths(16_018));
    assert_eq!(
        margins,
        [6_294, 5_498, 4_544].map(Decibels::from_hundredths)
    );
    assert_eq!(radius, 20_777);
    assert_eq!(limit, Decibels::from_db(27));

    Ok(())
}
