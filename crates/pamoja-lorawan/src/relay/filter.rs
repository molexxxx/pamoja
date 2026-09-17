//! Which join requests a relay forwards, TS011-1.0.1 sections 8.6 and 10.3.

use crate::mac::{MacCommand, FILTER_EUI_MAX};

/// How many rules a relay's join request filter holds, rule 0 being the default,
/// TS011-1.0.1 section 8.6.
pub const FILTER_RULES: usize = 16;

/// What a filter rule does with the join requests it matches, TS011-1.0.1 table 48.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterAction {
    /// Forward the join request to the network.
    Forward,
    /// Drop it.
    Filter,
}

impl FilterAction {
    /// Reads a coded action.
    ///
    /// # Arguments
    ///
    /// * `code` - the value a `FilterListReq` carries.
    ///
    /// # Returns
    ///
    /// The action for 1 and 2, or `None` for 0, which removes a rule, and 3, which is
    /// reserved.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<FilterAction> {
        match code {
            1 => Some(FilterAction::Forward),
            2 => Some(FilterAction::Filter),
            _ => None,
        }
    }

    /// Returns the value a `FilterListReq` carries.
    ///
    /// # Returns
    ///
    /// 1 to forward, 2 to filter.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            FilterAction::Forward => 1,
            FilterAction::Filter => 2,
        }
    }
}

/// One rule of a join request filter: the leading bytes of JoinEUI then DevEUI it matches,
/// and what it does with a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FilterRule {
    prefix: [u8; FILTER_EUI_MAX],
    len: u8,
    action: FilterAction,
}

impl FilterRule {
    /// Builds a rule.
    ///
    /// # Arguments
    ///
    /// * `prefix` - the leading bytes of JoinEUI then DevEUI, most significant first as an
    ///   EUI is written: three bytes match an organizationally unique identifier, eight a
    ///   JoinEUI, and sixteen one device.
    /// * `action` - what to do with a join request it matches.
    ///
    /// # Returns
    ///
    /// The rule, or `None` for an empty prefix or one past sixteen bytes.
    #[must_use]
    pub fn new(prefix: &[u8], action: FilterAction) -> Option<FilterRule> {
        if prefix.is_empty() || prefix.len() > FILTER_EUI_MAX {
            return None;
        }
        let mut bytes = [0u8; FILTER_EUI_MAX];
        bytes[..prefix.len()].copy_from_slice(prefix);
        Some(FilterRule {
            prefix: bytes,
            len: prefix.len() as u8,
            action,
        })
    }

    /// Returns the bytes the rule matches.
    ///
    /// # Returns
    ///
    /// The prefix, most significant first.
    #[must_use]
    pub fn prefix(&self) -> &[u8] {
        &self.prefix[..usize::from(self.len)]
    }

    /// Returns what the rule does with a match.
    ///
    /// # Returns
    ///
    /// The action.
    #[must_use]
    pub const fn action(&self) -> FilterAction {
        self.action
    }

    /// Reports whether a device's identifiers begin with the rule's prefix.
    ///
    /// # Arguments
    ///
    /// * `join_eui` - the JoinEUI, most significant byte first.
    /// * `dev_eui` - the DevEUI, most significant byte first.
    ///
    /// # Returns
    ///
    /// `true` when every byte of the prefix matches.
    #[must_use]
    pub fn matches(&self, join_eui: &[u8; 8], dev_eui: &[u8; 8]) -> bool {
        join_eui
            .iter()
            .chain(dev_eui)
            .zip(self.prefix())
            .all(|(byte, rule)| byte == rule)
    }
}

/// A relay's join request filter: a default action and up to fifteen rules, decided by
/// the longest prefix that matches, TS011-1.0.1 section 8.6.
///
/// # Examples
///
/// The filter of TS011-1.0.1 appendix 3, which forwards one manufacturer's devices, filters
/// one JoinEUI of theirs except for a range of DevEUIs, filters one device in that range,
/// and filters everyone else:
///
/// ```
/// use pamoja_lorawan::relay::{FilterAction, FilterRule, JoinFilter};
///
/// let join_eui = [0xAB, 0xCD, 0xEF, 0xAB, 0xCD, 0xEF, 0xAB, 0xCD];
/// let range = [0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46];
///
/// let mut filter = JoinFilter::new();
/// filter.set_default(FilterAction::Filter);
/// let rules = [
///     FilterRule::new(&join_eui[..3], FilterAction::Forward),
///     FilterRule::new(&join_eui, FilterAction::Filter),
///     FilterRule::new(&[&join_eui[..], &range[..]].concat(), FilterAction::Forward),
///     FilterRule::new(&[&join_eui[..], &range[..], &[0x48]].concat(), FilterAction::Filter),
/// ];
/// for (index, rule) in (1..).zip(rules) {
///     assert!(filter.set_rule(index, rule));
/// }
///
/// assert_eq!(
///     filter.decide(&join_eui, &[0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x46]),
///     FilterAction::Forward,
///     "inside the range",
/// );
/// assert_eq!(
///     filter.decide(&join_eui, &[0x12, 0x34, 0x56, 0x78, 0x28, 0x37, 0x46, 0x48]),
///     FilterAction::Filter,
///     "the one device filtered out of it",
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JoinFilter {
    default: FilterAction,
    rules: [Option<FilterRule>; FILTER_RULES - 1],
}

