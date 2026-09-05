"""Idiomatic device-identity facade.

Wraps the native :class:`pamoja._native.DeviceIdentity` with a Python-native
surface: ``str`` or ``bytes`` payloads, properties instead of getters, and a
named constructor. It adds ergonomics only; the signing and verifying happen in
the Rust core.
"""

from __future__ import annotations

from typing import Union

from pamoja._native import DeviceIdentity as _NativeDeviceIdentity
from pamoja._native import fingerprint as _native_fingerprint
from pamoja._native import verify as _native_verify
from pamoja._native import verify_message as _native_verify_message

__all__ = ["DeviceIdentity", "Payload", "fingerprint", "verify", "verify_message"]

#: A payload to sign or verify; ``str`` is encoded as UTF-8.
Payload = Union[str, bytes, bytearray, memoryview]


def _to_bytes(payload: Payload) -> bytes:
    """Encode a payload to bytes, so callers may pass either text or raw data.

    :param payload: The value to encode.
    :returns: The payload as bytes.
    """
    if isinstance(payload, str):
        return payload.encode("utf-8")
    return bytes(payload)


class DeviceIdentity:
    """A device's private signing identity.

    A reading that drives a health or billing decision has to be provably from
    the device that claims to have sent it, and provably unaltered on the way.
    Sign it here, and any holder of :attr:`public_key` can check it with
    :func:`verify`.

    Example::

        device = DeviceIdentity.from_seed(seed)
        signature = device.sign("21.5")
        verify(device.public_key, "21.5", signature)  # True
    """

    __slots__ = ("_native",)

    def __init__(self, seed: bytes) -> None:
        """Create an identity from a provisioned 32-byte secret seed.

        :param seed: The device's 32-byte secret, held on the device only.
        :raises ValueError: If the seed is not exactly 32 bytes.
        """
        self._native = _NativeDeviceIdentity(bytes(seed))

    @classmethod
    def from_seed(cls, seed: bytes) -> "DeviceIdentity":
        """Create an identity from a provisioned 32-byte secret seed.

        :param seed: The device's 32-byte secret, held on the device only.
        :returns: The identity that seed determines.
        :raises ValueError: If the seed is not exactly 32 bytes.
        """
        return cls(seed)

    @staticmethod
    def native(identity: "DeviceIdentity") -> _NativeDeviceIdentity:
        """Hand the generated identity to another capability facade.

        The audit and update capabilities sign with an identity this class
        holds, and the generated bindings take the generated type. This is how
        the two meet without a caller ever seeing it.

        :param identity: The identity to unwrap.
        :returns: The generated identity inside it.
        """
        return identity._native

    @property
    def public_key(self) -> bytes:
        """The 32-byte public key matching this identity, safe to share."""
        return self._native.public_key

    @property
    def fingerprint(self) -> str:
        """A 16-character lowercase hex label for this identity."""
        return self._native.fingerprint

    def sign(self, payload: Payload) -> bytes:
        """Sign a payload.

        :param payload: The bytes to cover; ``str`` is encoded as UTF-8.
        :returns: The 64-byte detached signature.
        """
        return self._native.sign(_to_bytes(payload))

    def sign_message(self, payload: Payload) -> bytes:
        """Sign a payload and return one message carrying both.

        The message is the signature followed by the payload, which is usually what
        goes on a link: one blob to send, rather than a payload and a detached
        signature to keep together and split correctly at the far end.
        :func:`verify_message` reverses it.

        :param payload: The bytes to cover; ``str`` is encoded as UTF-8.
        :returns: The signature followed by the payload.
        """
        return self._native.sign_message(_to_bytes(payload))

    def __repr__(self) -> str:
        return f"DeviceIdentity(fingerprint={self.fingerprint!r})"


def verify(public_key: bytes, payload: Payload, signature: bytes) -> bool:
    """Verify that a signature covers a payload and was made by a public key.

    :param public_key: The 32-byte public key of the claimed signer.
    :param payload: The bytes the signature should cover.
    :param signature: The 64-byte detached signature.
    :returns: ``True`` if the signature is authentic, and ``False`` if the
        payload was altered or was signed by a different device.
    :raises ValueError: If an argument is not the expected length.
    """
    return _native_verify(bytes(public_key), _to_bytes(payload), bytes(signature))


def verify_message(public_key: bytes, message: bytes) -> bytes | None:
    """Verify a signed message and return the payload it carries.

    :param public_key: The 32-byte public key of the claimed signer.
    :param message: The signature followed by the payload, as
        :meth:`DeviceIdentity.sign_message` wrote it.
    :returns: The payload if the message is authentic, and ``None`` if it is too
        short to hold a signature, was altered, or was signed by a different device.
    :raises ValueError: If the key is not the expected length.
    """
    return _native_verify_message(bytes(public_key), bytes(message))


def fingerprint(public_key: bytes) -> str:
    """Return the short hex fingerprint of a public key.

    :param public_key: The 32-byte public key to label.
    :returns: A 16-character lowercase hex label.
    :raises ValueError: If the key is not exactly 32 bytes.
    """
    return _native_fingerprint(bytes(public_key))
