"""Idiomatic secured-session facade.

Two devices that already know each other's public keys can agree on a session key
without ever sending it, and then exchange messages that are confidential, cannot
be altered undetected, and cannot be replayed. That is the whole of what a small
device usually needs from transport security, at a fraction of what a TLS stack
costs it.
"""

from __future__ import annotations

import enum

from pamoja._native import AgreementKey, SealedMessage
from pamoja._native import Session as _Session
from pamoja._native import hkdf_sha256_expand as _hkdf_sha256
from pamoja._native import hmac_sha256_digest as _hmac_sha256

__all__ = [
    "AgreementKey",
    "Role",
    "SealedMessage",
    "Session",
    "hkdf_sha256",
    "hmac_sha256",
]


class Role(str, enum.Enum):
    """Which side of a session a device is on.

    The two devices must choose opposite roles: the role decides the order the
    public keys are mixed in and which direction each side tags its messages
    with, so a session where both sides claim the same role opens nothing.
    """

    #: The device that opens the session.
    INITIATOR = "Initiator"
    #: The device that answers.
    RESPONDER = "Responder"


class Session:
    """A confidential, tamper-evident, replay-protected channel with one peer."""

    def __init__(
        self,
        local: AgreementKey,
        peer_public_key: bytes,
        salt: bytes,
        role: Role,
    ) -> None:
        """Establish a session with a peer.

        :param local: This device's key-agreement secret.
        :param peer_public_key: The peer's 32-byte public key, already
            authenticated by pinning or by a signature.
        :param salt: A fresh per-session salt both sides share, exchanged in the
            clear. Reusing one with the same pair of keys reuses the session key,
            so it must change each session.
        :param role: Whether this device opens the session or answers.
        """
        self._inner = _Session(local, peer_public_key, salt, role.value)

    def seal(self, plaintext: bytes, aad: bytes = b"") -> SealedMessage:
        """Seal a message for the peer.

        :param plaintext: The message to protect.
        :param aad: Data authenticated but not encrypted, so it stays readable on
            the wire yet cannot be altered: a device identifier or a routing
            header belongs here.
        :returns: The ciphertext, with the counter and tag to send beside it.
        """
        return self._inner.seal(plaintext, aad)

    def open(self, sealed: SealedMessage, aad: bytes = b"") -> bytes:
        """Open a message from the peer.

        :param sealed: The ciphertext with the counter and tag that arrived with
            it.
        :param aad: The same associated data the sender authenticated.
        :returns: The plaintext.
        :raises PamojaError: When the counter repeats or is older than the replay
            window still tracks, and when the tag does not authenticate. Nothing
            readable is ever returned from a message that failed either check.
        """
        return self._inner.open(sealed, aad)


def hmac_sha256(key: bytes, message: bytes) -> bytes:
    """Compute a keyed hash over a message.

    This is the primitive a host uses to authenticate a pairing exchange or a
    single command, where a whole session would be more than the job needs.

    :param key: The secret key.
    :param message: The message to authenticate.
    :returns: The 32-byte digest.
    """
    return _hmac_sha256(key, message)


def hkdf_sha256(salt: bytes, ikm: bytes, info: bytes, length: int) -> bytes:
    """Expand input keying material into ``length`` bytes bound to ``info``.

    :param salt: The salt, which may be empty.
    :param ikm: The input keying material.
    :param info: Context binding the output to its purpose.
    :param length: How many bytes to derive.
    :returns: The derived bytes.
    """
    return _hkdf_sha256(salt, ikm, info, length)
