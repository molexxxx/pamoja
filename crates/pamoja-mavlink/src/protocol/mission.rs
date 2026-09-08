//! The mission (plan) transfer protocol.
//!
//! A plan is transferred one item at a time, driven entirely by the receiver: the receiver
//! announces or is told the item count, then requests item 0, item 1, and so on, and the
//! sender answers each request with the matching [`MissionItemInt`]. The transfer ends with a
//! [`MissionAck`]. Items must arrive in order; an out-of-order item is dropped and the expected
//! sequence number is re-requested.
//!
//! The two roles are symmetric, so this module models them as two machines rather than by
//! direction: a [`MissionSender`] holds the items and answers requests, and a
//! [`MissionReceiver`] asks for items and collects them. A ground station uploading a plan is a
//! sender talking to a vehicle receiver; a ground station downloading a plan is a receiver
//! talking to a vehicle sender. The same two machines cover both, and neither allocates: the
//! sender borrows the item slice, and the receiver hands each item back to the caller instead
//! of storing them.

use crate::dialect::{
    mav_mission_result, MissionAck, MissionCount, MissionItemInt, MissionRequestInt,
    MissionRequestList,
};

/// Holds a plan and answers a receiver's requests for its items.
///
/// The sender borrows the items and stamps the target ids, the mission type, and the requested
/// sequence number onto each one as it is handed out, so the caller supplies only the item
/// content (command, frame, position, parameters).
pub struct MissionSender<'a> {
    items: &'a [MissionItemInt],
    target_system: u8,
    target_component: u8,
    mission_type: u8,
}

impl<'a> MissionSender<'a> {
    /// Creates a sender for a plan bound for a target vehicle.
    ///
    /// # Arguments
    ///
    /// * `items` - the mission items, in sequence order.
    /// * `target_system` - the receiving system's id.
    /// * `target_component` - the receiving component's id.
    /// * `mission_type` - the [`MAV_MISSION_TYPE`](crate::dialect::mav_mission_type) of the plan.
    ///
    /// # Returns
    ///
    /// The sender.
    pub fn new(
        items: &'a [MissionItemInt],
        target_system: u8,
        target_component: u8,
        mission_type: u8,
    ) -> Self {
        MissionSender {
            items,
            target_system,
            target_component,
            mission_type,
        }
    }

    /// Returns the number of items in the plan.
    ///
    /// # Returns
    ///
    /// The item count.
    pub fn len(&self) -> u16 {
        self.items.len() as u16
    }

    /// Reports whether the plan has no items.
    ///
    /// # Returns
    ///
    /// `true` if the plan is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Builds the [`MissionCount`] that opens the transfer.
    ///
    /// # Returns
    ///
    /// The count message to send first.
    pub fn count(&self) -> MissionCount {
        MissionCount {
            count: self.len(),
            target_system: self.target_system,
            target_component: self.target_component,
            mission_type: self.mission_type,
            opaque_id: 0,
        }
    }

    /// Builds the item to answer a request for `seq`, stamped with the sequence number, the
    /// target ids, and the mission type.
    ///
    /// # Arguments
    ///
    /// * `seq` - the requested sequence number.
    ///
    /// # Returns
    ///
    /// The item to send, or [`None`] if `seq` is past the end of the plan.
    pub fn item(&self, seq: u16) -> Option<MissionItemInt> {
        self.items.get(seq as usize).map(|item| MissionItemInt {
            seq,
            target_system: self.target_system,
            target_component: self.target_component,
            mission_type: self.mission_type,
            ..*item
        })
    }
}

/// The next thing a [`MissionReceiver`] should send.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReceiverAction {
    /// Ask for this sequence number.
    Request(MissionRequestInt),
    /// The transfer is complete; send this acknowledgment.
    Ack(MissionAck),
}

/// Requests a plan's items in order and collects them, ending with an acknowledgment.
///
/// The receiver tracks the announced count and the next expected sequence number. It never
/// stores items; each accepted item is handed back to the caller from
/// [`on_item`](MissionReceiver::on_item).
pub struct MissionReceiver {
    target_system: u8,
    target_component: u8,
    mission_type: u8,
    count: u16,
    next: u16,
    complete: bool,
}

