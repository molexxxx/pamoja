//! A gateway on an SX1302 concentrator.
//!
//! Brings a concentrator up from a configuration file, then forwards what it hears to a
//! network server and transmits what comes back.
//!
//! Run: `pamoja-gateway /etc/pamoja/gateway.json`
//!
//! The bring-up order is not a matter of taste. The front ends are tuned before the clock is
//! taken from one of them, because a front end that is not listening clocks nothing; the
//! receivers are given their channels before anything is switched on; and the two
//! microcontrollers are configured after their firmware is loaded rather than before.

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pamoja_gateway::daemon::{forward, image, walk, Config, Upstream};
use pamoja_gateway::station::{Levels, Message, Station};
use pamoja_gateway::udp::{CrcStatus, Packet as Datagram, Stat, TxStatus, Uplink};
use pamoja_lora::LinkSettings;
use pamoja_radios::linux::{self, LinuxConcentrator, Wiring};
use pamoja_radios::sx1302::channel::{Plan, MULTI_BANDWIDTH_HZ};
use pamoja_radios::sx1302::rx::{self, BUFFER_LEN};
use pamoja_radios::sx1302::timestamp::Counter;
use pamoja_radios::sx1302::tx::{gain_for, start_delay, Chain, FrontEnd, Transmit, Trigger};
use tokio::net::UdpSocket;

/// How long to wait between asking the concentrator what it heard.
const POLL: Duration = Duration::from_millis(10);

/// How often a gateway holds its route open, which the protocol expects every few seconds.
const KEEPALIVE: Duration = Duration::from_secs(5);

/// How often a gateway reports how it is doing.
const REPORT: Duration = Duration::from_secs(30);

/// The filter setting the channels are configured with, which the start delay is worked from.
const CHIRP_LOWPASS: u8 = 6;

/// How close to its window a downlink can arrive and still be programmed, in microseconds.
///
/// The reference forwarder refuses anything inside its start delay, its overlap margin and
/// the delay it programs a queued packet by, which is 42500 us together. This daemon
/// programs the chain as a downlink arrives rather than queueing it, so that last part is
/// margin rather than a wait, and it is kept: configuring a chain is dozens of transfers and
/// a payload burst, and a packet that misses its window is answered as sent while the device
/// hears nothing.
const TOO_LATE_US: u32 = 1_500 + 1_000 + 40_000;

/// How far ahead of now a downlink can be scheduled, in microseconds.
///
/// A class A window is a second or two out and a class B one falls inside 128 seconds, so
/// the reference treats anything past four beacon periods as a timestamp that is wrong
/// rather than early.
const TOO_EARLY_US: u32 = 4 * 128 * 1_000_000;

