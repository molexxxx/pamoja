"""The Zenoh key expression guide example; see docs/guides/zenoh.md."""

# ANCHOR: example
from pamoja.zenoh import canonize, is_canon, is_valid, join, matches


# A key expression names a set of keys. Chunks sit between slashes, `*` stands for exactly one
# chunk, whatever it holds, and `**` for any number of them, including none.
def selects(pattern: str, key: str) -> str:
    verdict = "covers" if matches(pattern, key) else "misses"
    return f"{verdict:<10}{pattern} {verdict} {key}"


print(selects("farm/*/power", "farm/t7/power"))
print(selects("farm/*/power", "farm/substation/power"))
print(f"{selects('farm/*/power', 'farm/row2/t14/power')}, since * is exactly one chunk")
print(selects("farm/**/power", "farm/row2/t14/power"))
print(f"{selects('farm/**/alarm', 'farm/alarm')}, where ** is no chunk at all")

# `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
print(selects("farm/t$*/power", "farm/t7/power"))
print(selects("farm/t$*/power", "farm/substation/power"))

# One set of keys has one canonical spelling, and a Zenoh session accepts no other.
for written in ("farm/*/**/power", "farm/**/*/power", "farm/**/**/power"):
    canonical = canonize(written)
    if is_canon(written):
        print(f"canonical {written}, as written")
    else:
        print(f"rewritten {written} is spelled {canonical}")

# Joining places one expression beneath another, and canonizes the seam between them.
for prefix, suffix in (("farm/t7", "power"), ("farm/**", "*/power")):
    print(f"joined    {prefix} and {suffix} make {join(prefix, suffix)}")

# A malformed expression is refused rather than repaired into something plausible.
for written, why in (
    ("farm//power", "a chunk is empty"),
    ("farm/t7*/power", "* stands alone in its chunk, or after $"),
    ("farm/t7/power?", "? and # are reserved"),
):
    if not is_valid(written) and canonize(written) is None:
        print(f"malformed {written}, since {why}")
# ANCHOR_END: example

assert matches("farm/*/power", "farm/substation/power")
assert not matches("farm/*/power", "farm/row2/t14/power")
assert matches("farm/**/alarm", "farm/alarm")
assert not matches("farm/t$*/power", "farm/substation/power")
assert canonize("farm/**/*/power") == "farm/*/**/power"
assert join("farm/**", "*/power") == "farm/*/**/power"

# ANCHOR: relations
from pamoja.zenoh import includes, intersects


# Two expressions intersect when some key belongs to both. That is the question a router asks
# before it forwards a publication on one to a subscriber on the other.
def overlap(a: str, b: str) -> str:
    if intersects(a, b):
        return f"overlap   {a} and {b} share a key"
    return f"disjoint  {a} and {b} share no key"


print(overlap("farm/*/power", "farm/t7/**"))
print(overlap("farm/*/power", "farm/*/alarm"))


# One includes the other when every key of the second belongs to the first, so a bridge that
# already holds the wider subscription declares nothing new for the narrower one.
def covers(a: str, b: str) -> str:
    if includes(a, b):
        return f"included  {a} covers every key of {b}"
    return f"wider     {b} holds keys {a} does not"


print(covers("farm/**", "farm/*/power"))
print(covers("farm/*/power", "farm/**"))

# Two spellings of one set include each other, which compares expressions nobody canonized.
one, other = "farm/**/*/power", "farm/*/**/power"
if one != other and includes(one, other) and includes(other, one):
    print(f"same      {one} and {other} select the same keys")
# ANCHOR_END: relations

assert intersects("farm/*/power", "farm/t7/**")
assert not intersects("farm/*/power", "farm/*/alarm")
assert includes("farm/**", "farm/*/power")
assert not includes("farm/*/power", "farm/**")

# ANCHOR: sealed
# A chunk that starts with @ is verbatim: no wildcard selects it, and only the same chunk
# matches it. A second payload version under @v2 stays out of every subscription that does not
# name it, the way Zenoh keeps its own administration space out of **.
current = "farm/@v2/t7/power"
print(f"{selects('farm/**', current)}, since no wildcard selects a chunk that starts with @")
print(selects("farm/@v2/**", current))
print(f"{overlap('farm/@v1/**', 'farm/@v2/**')}, so a reader of one version never sees the other")
# ANCHOR_END: sealed

assert not matches("farm/**", current)
assert matches("farm/@v2/**", current)
assert not intersects("farm/@v1/**", "farm/@v2/**")