impl MissionReceiver {
    /// Creates a receiver for a plan from a target vehicle.
    ///
    /// # Arguments
    ///
    /// * `target_system` - the sending system's id.
    /// * `target_component` - the sending component's id.
    /// * `mission_type` - the [`MAV_MISSION_TYPE`](crate::dialect::mav_mission_type) to transfer.
    ///
    /// # Returns
    ///
    /// The receiver, before any count is known.
    pub fn new(target_system: u8, target_component: u8, mission_type: u8) -> Self {
        MissionReceiver {
            target_system,
            target_component,
            mission_type,
            count: 0,
            next: 0,
            complete: false,
        }
    }

    /// Builds the [`MissionRequestList`] that starts a download.
    ///
    /// Used when the receiver initiates the transfer (a ground station downloading a plan); a
    /// receiver that is answering an unsolicited [`MissionCount`] does not send it.
    ///
    /// # Returns
    ///
    /// The request-list message.
    pub fn request_list(&self) -> MissionRequestList {
        MissionRequestList {
            target_system: self.target_system,
            target_component: self.target_component,
            mission_type: self.mission_type,
        }
    }

    /// Handles the announced item count and returns the first action.
    ///
    /// # Arguments
    ///
    /// * `count` - the number of items the sender will provide.
    ///
    /// # Returns
    ///
    /// A [`ReceiverAction::Request`] for item 0, or a [`ReceiverAction::Ack`] straight away if
    /// the plan is empty.
    pub fn on_count(&mut self, count: u16) -> ReceiverAction {
        self.count = count;
        self.next = 0;
        if count == 0 {
            self.complete = true;
            return ReceiverAction::Ack(self.ack());
        }
        ReceiverAction::Request(self.request(0))
    }

    /// Handles an incoming item and returns the accepted item plus the next action.
    ///
    /// An in-order item is accepted and returned, and the receiver advances to request the next
    /// one or to acknowledge the transfer if it was the last. An out-of-order item is not
    /// accepted, and the expected sequence number is re-requested.
    ///
    /// # Arguments
    ///
    /// * `item` - the decoded item.
    ///
    /// # Returns
    ///
    /// A pair of the accepted item (`Some` only when in order) and the next
    /// [`ReceiverAction`].
    pub fn on_item(&mut self, item: &MissionItemInt) -> (Option<MissionItemInt>, ReceiverAction) {
        if self.complete || item.seq != self.next {
            // Out of order (or after completion): re-request what is still expected.
            return (None, ReceiverAction::Request(self.request(self.next)));
        }
        self.next += 1;
        if self.next >= self.count {
            self.complete = true;
            (Some(*item), ReceiverAction::Ack(self.ack()))
        } else {
            (
                Some(*item),
                ReceiverAction::Request(self.request(self.next)),
            )
        }
    }

    /// Reports whether the transfer has finished.
    ///
    /// # Returns
    ///
    /// `true` once every item has been received and the acknowledgment produced.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Returns the next sequence number the receiver expects.
    ///
    /// # Returns
    ///
    /// The expected sequence number.
    pub fn expected(&self) -> u16 {
        self.next
    }

    fn request(&self, seq: u16) -> MissionRequestInt {
        MissionRequestInt {
            seq,
            target_system: self.target_system,
            target_component: self.target_component,
            mission_type: self.mission_type,
        }
    }