#[tokio::main]
async fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: pamoja-gateway <configuration.json>");
        return ExitCode::FAILURE;
    };

    match run(Path::new(&path)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("pamoja-gateway: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the configuration, brings the concentrator up, and forwards until something ends it.
async fn run(path: &Path) -> Result<(), String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let config = Config::parse(&text).map_err(|error| error.to_string())?;

    let gain_control = image(Path::new(&config.concentrator.gain_control_firmware))
        .map_err(|error| error.to_string())?;
    let arbiter = image(Path::new(&config.concentrator.arbiter_firmware))
        .map_err(|error| error.to_string())?;

    let mut wiring = Wiring::new(
        &config.concentrator.spi,
        &config.concentrator.gpio_chip,
        config.concentrator.reset_line,
    );
    if let Some(line) = config.concentrator.power_enable_line {
        wiring = wiring.with_power_enable_line(line);
    }

    // The second binding is the supply line on a board that gates its concentrator. It stays
    // bound on purpose: a GPIO line is released when its handle drops, so letting go of this
    // one switches the card off while everything else still looks configured.
    let (mut chip, _supply) = linux::open_sx1302(&wiring).map_err(|error| error.to_string())?;

    let plan = Plan::new(config.radio.carrier_hz, &config.radio.channels)
        .looking_for(&config.radio.spreading_factors)
        .network(config.radio.lorawan_public);

    if let Some(model) =
        walk(&mut chip, &config, &plan, &gain_control, &arbiter).map_err(|why| why.to_string())?
    {
        println!("pamoja-gateway: {model:?} answering");
    }
    println!(
        "pamoja-gateway: concentrator is listening on {} channels",
        config.radio.channels.len()
    );

    match &config.upstream {
        Upstream::Forwarder { host, port } => {
            forwarding(chip, &config, (host.as_str(), *port)).await
        }
        Upstream::Station { endpoint } => stationing(chip, &config, endpoint).await,
    }
}

/// Forwards uplinks to a packet forwarder, and transmits what it sends back.
async fn forwarding(
    mut chip: LinuxConcentrator,
    config: &Config,
    server: (&str, u16),
) -> Result<(), String> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|error| format!("no socket: {error}"))?;
    socket
        .connect(server)
        .await
        .map_err(|error| format!("{}:{}: {error}", server.0, server.1))?;

    let mut counter = Counter::new();
    let mut buffer = [0u8; BUFFER_LEN];
    let mut datagram = [0u8; 2048];
    let mut token = 0u16;
    let mut heard = 0u32;
    let mut checked = 0u32;
    let mut forwarded = 0u32;
    let mut downlinks = 0u32;
    let mut sent = 0u32;

    // When the band is free again, in the concentrator's own microseconds. A duty cycle is a
    // share of time rather than a count of packets, so honoring one means staying quiet
    // after a transmission for as long as its airtime owes.
    let mut band_free_at: Option<u32> = None;

    let mut keepalive = tokio::time::interval(KEEPALIVE);
    let mut report = tokio::time::interval(REPORT);
    let mut poll = tokio::time::interval(POLL);

    loop {
        tokio::select! {
            _ = poll.tick() => {
                let taken = chip
                    .receive(&mut buffer)
                    .map_err(|error| format!("the concentrator stopped answering: {error}"))?;
                if taken.is_empty() {
                    continue;
                }

                let mut packets = Vec::new();
                for packet in rx::packets(taken) {
                    heard += 1;
                    if let Some(entry) = forward::heard(
                        &packet,
                        config.radio.carrier_hz,
                        &config.radio.channels,
                        &counter,
                    ) {
                        checked += u32::from(entry.crc == CrcStatus::Ok);
                        packets.push(entry);
                    }
                }

                if !packets.is_empty() {
                    forwarded += packets.len() as u32;
                    token = token.wrapping_add(1);
                    let push = Datagram::PushData {
                        token,
                        gateway: config.gateway,
                        uplink: Uplink { packets, status: None },
                    };
                    send(&socket, &push).await?;
                }
            }
            _ = keepalive.tick() => {
                token = token.wrapping_add(1);
                send(&socket, &Datagram::PullData { token, gateway: config.gateway }).await?;

                // The counter is read on the same beat, which keeps its rollover count
                // current whether or not anything was heard.
                chip.counter(&mut counter)
                    .map_err(|error| format!("the counter stopped answering: {error}"))?;
            }
            _ = report.tick() => {
                token = token.wrapping_add(1);
                let status = Stat::new()
                    .at(seconds_now())
                    .with_counts(heard, checked, forwarded)
                    .with_downlinks(downlinks, sent);
                let push = Datagram::PushData {
                    token,
                    gateway: config.gateway,
                    uplink: Uplink::from(status),
                };
                send(&socket, &push).await?;
            }
            received = socket.recv(&mut datagram) => {
                let len = received.map_err(|error| format!("the uplink closed: {error}"))?;
                if let Ok(Datagram::PullResp { token: asked, transmit }) =
                    Datagram::parse(&datagram[..len])
                {
                    downlinks += 1;
                    let status = match transmit_one(
                        &mut chip,
                        config,
                        &transmit,
                        &mut counter,
                        &mut band_free_at,
                    ) {
                        Ok(()) => {
                            sent += 1;
                            TxStatus::None
                        }
                        Err((status, why)) => {
                            eprintln!("pamoja-gateway: {why}");
                            status
                        }
                    };
                    let answer = Datagram::TxAck {
                        token: asked,
                        gateway: config.gateway,
                        status,
                    };
                    send(&socket, &answer).await?;
                }
            }
        }
    }
}

/// The clock a station reports its own time in.
///
/// The protocol carries 48 bits of microseconds, and the concentrator counts in 27 bits with
/// five more for the rollovers it has seen, which together is exactly 32 bits. So one full
/// turn of the counter's own wrap field is 2^32 microseconds, and counting those turns widens
/// the 32 bits the driver keeps into the 48 the protocol wants. The low 32 bits of the result
/// are still the counter value the chain is triggered at, which is what makes a downlink the
/// server timed against this clock land where it was asked for.
struct Clock {
    /// How many times the counter's wrap field has turned over.
    epochs: u64,
    /// The wrap count last seen, to notice when it does.
    wraps: u8,
}