impl JoinFilter {
    /// A filter that forwards every join request, as a relay starts.
    ///
    /// # Returns
    ///
    /// The filter.
    #[must_use]
    pub const fn new() -> JoinFilter {
        JoinFilter {
            default: FilterAction::Forward,
            rules: [None; FILTER_RULES - 1],
        }
    }

    /// Returns what happens to a join request no rule matches.
    ///
    /// # Returns
    ///
    /// Rule 0's action.
    #[must_use]
    pub const fn default_action(&self) -> FilterAction {
        self.default
    }

    /// Sets what happens to a join request no rule matches.
    ///
    /// # Arguments
    ///
    /// * `action` - rule 0's action.
    pub fn set_default(&mut self, action: FilterAction) {
        self.default = action;
    }

    /// Returns a rule.
    ///
    /// # Arguments
    ///
    /// * `index` - the rule, 1 to 15.
    ///
    /// # Returns
    ///
    /// The rule, or `None` for an empty index or one out of range.
    #[must_use]
    pub fn rule(&self, index: u8) -> Option<FilterRule> {
        let slot = usize::from(index).checked_sub(1)?;
        self.rules.get(slot).copied().flatten()
    }

    /// Sets or removes a rule.
    ///
    /// # Arguments
    ///
    /// * `index` - the rule, 1 to 15.
    /// * `rule` - the rule, or `None` to remove it.
    ///
    /// # Returns
    ///
    /// `false` for index 0, which [`set_default`](Self::set_default) sets, or past 15.
    pub fn set_rule(&mut self, index: u8, rule: Option<FilterRule>) -> bool {
        match usize::from(index)
            .checked_sub(1)
            .and_then(|slot| self.rules.get_mut(slot))
        {
            Some(slot) => {
                *slot = rule;
                true
            }
            None => false,
        }
    }

    /// Decides what happens to a join request, by the longest rule that matches it.
    ///
    /// Of two matching rules of the same length, the one at the higher index decides.
    ///
    /// # Arguments
    ///
    /// * `join_eui` - the request's JoinEUI, most significant byte first.
    /// * `dev_eui` - its DevEUI, most significant byte first.
    ///
    /// # Returns
    ///
    /// Forward or filter.
    #[must_use]
    pub fn decide(&self, join_eui: &[u8; 8], dev_eui: &[u8; 8]) -> FilterAction {
        let mut longest = 0;
        let mut action = self.default;
        for rule in self.rules.iter().flatten() {
            if rule.len >= longest && rule.matches(join_eui, dev_eui) {
                longest = rule.len;
                action = rule.action;
            }
        }
        action
    }

    /// Takes a `FilterListReq` and answers it, TS011-1.0.1 section 10.3.
    ///
    /// Rule 0 takes an empty prefix with forward or filter. Rules 1 to 15 take a prefix
    /// with forward or filter to set a rule, and an empty prefix with action 0 to remove
    /// one. Anything else changes nothing and is refused.
    ///
    /// # Arguments
    ///
    /// * `index` - the rule.
    /// * `action` - the coded action, table 48.
    /// * `eui_len` - the prefix length the request gave.
    /// * `eui` - the prefix, most significant first, as
    ///   [`MacCommand::FilterListReq`] holds it.
    ///
    /// # Returns
    ///
    /// The `FilterListAns` to send back.
    pub fn apply(
        &mut self,
        index: u8,
        action: u8,
        eui_len: u8,
        eui: &[u8; FILTER_EUI_MAX],
    ) -> MacCommand {
        let eui_len_ack = usize::from(eui_len) <= FILTER_EUI_MAX;
        let action_ack = action <= 2;
        let combined_rules_ack = matches!(
            (index, action, eui_len),
            (0, 1 | 2, 0) | (1..=15, 1 | 2, 1..) | (1..=15, 0, 0)
        );
        if eui_len_ack && action_ack && combined_rules_ack {
            let action = FilterAction::from_code(action);
            match (index, action) {
                (0, Some(action)) => self.set_default(action),
                (_, Some(action)) => {
                    self.set_rule(index, FilterRule::new(&eui[..usize::from(eui_len)], action));
                }
                (_, None) => {
                    self.set_rule(index, None);
                }
            }
        }
        MacCommand::FilterListAns {
            combined_rules_ack,
            eui_len_ack,
            action_ack,
        }
    }
}

impl Default for JoinFilter {
    fn default() -> JoinFilter {
        JoinFilter::new()
    }
}
