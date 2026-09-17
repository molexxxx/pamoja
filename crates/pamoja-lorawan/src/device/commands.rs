//! What a device does with the MAC commands a downlink carries.
//!
//! Each command follows its section of TS001-1.0.4 chapter 5, with the regional meaning of
//! its fields from RP002-1.0.5. A command that changes several settings changes all of them
//! or none, and says which parts it could not take.

use pamoja_lora::region::PlanKind;

use super::channels::DYNAMIC_MAX_CHANNELS;
use super::{Channel, Delivery, DeviceTime, EndDevice, LinkCheck};
use crate::mac::{eirp_dbm, receive_delay_s, MacCommand, MacCommands};
use crate::{Direction, Version};

/// The most `LinkADRReq` commands a device takes as one contiguous block.
const MAX_BLOCK: usize = 64;

impl EndDevice<'_> {
    /// Acts on every command in a downlink's frame options or port 0 payload, in order.
    ///
    /// Reading stops at a command this version does not know, since a command carries no
    /// length to skip it by.
    pub(super) fn process_commands(&mut self, bytes: &[u8], snr_db: i8, delivery: &mut Delivery) {
        let mut commands = MacCommands::new(Direction::Downlink, bytes).peekable();
        while let Some(Ok(command)) = commands.next() {
            match command {
                MacCommand::LinkAdrReq {
                    channel_mask,
                    mask_control,
                    ..
                } => {
                    let mut block = [(0u16, 0u8); MAX_BLOCK];
                    block[0] = (channel_mask, mask_control);
                    let mut count = 1;
                    let mut last = command;
                    while let Some(Ok(MacCommand::LinkAdrReq {
                        channel_mask,
                        mask_control,
                        ..
                    })) = commands.peek().copied()
                    {
                        if count < MAX_BLOCK {
                            block[count] = (channel_mask, mask_control);
                        }
                        count += 1;
                        last = commands.next().and_then(Result::ok).unwrap_or(last);
                    }
                    if let MacCommand::LinkAdrReq {
                        data_rate,
                        tx_power,
                        transmissions,
                        ..
                    } = last
                    {
                        self.link_adr(
                            &block[..count.min(MAX_BLOCK)],
                            data_rate,
                            tx_power,
                            transmissions,
                            count,
                        );
                    }
                }
                MacCommand::DutyCycleReq { max_duty_cycle } => {
                    self.max_duty_cycle = max_duty_cycle & 0x0F;
                    self.answers.push(MacCommand::DutyCycleAns, false);
                }
                MacCommand::RxParamSetupReq {
                    rx1_offset,
                    rx2_data_rate,
                    frequency_hz,
                } => {
                    let channel_ack = self.settings.usable(frequency_hz);
                    let rx2_data_rate_ack = self.downlink_link(rx2_data_rate).is_ok();
                    let rx1_offset_ack = rx1_offset <= self.plan.max_rx1_data_rate_offset;
                    if channel_ack && rx2_data_rate_ack && rx1_offset_ack {
                        self.rx1_dr_offset = rx1_offset;
                        self.rx2_data_rate = rx2_data_rate;
                        self.rx2_frequency_hz = frequency_hz;
                    }
                    self.answers.push(
                        MacCommand::RxParamSetupAns {
                            rx1_offset_ack,
                            rx2_data_rate_ack,
                            channel_ack,
                        },
                        true,
                    );
                }
                MacCommand::DevStatusReq => {
                    self.answers.push(
                        MacCommand::DevStatusAns {
                            battery: self.battery.code(),
                            margin: snr_db.clamp(-32, 31),
                        },
                        false,
                    );
                }
                MacCommand::NewChannelReq {
                    index,
                    frequency_hz,
                    max_data_rate,
                    min_data_rate,
                } => {
                    if !self.plan.kind.is_dynamic() {
                        continue;
                    }
                    let index = usize::from(index);
                    // TS001-1.0.x keeps the default channels out of reach of this command,
                    // and RP002-1.0.5 ends a dynamic plan at channel 79.
                    let in_range =
                        index >= self.channels.default_count() && index < DYNAMIC_MAX_CHANNELS;
                    let (frequency_ok, data_rate_range_ok) = if frequency_hz == 0 {
                        (in_range, in_range)
                    } else {
                        (
                            in_range && self.settings.usable(frequency_hz),
                            in_range
                                && min_data_rate <= max_data_rate
                                && self.plan.uplink_data_rate(min_data_rate).is_some()
                                && self.plan.uplink_data_rate(max_data_rate).is_some(),
                        )
                    };
                    if frequency_ok && data_rate_range_ok {
                        if frequency_hz == 0 {
                            self.channels.remove(index);
                        } else {
                            self.channels.create(
                                index,
                                Channel::new(frequency_hz, min_data_rate, max_data_rate),
                            );
                        }
                    }
                    self.answers.push(
                        MacCommand::NewChannelAns {
                            data_rate_range_ok,
                            frequency_ok,
                        },
                        false,
                    );
                }
                MacCommand::RxTimingSetupReq { delay } => {
                    self.rx1_delay_us = u32::from(receive_delay_s(delay & 0x0F)) * 1_000_000;
                    self.answers.push(MacCommand::RxTimingSetupAns, true);
                }
                MacCommand::TxParamSetupReq {
                    max_eirp,
                    uplink_dwell,
                    downlink_dwell,
                } => {
                    // TS001-1.0.4 section 5.8: a region that does not implement the command
                    // neither applies nor answers it.
                    if !self.plan.tx_param_setup {
                        continue;
                    }
                    if let Some(dbm) = eirp_dbm(max_eirp) {
                        self.max_eirp_dbm = dbm as i8;
                    }
                    self.uplink_dwell = uplink_dwell;
                    self.downlink_dwell = downlink_dwell;
                    self.answers.push(MacCommand::TxParamSetupAns, true);
                }
                MacCommand::DlChannelReq {
                    index,
                    frequency_hz,
                } => {
                    if !matches!(self.plan.kind, PlanKind::Dynamic { .. }) {
                        continue;
                    }
                    let index = usize::from(index);
                    let uplink_frequency_exists = self.channels.get(index).is_some();
                    let frequency_ok = self.settings.usable(frequency_hz);
                    if uplink_frequency_exists && frequency_ok {
                        self.channels.set_downlink(index, frequency_hz);
                    }
                    self.answers.push(
                        MacCommand::DlChannelAns {
                            uplink_frequency_exists,
                            frequency_ok,
                        },
                        true,
                    );
                }
                MacCommand::LinkCheckAns { margin, gateways } => {
                    delivery.link_check = Some(LinkCheck {
                        margin_db: margin,
                        gateways,
                    });
                }
                MacCommand::DeviceTimeAns { seconds, fraction } => {
                    delivery.device_time = Some(DeviceTime {
                        gps_seconds: seconds,
                        fraction,
                    });
                }
                _ => {}
            }
        }
    }

    /// Takes a contiguous block of `LinkADRReq` commands, TS001-1.0.4 section 5.2.
    ///
    /// The channel controls apply in order as one mask; the data rate, power and repetition
    /// come from the last command. With adaptive data rate on, or under LoRaWAN 1.0.3, the
    /// whole block is taken or none of it. With it off, TS001-1.0.4 lets each part stand
    /// alone. A value of 15 for the data rate or power keeps the current one, which is a
    /// TS001-1.0.4 rule.
    fn link_adr(
        &mut self,
        block: &[(u16, u8)],
        data_rate: u8,
        tx_power: u8,
        transmissions: u8,
        requests: usize,
    ) {
        let v1_0_4 = self.settings.version == Version::V1_0_4;
        let mask = self.channels.mask_after(block, &self.plan.mask_controls);
        let channel_mask_ack = mask.is_some();
        let working = mask.unwrap_or(self.channels.mask());

        let data_rate = if v1_0_4 && data_rate == 0x0F {
            self.data_rate
        } else {
            data_rate
        };
        let data_rate_ack = self.uplink_link(data_rate).is_ok()
            && self.application_room(data_rate).is_ok()
            && self.channels.carries(&working, data_rate);

        let keep_power = v1_0_4 && tx_power == 0x0F;
        let tx_power = if keep_power { self.tx_power } else { tx_power };
        let power_ack = keep_power || self.power_reachable(tx_power);

        let whole = channel_mask_ack && data_rate_ack && power_ack;
        let piecemeal = v1_0_4 && !self.settings.adr;
        if whole || piecemeal {
            if channel_mask_ack {
                self.channels.set_mask(working);
                self.nb_trans = if transmissions == 0 { 1 } else { transmissions };
            }
            if data_rate_ack {
                self.data_rate = data_rate;
            }
            if power_ack {
                self.tx_power = tx_power;
            }
        }

        let answer = MacCommand::LinkAdrAns {
            power_ack,
            data_rate_ack,
            channel_mask_ack,
        };
        let answers = if v1_0_4 { requests } else { 1 };
        for _ in 0..answers {
            self.answers.push(answer, false);
        }
    }

    /// Reports whether the radio can go at or below the power an index names.
    ///
    /// TS001-1.0.4 section 5.2: a power above what the device reaches is acknowledged and
    /// sent at its maximum; one below what it reaches is refused.
    fn power_reachable(&self, index: u8) -> bool {
        self.plan
            .tx_power_dbm(index, self.max_eirp_dbm)
            .is_some_and(|power| self.radio_dbm(power) >= i16::from(self.settings.min_output_dbm))
    }
}