impl Clock {
    /// A clock that has read nothing yet.
    const fn new() -> Clock {
        Clock {
            epochs: 0,
            wraps: 0,
        }
    }

    /// Takes a reading, noticing a turn of the wrap field.
    fn advance(&mut self, counter: &Counter) {
        if counter.free.wraps < self.wraps {
            self.epochs += 1;
        }
        self.wraps = counter.free.wraps;
    }

    /// The station time for a moment the driver has widened.
    ///
    /// The session byte sits above the microseconds and the transmit unit above that. A
    /// gateway with one chain is always unit zero, and the session is fixed for the run, so
    /// an `xtime` echoed back by the server still resolves to this one.
    fn at(&self, widened: u32, session: u8) -> i64 {
        let micros = (self.epochs << 32) | u64::from(widened);
        let tagged = (u64::from(session) << 48) | (micros & 0xffff_ffff_ffff);
        tagged as i64
    }
}

/// Runs a gateway that speaks the Basics Station protocol.
///
/// The station asks its discovery endpoint where the network server is, opens the websocket
/// it names, and is told how to configure its radios. After that it reports what it hears
/// and transmits what it is asked to, both in the server's own terms: a data rate is an index
/// into the table the server sent rather than a spreading factor, and a downlink is timed
/// against the uplink it answers rather than against a concentrator count.
async fn stationing(
    mut chip: LinuxConcentrator,
    config: &Config,
    endpoint: &str,
) -> Result<(), String> {
    let mut station = Station::connect(endpoint, config.gateway)
        .await
        .map_err(|error| format!("{endpoint}: {error}"))?;

    // The server names the region, the band it may use and the data rates it counts in. A
    // station that cannot read them has nothing to report a packet in terms of.
    let Message::RouterConfig {
        region,
        data_rates,
        freq_range,
        max_eirp,
        ..
    } = station.config().clone()
    else {
        return Err(
            "the server answered the version with something other than a configuration".to_owned(),
        );
    };

    // The server names the ceiling for the region it put this gateway in, so the power a
    // downlink goes out at comes from the network rather than from the file on the board.
    let max_power_dbm = max_eirp.clamp(f64::from(i8::MIN), f64::from(i8::MAX)) as i8;
    println!(
        "pamoja-gateway: {region}, {} data rates, {} to {} Hz, up to {max_power_dbm} dBm",
        data_rates.len(),
        freq_range.0,
        freq_range.1
    );

    // Fixed for the run, so an xtime the server echoes back belongs to this session and not
    // to the one before a restart.
    let session = seconds_now() as u8;

    let mut counter = Counter::new();
    let mut clock = Clock::new();
    let mut buffer = [0u8; BUFFER_LEN];
    let mut band_free_at: Option<u32> = None;

    loop {
        // A timeout rather than a select: the client is one object, and holding a receive
        // open while sending would borrow it twice.
        match tokio::time::timeout(POLL, station.recv()).await {
            Ok(message) => {
                let message = message.map_err(|error| format!("the session ended: {error}"))?;
                answer(
                    &mut chip,
                    &mut station,
                    config,
                    &mut counter,
                    &clock,
                    &mut band_free_at,
                    (session, max_power_dbm),
                    message,
                )
                .await?;
            }
            Err(_) => {
                chip.counter(&mut counter)
                    .map_err(|error| format!("the counter stopped answering: {error}"))?;
                clock.advance(&counter);

                let taken = chip
                    .receive(&mut buffer)
                    .map_err(|error| format!("the concentrator stopped answering: {error}"))?;

                for packet in rx::packets(taken) {
                    let Some(report) =
                        overheard(&packet, config, &data_rates, &clock, &counter, session)
                    else {
                        continue;
                    };
                    station
                        .send(&report)
                        .await
                        .map_err(|error| format!("the session stopped taking uplinks: {error}"))?;
                }
            }
        }
    }
}

