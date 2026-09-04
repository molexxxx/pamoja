"""The Zenoh keys guide example; see docs/guides/zenoh.md."""

# ANCHOR: example
from pamoja.zenoh import canonize, is_canon, is_valid, matches

# A key expression names a set of keys. `*` stands for exactly one chunk, so this
# selects the battery of any node, and not a battery nested deeper.
assert is_valid("fleet/*/battery")
assert matches("fleet/*/battery", "fleet/n7/battery")
assert not matches("fleet/*/battery", "fleet/n7/rack/battery")

# `**` stands for any number of chunks, including none, which is what a subscription
# covering a whole subtree wants.
assert matches("fleet/**", "fleet/n7/rack/battery")
assert matches("fleet/**/battery", "fleet/battery")

# Two expressions that select the same keys have one canonical form. Comparing or
# routing on the written form would treat these as different subscriptions.
assert not is_canon("fleet/**/**/battery")
assert canonize("fleet/**/**/battery") == "fleet/**/battery"
assert is_canon("fleet/**/battery")

# A malformed expression is rejected rather than canonized into something plausible.
assert not is_valid("fleet//battery")
assert canonize("fleet//battery") is None
# ANCHOR_END: example
