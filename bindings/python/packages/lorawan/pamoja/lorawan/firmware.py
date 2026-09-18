"""Firmware management, TS006-1.0.0, on port :data:`PORT`.

What a device is running, what upgrade image it holds and whether that image can be installed,
and the single reboot a device keeps, set either as a moment in time or as a countdown.

>>> from pamoja.lorawan import firmware, package_encode, package_parse
>>> manager = firmware.Firmware(2, 7)
>>> manager.set_image(firmware.ImageStatus.VALID, 3)
>>> asked = package_encode(firmware.PORT, "dev_upgrade_image_req")
>>> package_parse(firmware.PORT, True, manager.heard(asked)).next_version
3
"""

from __future__ import annotations

import enum

from pamoja._native import LORAWAN_FIRMWARE_PORT, LorawanFirmware

__all__ = ["PORT", "Firmware", "ImageStatus"]

#: The port this package is spoken on.
PORT = LORAWAN_FIRMWARE_PORT

#: The firmware management package running on a device.
Firmware = LorawanFirmware


class ImageStatus(str, enum.Enum):
    """What a device makes of the firmware upgrade image it holds, table 10."""

    #: It is holding none.
    NONE = "none"
    #: One is there, but it is corrupt or its signature does not verify.
    CORRUPT = "corrupt"
    #: One is there and authentic, but it is not for this hardware.
    WRONG_HARDWARE = "wrong_hardware"
    #: One is there, and it can be installed.
    VALID = "valid"