/// Turns a packet the concentrator heard into the message a station reports it with.
///
/// Returns `None` for a packet on a channel that was never configured, one the frequency
/// shift keying receiver heard, or one whose settings are not in the table the server sent.
fn overheard(
    packet: &rx::Packet<'_>,
    config: &Config,
    data_rates: &[(u8, u32, bool)],
    clock: &Clock,
    counter: &Counter,
    session: u8,
) -> Option<Message> {
    let frequency_hz = forward::carrier(
        config.radio.carrier_hz,
        &config.radio.channels,
        packet.channel,
    )?;

    // A station counts in the server's own data rates, so the spreading factor and bandwidth
    // have to be looked up rather than sent. An uplink cannot arrive on a downlink-only rate.
    let data_rate =
        data_rates
            .iter()
            .position(|(spreading_factor, bandwidth_hz, downlink_only)| {
                !downlink_only
                    && *spreading_factor == packet.datarate
                    && *bandwidth_hz == MULTI_BANDWIDTH_HZ
            })?;

    let levels = Levels {
        rctx: 0,
        xtime: clock.at(counter.widened_packet(packet.timestamp), session),
        gpstime: None,
        rssi: f64::from(packet.rssi_channel),
        snr: f64::from(packet.snr_average) / 4.0,
    };

    Message::heard(packet.payload, data_rate as u8, frequency_hz, levels).ok()
}

/// Answers one message from the network server.
#[allow(clippy::too_many_arguments)]
async fn answer(
    chip: &mut LinuxConcentrator,
    station: &mut Station,
    config: &Config,
    counter: &mut Counter,
    clock: &Clock,
    band_free_at: &mut Option<u32>,
    run: (u8, i8),
    message: Message,
) -> Result<(), String> {
    let (session, max_power_dbm) = run;
    match message {
        Message::Downlink {
            dev_eui,
            diid,
            pdu,
            rx_delay,
            rx1,
            rx2,
            xtime,
            rctx,
            ..
        } => {
            // The moment is timed against the uplink this answers, and the server hands that
            // back untouched. Without it there is nothing to schedule against, and inventing
            // one would put the packet on the air while the device is not listening.
            let Some(uplink_at) = xtime else {
                eprintln!("pamoja-gateway: a downlink carried no uplink time to answer at");
                return Ok(());
            };

            // A delay of zero, or none at all, is one second rather than no delay, and the
            // second window always follows the first by another second.
            let delay = i64::from(rx_delay.unwrap_or(1).max(1));
            let windows = [
                rx1.map(|(rate, frequency)| (uplink_at + delay * 1_000_000, rate, frequency)),
                rx2.map(|(rate, frequency)| (uplink_at + (delay + 1) * 1_000_000, rate, frequency)),
            ];

            match transmit_at(
                chip,
                config,
                counter,
                band_free_at,
                &pdu,
                max_power_dbm,
                &windows,
            ) {
                Ok(window) => {
                    let report = Message::Transmitted {
                        diid,
                        dev_eui,
                        rctx: rctx.unwrap_or(0),
                        xtime: window,
                        txtime: window as f64 / 1e6,
                        gpstime: None,
                    };
                    station
                        .send(&report)
                        .await
                        .map_err(|error| format!("the session stopped taking reports: {error}"))?;
                }
                Err(why) => eprintln!("pamoja-gateway: {why}"),
            }
        }
        Message::TimeSync { txtime, .. } => {
            // The server keeps the two clocks together. Answering with the station time it
            // asked about is what lets it work out the offset.
            let answer = Message::TimeSync {
                txtime,
                xtime: Some(clock.at(counter.free.widened(), session)),
                gpstime: None,
            };
            station
                .send(&answer)
                .await
                .map_err(|error| format!("the session stopped taking the clock: {error}"))?;
        }
        Message::Schedule { frames } => {
            eprintln!(
                "pamoja-gateway: a schedule of {} frames is not driven by this program",
                frames.len()
            );
        }
        other => {
            eprintln!("pamoja-gateway: {} is not read here", other.msgtype());
        }
    }
    Ok(())
}

