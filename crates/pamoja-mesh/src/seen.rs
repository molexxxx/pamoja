//! Duplicate suppression for flooded packets.

/// A fixed-size memory of the most recently seen packets, so a node relays each one once.
///
/// In a flood every node rebroadcasts what it hears, so the same packet reaches a node
/// from several neighbours. Without a memory of what it has already handled, a node would
/// relay every copy and the flood would multiply without bound. This cache remembers the
/// last `N` packet keys (a [`dedup_key`](crate::Frame::dedup_key), the source and sequence
/// id) in a ring, evicting the oldest as new ones arrive, so the test for "have I seen
/// this?" stays cheap and needs no allocation. `N` sets how far back the memory reaches;
/// a small power of two such as 32 or 64 suits a local mesh.
///
/// # Examples
///
/// ```
/// use pamoja_mesh::SeenCache;
///
/// let mut seen: SeenCache<8> = SeenCache::new();
/// assert!(seen.record((0x42, 1)));  // first time: newly recorded
/// assert!(!seen.record((0x42, 1))); // again: a duplicate
/// assert!(seen.record((0x42, 2)));  // a different packet
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SeenCache<const N: usize> {
    keys: [Option<(u32, u16)>; N],
    next: usize,
}

impl<const N: usize> SeenCache<N> {
    /// Creates an empty cache.
    ///
    /// # Returns
    ///
    /// A cache holding no keys.
    pub const fn new() -> Self {
        SeenCache {
            keys: [None; N],
            next: 0,
        }
    }

    /// Reports whether a key is currently remembered.
    ///
    /// # Arguments
    ///
    /// * `key` - the packet key to look for, from [`dedup_key`](crate::Frame::dedup_key).
    ///
    /// # Returns
    ///
    /// `true` if the key is in the cache.
    pub fn contains(&self, key: (u32, u16)) -> bool {
        self.keys.contains(&Some(key))
    }

    /// Records a key, reporting whether it was new.
    ///
    /// This is the flood test: record the key of a received packet, and act on the packet
    /// only when this returns `true`. The oldest remembered key is evicted once the cache
    /// is full.
    ///
    /// # Arguments
    ///
    /// * `key` - the packet key to record, from [`dedup_key`](crate::Frame::dedup_key).
    ///
    /// # Returns
    ///
    /// `true` if the key was not already remembered (the packet is new), `false` if it was
    /// (the packet is a duplicate).
    pub fn record(&mut self, key: (u32, u16)) -> bool {
        record_into(&mut self.keys, &mut self.next, key)
    }
}

impl<const N: usize> Default for SeenCache<N> {
    fn default() -> Self {
        Self::new()
    }
}

// Records a key into a ring of slots, reporting whether it was new. Split out so the
// fixed-size and runtime-sized caches share one implementation rather than two that can
// drift.
fn record_into(keys: &mut [Option<(u32, u16)>], next: &mut usize, key: (u32, u16)) -> bool {
    if keys.contains(&Some(key)) {
        return false;
    }
    // A zero-capacity cache remembers nothing, so every key reads as new and every packet
    // is relayed, which is the behaviour of a node with no cache at all.
    if keys.is_empty() {
        return true;
    }
    keys[*next] = Some(key);
    *next = (*next + 1) % keys.len();
    true
}

/// A duplicate cache whose size is chosen when it is built, rather than at compile time.
///
/// [`SeenCache`] fixes its capacity in the type, which suits a microcontroller that knows
/// its own limits. A gateway, or any caller reaching this through a language binding, does
/// not know the size until it runs, and a const generic cannot cross a foreign function
/// boundary at all. This is the same cache with its slots on the heap, so both share one
/// implementation and answer identically.
///
/// Requires the `alloc` feature.
///
/// # Examples
///
/// ```
/// use pamoja_mesh::DynamicSeenCache;
///
/// // A busy relay remembers more packets than a leaf node needs to.
/// let mut seen = DynamicSeenCache::new(1024);
/// assert!(seen.record((0x42, 1)));
/// assert!(!seen.record((0x42, 1)));
/// assert_eq!(seen.capacity(), 1024);
/// ```
#[cfg(any(feature = "alloc", test))]
#[derive(Clone, Debug)]
pub struct DynamicSeenCache {
    keys: alloc::vec::Vec<Option<(u32, u16)>>,
    next: usize,
}