    fn ack(&self) -> MissionAck {
        MissionAck {
            target_system: self.target_system,
            target_component: self.target_component,
            type_: mav_mission_result::ACCEPTED,
            mission_type: self.mission_type,
            opaque_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::{mav_cmd, mav_frame, mav_mission_type};

    fn waypoint(seq: u16, lat: i32, lon: i32, alt: f32) -> MissionItemInt {
        MissionItemInt {
            param1: 0.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            x: lat,
            y: lon,
            z: alt,
            seq,
            command: mav_cmd::NAV_WAYPOINT,
            target_system: 0,
            target_component: 0,
            frame: mav_frame::GLOBAL_RELATIVE_ALT_INT,
            current: (seq == 0) as u8,
            autocontinue: 1,
            mission_type: mav_mission_type::MISSION,
        }
    }

    #[test]
    fn the_sender_stamps_the_target_and_sequence_onto_each_item() {
        let items = [waypoint(0, 10, 20, 30.0), waypoint(1, 11, 21, 31.0)];
        let sender = MissionSender::new(&items, 7, 8, mav_mission_type::MISSION);
        assert_eq!(sender.count().count, 2);
        assert_eq!(sender.count().target_system, 7);

        let item = sender.item(1).unwrap();
        assert_eq!(item.seq, 1);
        assert_eq!(item.target_system, 7);
        assert_eq!(item.target_component, 8);
        assert_eq!(item.x, 11);
        assert!(sender.item(2).is_none());
    }

    #[test]
    fn a_sender_and_receiver_complete_an_in_order_transfer() {
        // This drives the documented upload/download exchange end to end: COUNT, then a
        // REQUEST_INT and ITEM_INT for each sequence number, then an ACK.
        let items = [
            waypoint(0, 10, 20, 30.0),
            waypoint(1, 11, 21, 31.0),
            waypoint(2, 12, 22, 32.0),
        ];
        let sender = MissionSender::new(&items, 1, 1, mav_mission_type::MISSION);
        let mut receiver = MissionReceiver::new(1, 1, mav_mission_type::MISSION);

        let mut action = receiver.on_count(sender.count().count);
        let mut collected = [MissionItemInt::default_zeroed(); 3];
        let mut collected_len = 0usize;
        loop {
            match action {
                ReceiverAction::Request(request) => {
                    let item = sender.item(request.seq).expect("in range");
                    let (accepted, next) = receiver.on_item(&item);
                    if let Some(item) = accepted {
                        collected[collected_len] = item;
                        collected_len += 1;
                    }
                    action = next;
                }
                ReceiverAction::Ack(ack) => {
                    assert_eq!(ack.type_, mav_mission_result::ACCEPTED);
                    break;
                }
            }
        }
        assert!(receiver.is_complete());
        assert_eq!(collected_len, 3);
        assert_eq!(collected[0].x, 10);
        assert_eq!(collected[2].x, 12);
    }

    #[test]
    fn an_out_of_order_item_is_dropped_and_re_requested() {
        let items = [waypoint(0, 10, 20, 30.0), waypoint(1, 11, 21, 31.0)];
        let sender = MissionSender::new(&items, 1, 1, mav_mission_type::MISSION);
        let mut receiver = MissionReceiver::new(1, 1, mav_mission_type::MISSION);

        let first = receiver.on_count(sender.count().count);
        assert_eq!(first, ReceiverAction::Request(sender_request(0)));

        // The sender (wrongly) answers with item 1 instead of item 0.
        let (accepted, action) = receiver.on_item(&sender.item(1).unwrap());
        assert!(accepted.is_none());
        // The receiver still wants item 0.
        assert_eq!(action, ReceiverAction::Request(sender_request(0)));
        assert_eq!(receiver.expected(), 0);
    }

    #[test]
    fn an_empty_plan_acknowledges_immediately() {
        let items: [MissionItemInt; 0] = [];
        let sender = MissionSender::new(&items, 1, 1, mav_mission_type::MISSION);
        let mut receiver = MissionReceiver::new(1, 1, mav_mission_type::MISSION);
        match receiver.on_count(sender.count().count) {
            ReceiverAction::Ack(ack) => assert_eq!(ack.type_, mav_mission_result::ACCEPTED),
            ReceiverAction::Request(_) => panic!("an empty plan needs no items"),
        }
        assert!(receiver.is_complete());
    }

    fn sender_request(seq: u16) -> MissionRequestInt {
        MissionRequestInt {
            seq,
            target_system: 1,
            target_component: 1,
            mission_type: mav_mission_type::MISSION,
        }
    }

    impl MissionItemInt {
        fn default_zeroed() -> Self {
            MissionItemInt {
                param1: 0.0,
                param2: 0.0,
                param3: 0.0,
                param4: 0.0,
                x: 0,
                y: 0,
                z: 0.0,
                seq: 0,
                command: 0,
                target_system: 0,
                target_component: 0,
                frame: 0,
                current: 0,
                autocontinue: 0,
                mission_type: 0,
            }
        }
    }
}