/// Puts one downlink on the air at the first window that is still reachable.
///
/// The server names one window or two. The second is tried when the first is already too
/// close to program, which is what the reference does rather than letting a packet go out
/// after the device has stopped listening.
///
/// # Returns
///
/// The station time the packet was scheduled at, which is what the transmission report
/// carries back.
fn transmit_at(
    chip: &mut LinuxConcentrator,
    config: &Config,
    counter: &mut Counter,
    band_free_at: &mut Option<u32>,
    pdu: &[u8],
    max_power_dbm: i8,
    windows: &[Option<(i64, u8, u32)>; 2],
) -> Result<i64, String> {
    let (now, _) = chip
        .counter(counter)
        .map_err(|error| format!("the counter stopped answering: {error}"))?;

    // The low bits of the station clock are the counter a chain is triggered at, so a window
    // becomes a trigger without conversion.
    let (window, data_rate, frequency_hz) = windows
        .iter()
        .flatten()
        .find(|(at, _, _)| (*at as u32).wrapping_sub(now) > TOO_LATE_US)
        .copied()
        .ok_or_else(|| "every window the server named is too close to program".to_owned())?;
    let at = window as u32;

    // A station is told a data rate rather than a spreading factor, and the table it came
    // from names the bandwidth. Every rate a gateway answers on here is the 125 kHz one.
    let link = LinkSettings::new(data_rate, MULTI_BANDWIDTH_HZ).without_crc();
    let delay = start_delay(FrontEnd::Sx1250, link.bandwidth_hz(), CHIRP_LOWPASS)
        .ok_or_else(|| format!("{MULTI_BANDWIDTH_HZ} Hz is a bandwidth no front end covers"))?;
    let gain = gain_for(&config.radio.gains, max_power_dbm)
        .ok_or_else(|| "the board names an empty transmit gain table".to_owned())?;

    if let Some(free_at) = *band_free_at {
        let owed = free_at.wrapping_sub(now);
        if owed != 0 && owed < u32::MAX / 2 {
            return Err(format!(
                "the duty cycle owes this band another {owed} us of silence"
            ));
        }
    }

    let request = Transmit {
        frequency_hz,
        link,
        gain,
        invert_polarity: true,
        public: config.radio.lorawan_public,
        payload: pdu,
    };

    chip.transmit(Chain::A, &request, Trigger::At(at), delay)
        .map_err(|error| format!("the downlink was refused: {error}"))?;

    if let Some(permille) = config.radio.duty_cycle_permille {
        let quiet = link.airtime_us(pdu.len()) + link.min_off_time_us(pdu.len(), permille);
        *band_free_at = Some(now.wrapping_add(u32::try_from(quiet).unwrap_or(u32::MAX)));
    }

    Ok(window)
}

/// Puts one downlink on the air, or says why it did not.
///
/// The protocol answers with one of eight words and no others, so not every refusal here has
/// an exact one. A modulation this daemon does not drive has no word at all, and a duty
/// cycle that is not yet spent is reported as a collision, because the slot genuinely is
/// taken: by the silence the last transmission owes.
fn transmit_one(
    chip: &mut LinuxConcentrator,
    config: &Config,
    transmit: &pamoja_gateway::udp::Txpk,
    counter: &mut Counter,
    band_free_at: &mut Option<u32>,
) -> Result<(), (TxStatus, String)> {
    let link = transmit.modulation.link().ok_or_else(|| {
        (
            TxStatus::TxFreq,
            "a downlink that is not LoRa is not driven by this program".to_owned(),
        )
    })?;

    // A GPS time can only be kept by a gateway with a GPS, and this one drives none.
    if transmit.gps_millis.is_some() {
        return Err((
            TxStatus::GpsUnlocked,
            "a downlink asked for a GPS time and this gateway has no GPS".to_owned(),
        ));
    }

    if let Some((lowest, highest)) = config.radio.tx_bounds {
        if transmit.frequency_hz < lowest || transmit.frequency_hz > highest {
            return Err((
                TxStatus::TxFreq,
                format!(
                    "{} Hz is outside the {lowest} to {highest} Hz this board transmits in",
                    transmit.frequency_hz
                ),
            ));
        }
    }

    let delay =
        start_delay(FrontEnd::Sx1250, link.bandwidth_hz(), CHIRP_LOWPASS).ok_or_else(|| {
            (
                TxStatus::TxFreq,
                format!(
                    "{} Hz is a bandwidth no front end covers",
                    link.bandwidth_hz()
                ),
            )
        })?;

    // The strongest entry that does not exceed what was asked for, which is what the
    // reference does: a request above the table transmits at the most the board reaches
    // rather than being refused.
    let gain = gain_for(&config.radio.gains, transmit.power_dbm).ok_or_else(|| {
        (
            TxStatus::TxPower,
            "the board names an empty transmit gain table".to_owned(),
        )
    })?;

    // Read the counter here rather than trusting the keepalive beat. A receive window is a
    // matter of microseconds, and a reading seconds old decides nothing.
    let (now, _) = chip.counter(counter).map_err(|error| {
        (
            TxStatus::TooLate,
            format!("the counter stopped answering: {error}"),
        )
    })?;

    let trigger = match (transmit.immediate, transmit.timestamp_us) {
        (true, _) | (false, None) => Trigger::Immediate,
        (false, Some(at)) => {
            // Unsigned throughout, so a counter that has rolled over still subtracts right.
            let ahead = at.wrapping_sub(now);
            if ahead <= TOO_LATE_US {
                return Err((
                    TxStatus::TooLate,
                    format!("a window {ahead} us away is too close to program"),
                ));
            }
            if ahead > TOO_EARLY_US {
                return Err((
                    TxStatus::TooEarly,
                    format!("a window {ahead} us away is further out than a downlink is sent"),
                ));
            }
            Trigger::At(at)
        }
    };

    // Still owed while the remaining time reads as less than half the counter, which is what
    // parts a moment not yet reached from one long past.
    if let Some(free_at) = *band_free_at {
        let owed = free_at.wrapping_sub(now);
        if owed != 0 && owed < u32::MAX / 2 {
            return Err((
                TxStatus::CollisionPacket,
                format!("the duty cycle owes this band another {owed} us of silence"),
            ));
        }
    }

    let holding = chip.tx_status(Chain::A).map_err(|error| {
        (
            TxStatus::TooLate,
            format!("the chain stopped answering: {error}"),
        )
    })?;
    if !holding.is_free() {
        return Err((
            TxStatus::CollisionPacket,
            format!("the chain is {holding:?} with the packet before this one"),
        ));
    }

    let request = Transmit {
        frequency_hz: transmit.frequency_hz,
        link,
        gain,
        invert_polarity: transmit.invert_polarity,
        public: config.radio.lorawan_public,
        payload: &transmit.payload,
    };

    chip.transmit(Chain::A, &request, trigger, delay)
        .map_err(|error| {
            (
                TxStatus::TxFreq,
                format!("the downlink was refused: {error}"),
            )
        })?;

    // The band owes silence from the end of this packet, not the start of it.
    if let Some(permille) = config.radio.duty_cycle_permille {
        let quiet = link.airtime_us(transmit.payload.len())
            + link.min_off_time_us(transmit.payload.len(), permille);
        *band_free_at = Some(now.wrapping_add(u32::try_from(quiet).unwrap_or(u32::MAX)));
    }

    Ok(())
}