#[cfg(any(feature = "alloc", test))]
impl DynamicSeenCache {
    /// Creates an empty cache remembering up to `capacity` packets.
    ///
    /// # Arguments
    ///
    /// * `capacity` - how many recently seen packets to remember. A capacity of zero is
    ///   allowed and makes every packet read as new, which relays every copy.
    ///
    /// # Returns
    ///
    /// A cache holding no keys.
    pub fn new(capacity: usize) -> Self {
        DynamicSeenCache {
            keys: alloc::vec![None; capacity],
            next: 0,
        }
    }

    /// Returns how many packets this cache remembers.
    ///
    /// # Returns
    ///
    /// The capacity it was created with.
    pub fn capacity(&self) -> usize {
        self.keys.len()
    }

    /// Reports whether a key is currently remembered.
    ///
    /// # Arguments
    ///
    /// * `key` - the packet key to look for, from [`dedup_key`](crate::Frame::dedup_key).
    ///
    /// # Returns
    ///
    /// `true` if the key is in the cache.
    pub fn contains(&self, key: (u32, u16)) -> bool {
        self.keys.contains(&Some(key))
    }

    /// Records a key, reporting whether it was new.
    ///
    /// # Arguments
    ///
    /// * `key` - the packet key to record, from [`dedup_key`](crate::Frame::dedup_key).
    ///
    /// # Returns
    ///
    /// `true` if the packet is new, `false` if it is a duplicate.
    pub fn record(&mut self, key: (u32, u16)) -> bool {
        record_into(&mut self.keys, &mut self.next, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_new_once_then_a_duplicate() {
        let mut seen: SeenCache<8> = SeenCache::new();
        assert!(seen.record((1, 1)));
        assert!(!seen.record((1, 1)));
        assert!(seen.contains((1, 1)));
    }

    #[test]
    fn different_sources_and_ids_are_distinct() {
        let mut seen: SeenCache<8> = SeenCache::new();
        assert!(seen.record((1, 1)));
        assert!(seen.record((1, 2)));
        assert!(seen.record((2, 1)));
        assert!(!seen.record((1, 1)));
    }

    #[test]
    fn the_oldest_key_is_evicted_when_full() {
        let mut seen: SeenCache<2> = SeenCache::new();
        assert!(seen.record((0, 1)));
        assert!(seen.record((0, 2)));
        // Recording a third key evicts the oldest, (0, 1).
        assert!(seen.record((0, 3)));
        assert!(!seen.contains((0, 1)));
        assert!(seen.contains((0, 2)));
        assert!(seen.contains((0, 3)));
        // The evicted key is treated as new again.
        assert!(seen.record((0, 1)));
    }

    #[test]
    fn an_empty_cache_remembers_nothing() {
        let seen: SeenCache<4> = SeenCache::default();
        assert!(!seen.contains((1, 1)));
    }

    #[test]
    fn a_zero_capacity_cache_reads_every_key_as_new_without_panicking() {
        let mut seen: SeenCache<0> = SeenCache::new();
        assert!(seen.record((1, 1)));
        assert!(seen.record((1, 1))); // nothing was remembered, so it is new again
        assert!(!seen.contains((1, 1)));
    }

    #[test]
    fn a_runtime_sized_cache_answers_the_same_way() {
        let mut fixed: SeenCache<4> = SeenCache::new();
        let mut dynamic = DynamicSeenCache::new(4);
        for key in [(1u32, 1u16), (1, 1), (1, 2), (2, 1), (1, 3), (1, 4), (1, 1)] {
            assert_eq!(
                fixed.record(key),
                dynamic.record(key),
                "the two caches evict identically"
            );
            assert_eq!(fixed.contains(key), dynamic.contains(key));
        }
    }

    #[test]
    fn a_runtime_sized_cache_remembers_what_it_was_sized_for() {
        let mut seen = DynamicSeenCache::new(2);
        assert_eq!(seen.capacity(), 2);
        assert!(seen.record((1, 1)));
        assert!(seen.record((1, 2)));
        assert!(seen.record((1, 3)));
        assert!(
            !seen.contains((1, 1)),
            "the oldest key is evicted once the cache is full"
        );
    }

    #[test]
    fn a_cache_with_no_room_relays_every_copy() {
        let mut seen = DynamicSeenCache::new(0);
        assert!(seen.record((1, 1)));
        assert!(
            seen.record((1, 1)),
            "with nothing remembered every copy is new"
        );
    }
}
