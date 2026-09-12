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
use pamoja_radios::sx1302::tx::{start_delay, Chain, Trigger};
use tokio::net::UdpSocket;

/// How long to wait between asking the concentrator what it heard.
const POLL: Duration = Duration::from_millis(10);

/// How often a gateway holds its route open, which the protocol expects every few seconds.
const KEEPALIVE: Duration = Duration::from_secs(5);

/// How often a gateway reports how it is doing.
const REPORT: Duration = Duration::from_secs(30);

/// The filter setting the channels are configured with, which the start delay is worked from.
const CHIRP_LOWPASS: u8 = 6;

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
    let mut sent = 0u32;

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
                    .with_downlinks(sent, sent);
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
                    let status = match transmit_one(&mut chip, &transmit) {
                        Ok(()) => {
                            sent += 1;
                            TxStatus::None
                        }
                        Err(why) => {
                            eprintln!("pamoja-gateway: {why}");
                            TxStatus::TxFreq
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

/// Puts one downlink on the air.
fn transmit_one(
    chip: &mut LinuxConcentrator,
    transmit: &pamoja_gateway::udp::Txpk,
) -> Result<(), String> {
    let link = transmit
        .modulation
        .link()
        .ok_or_else(|| "a downlink that is not LoRa is not driven by this program".to_owned())?;
    let delay = start_delay(
        pamoja_radios::sx1302::tx::FrontEnd::Sx1250,
        link.bandwidth_hz(),
        CHIRP_LOWPASS,
    )
    .ok_or_else(|| {
        format!(
            "{} Hz is a bandwidth no front end covers",
            link.bandwidth_hz()
        )
    })?;

    let trigger = match (transmit.immediate, transmit.timestamp_us) {
        (true, _) => Trigger::Immediate,
        (false, Some(at)) => Trigger::At(at),
        (false, None) => Trigger::Immediate,
    };

    chip.transmit(Chain::A, &transmit.payload, trigger, delay)
        .map_err(|error| format!("the downlink was refused: {error}"))
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
