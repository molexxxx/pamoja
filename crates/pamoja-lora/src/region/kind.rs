//! Whether a region's channels are created by the network or numbered in advance.

use super::FixedChannelList;

/// How a region defines its channels, which decides what a network can change on a device.
///
/// RP002-1.0.5 splits the regions in two. A dynamic plan gives every device a few default
/// channels and lets the network add more, with `NewChannelReq` or a join accept's channel
/// list, and answers in the first receive window on the uplink's own frequency. A fixed plan
/// numbers every channel in advance, and a network only enables and disables them.
///
/// # Examples
///
/// ```
/// use pamoja_lora::region::{FixedChannelList, PlanKind, Region};
///
/// assert_eq!(
///     Region::Eu868.plan().kind,
///     PlanKind::Dynamic { channel_list: Some(FixedChannelList::Mhz800) }
/// );
/// assert_eq!(Region::Us915.plan().kind, PlanKind::Fixed);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanKind {
    /// Default channels every device has, and more the network creates.
    Dynamic {
        /// The published numbering a type 1 channel list is read against, for a region
        /// whose regional parameters name one, and `None` where the region takes only a
        /// list of frequencies.
        channel_list: Option<FixedChannelList>,
    },
    /// Every channel numbered in advance, enabled and disabled by channel masks.
    Fixed,
}

impl PlanKind {
    /// Reports whether a network may create channels in this plan.
    ///
    /// # Returns
    ///
    /// `true` for a dynamic plan, which is the kind `NewChannelReq` and `DlChannelReq`
    /// apply to.
    pub const fn is_dynamic(self) -> bool {
        matches!(self, PlanKind::Dynamic { .. })
    }
}
