"""The Zenoh key expression guide example; see docs/guides/zenoh.md."""

# ANCHOR: example
from pamoja.zenoh import canonize, is_canon, is_valid, matches

# A key expression names a set of keys. `*` stands for exactly one chunk, so this selects
# the battery of any node, and not a battery nested deeper.
any_node = "fleet/*/battery"
for key in ("fleet/n7/battery", "fleet/n7/rack/battery"):
    print(f"{any_node} covers {key}: {matches(any_node, key)}")

# `**` stands for any number of chunks, including none, which is what a subscription
# covering a whole subtree wants.
print(f"fleet/** covers a nested key: {matches('fleet/**', 'fleet/n7/rack/battery')}")
print(f"fleet/**/battery covers fleet/battery: {matches('fleet/**/battery', 'fleet/battery')}")

# Two expressions that select the same keys have one canonical form. Comparing or routing
# on the written form would treat these as different subscriptions.
written = "fleet/**/**/battery"
canonical = canonize(written)
print(f"{written} is canonical: {is_canon(written)}, and canonizes to {canonical}")

# A malformed expression is rejected rather than canonized into something plausible.
malformed = "fleet//battery"
print(f"{malformed} is valid: {is_valid(malformed)}, canonizes to {canonize(malformed)}")
# ANCHOR_END: example

assert is_valid(any_node)
assert matches(any_node, "fleet/n7/battery")
assert not matches(any_node, "fleet/n7/rack/battery")
assert matches("fleet/**", "fleet/n7/rack/battery")
assert matches("fleet/**/battery", "fleet/battery")
assert not is_canon(written)
assert canonical == "fleet/**/battery"
assert is_canon("fleet/**/battery")
assert not is_valid(malformed)
assert canonize(malformed) is None
