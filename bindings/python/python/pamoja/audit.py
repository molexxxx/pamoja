"""Idiomatic audit-log facade.

A log that can be edited after the fact proves nothing. Each record here is
signed and carries the hash of the one before it, so altering a record, dropping
one, or reordering two breaks the chain at that point and at every point after
it. The index is part of what is signed, which is what makes a record removed
from the end detectable too.
"""

from __future__ import annotations

from collections.abc import Sequence

from ._core import AuditEntry, AuditVerifier
from ._core import AuditLog as _AuditLog
from ._core import verify_audit_chain as _verify_audit_chain
from .security import DeviceIdentity

__all__ = [
    "AuditEntry",
    "AuditLog",
    "AuditVerifier",
    "verify_chain",
]


class AuditLog:
    """A log that signs what it is given and chains it onto what came before."""

    __slots__ = ("_inner",)

    def __init__(self, identity: DeviceIdentity, last: AuditEntry | None = None) -> None:
        """Create a log that signs with a device identity.

        :param identity: The identity whose signature each record will carry.
        :param last: The final record of an existing log to carry on from, or
            ``None`` to start empty.
        """
        signer = DeviceIdentity.native(identity)
        self._inner = (
            _AuditLog(signer) if last is None else _AuditLog.resume(signer, last)
        )

    @staticmethod
    def resume(identity: DeviceIdentity, last: AuditEntry) -> "AuditLog":
        """Create a log that carries on from the last record an earlier one wrote.

        This is what a device does after a restart: the chain continues at the
        next index and hashes onto the record it left off at, so a reboot leaves
        no gap for a record to be removed through.

        :param identity: The identity whose signature each record will carry.
        :param last: The final record of the existing log.
        :returns: The log, positioned after ``last``.
        """
        return AuditLog(identity, last)

    def append(self, payload: bytes) -> AuditEntry:
        """Append a payload, signing it and chaining it onto the last record.

        :param payload: The record to store.
        :returns: The new entry.
        """
        return self._inner.append(payload)


def verify_chain(public_key: bytes, entries: Sequence[AuditEntry]) -> bool:
    """Check a whole chain that has already arrived.

    :param public_key: The 32-byte key the records were signed with.
    :param entries: The records, in the order they were written.
    :returns: ``True`` when every record follows the one before it and carries a
        signature that holds.
    """
    return _verify_audit_chain(public_key, list(entries))