/// Sends one datagram.
async fn send(socket: &UdpSocket, packet: &Datagram) -> Result<(), String> {
    socket
        .send(&packet.to_bytes())
        .await
        .map(|_| ())
        .map_err(|error| format!("the uplink stopped taking datagrams: {error}"))
}

/// The gateway clock, in seconds since the epoch.
fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A counter that has seen this many rollovers of its microseconds.
    fn counter_at(wraps: u8) -> Counter {
        let mut counter = Counter::new();
        counter.free.wraps = wraps;
        counter
    }

    #[test]
    fn the_station_clock_leaves_the_counter_in_its_low_bits() {
        let mut clock = Clock::new();
        clock.advance(&counter_at(5));

        // The low thirty two bits stay the value the chain is triggered at, which is what
        // lets a window the server timed become a trigger with no conversion at all.
        assert_eq!(clock.at(0x1234_5678, 0) as u32, 0x1234_5678);
    }

    #[test]
    fn a_turn_of_the_wrap_field_is_exactly_one_epoch() {
        let mut clock = Clock::new();
        clock.advance(&counter_at(31));
        let before = clock.at(0, 0);

        // Five bits of rollovers above twenty seven of microseconds is thirty two bits
        // together, so a turn of that field is 2^32 microseconds rather than a rounded
        // figure. Getting this wrong breaks downlink timing about seventy minutes in.
        clock.advance(&counter_at(0));
        assert_eq!(clock.at(0, 0) - before, 1_i64 << 32);
    }

    #[test]
    fn the_session_sits_above_the_microseconds() {
        let clock = Clock::new();

        // The session byte occupies bits 48 to 55, so it never disturbs a time below it.
        assert_eq!(clock.at(0xffff_ffff, 0), 0xffff_ffff_i64);
        assert_eq!(
            clock.at(0xffff_ffff, 0x2a),
            (0x2a_i64 << 48) | 0xffff_ffff_i64
        );
    }

    #[test]
    fn a_counter_that_has_not_turned_over_stays_in_one_epoch() {
        let mut clock = Clock::new();
        for wraps in [3, 7, 31] {
            clock.advance(&counter_at(wraps));
        }
        assert_eq!(clock.at(0, 0), 0);
    }
}
