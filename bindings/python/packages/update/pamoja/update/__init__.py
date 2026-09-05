"""Idiomatic signed-update facade.

A device that cannot be fixed in the field is a device that has to be visited,
and some of them are a day's travel away. Signed updates make that a network
operation instead: a release carries a manifest naming who it is for and what it
hashes to, a device refuses anything not signed by the key it trusts, and an
image that fails to confirm itself is rolled back to the one that worked.
"""

from __future__ import annotations

import enum

from pamoja._native import (
    BootDecision,
    Delegation,
    ImageVerifier,
    Manifest,
    Progress,
    SlotRecord,
    Updater,
    decode_manifest,
    encode_manifest,
    envelope_body,
    open_delegation,
    verify_envelope,
)
from pamoja._native import sign_delegation as _sign_delegation
from pamoja._native import image_digest as _image_digest
from pamoja._native import sign_manifest as _sign_manifest
from pamoja._native import update_format_raw as _format_raw
from pamoja._native import update_structure_version as _structure_version
from pamoja.security import DeviceIdentity

__all__ = [
    "FORMAT_RAW",
    "STRUCTURE_VERSION",
    "BootAction",
    "BootDecision",
    "Delegation",
    "ImageVerifier",
    "Manifest",
    "Progress",
    "SlotRecord",
    "SlotState",
    "Updater",
    "decode_manifest",
    "encode_manifest",
    "envelope_body",
    "open_delegation",
    "sign_delegation",
    "image_digest",
    "sign_manifest",
    "verify_envelope",
]

#: The manifest structure version this build writes.
STRUCTURE_VERSION = _structure_version()

#: The payload format meaning the payload is the image itself, byte for byte.
FORMAT_RAW = _format_raw()


class SlotState(str, enum.Enum):
    """What a device believes about one slot."""

    #: Nothing has been written here.
    EMPTY = "Empty"
    #: An image is arriving, and ``written`` says how much of it has.
    RECEIVING = "Receiving"
    #: A complete image that matched its manifest, not yet tried.
    STAGED = "Staged"
    #: Being tried for the first time; it reverts unless it confirms.
    PENDING = "Pending"
    #: Tried and confirmed working.
    CONFIRMED = "Confirmed"
    #: Tried and did not confirm, so it will not be tried again.
    FAILED = "Failed"


class BootAction(str, enum.Enum):
    """What a bootloader should do with what it found."""

    #: Nothing new to try; run the confirmed image.
    CONFIRMED = "Confirmed"
    #: A staged image is being tried for the first time.
    TRYING = "Trying"
    #: A pending image never confirmed, so it was failed.
    REVERTED = "Reverted"


def image_digest(image: bytes) -> bytes:
    """Hash a complete image, for a publisher filling in a manifest.

    The manifest commits to a SHA-256 over the image, and this is that hash, so a
    publisher does not need a hashing library of its own just to name the image it
    is releasing.

    :param image: The complete image the release carries.
    :returns: The 32-byte digest to put in a :class:`Manifest`.
    """
    return _image_digest(bytes(image))


def sign_manifest(manifest: Manifest, author: DeviceIdentity) -> bytes:
    """Sign a manifest into the envelope that is offered to a device.

    :param manifest: What the release says about itself.
    :param author: The identity signing the release.
    :returns: The signed envelope.
    """
    return _sign_manifest(manifest, DeviceIdentity.native(author))


def sign_delegation(delegation: Delegation, anchor: DeviceIdentity) -> bytes:
    """Sign a delegation, naming a release key the anchor stands behind.

    Keeping the anchor offline and rotating a release key under it is the
    arrangement to prefer, because the key that signs day to day is the one most
    likely to be stolen.

    :param delegation: The statement to sign.
    :param anchor: The anchor identity, which is the root of the trust.
    :returns: The signed delegation envelope.
    """
    return _sign_delegation(delegation, DeviceIdentity.native(anchor))
