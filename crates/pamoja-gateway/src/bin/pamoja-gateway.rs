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
use pamoja_gateway::udp::{CrcStatus, Packet as Datagram, Stat, TxStatus, Uplink};
use pamoja_radios::linux::{self, LinuxConcentrator, Wiring};
use pamoja_radios::sx1302::channel::Plan;
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

    let wiring = Wiring::new(
        &config.concentrator.spi,
        &config.concentrator.gpio_chip,
        config.concentrator.reset_line,
    );
    let mut chip = linux::open_sx1302(&wiring).map_err(|error| error.to_string())?;

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
        Upstream::Station { endpoint } => Err(format!(
            "{endpoint}: the Basics Station uplink is not driven by this program yet; name a forwarder instead"
        )),
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
