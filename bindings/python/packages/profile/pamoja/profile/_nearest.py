"""The name a misspelled one was probably meant to be, worked out the way the library's own
parser does, so a refusal reads the same in every language."""

from __future__ import annotations

from typing import Iterable, Optional

#: The control kinds the library ships, as a manifest names them.
BUILT_IN_KINDS = ("setpoint", "level", "surge", "monitor")


def nearest(given: str, allowed: Iterable[str]) -> Optional[str]:
    """Pick the allowed name a given one is most likely a misspelling of.

    :param given: The name the file or the program used.
    :param allowed: The names it may use there, in the order to prefer them.
    :returns: The closest allowed name, or ``None`` when none is within two edits and
        closer than half the given name's length.
    """
    limit = min(2, max(1, len(given) // 2))
    best: Optional[str] = None
    fewest: Optional[int] = None
    for name in allowed:
        edits = _distance(given, name)
        if edits <= limit and (fewest is None or edits < fewest):
            best, fewest = name, edits
    return best


def unresolved(kind: str, registered: list[str]) -> str:
    """The reason a control kind with no policy behind it is refused.

    :param kind: The custom kind the profile names.
    :param registered: The kinds a registry knows, in name order.
    :returns: The reason, worded as the library words it.
    """
    near = nearest(kind, [*BUILT_IN_KINDS, *registered])
    if near is not None:
        hint = f"; did you mean `{near}`?"
    elif not registered:
        hint = "; resolve the profile through a PolicyRegistry that registers it"
    else:
        hint = "; the registry knows `" + "`, `".join(registered) + "`"
    return f"codec error: no policy decides the control kind `{kind}`, which is not built in{hint}"


def _distance(a: str, b: str) -> int:
    previous = list(range(len(b) + 1))
    for i, left in enumerate(a):
        current = [i + 1]
        for j, right in enumerate(b):
            substitute = previous[j] + (left != right)
            current.append(min(substitute, previous[j + 1] + 1, current[j] + 1))
        previous = current
    return previous[len(b)]
