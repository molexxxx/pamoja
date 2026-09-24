"""A profile run as a node: the read, decide, act, and publish loop, and the registry that
finds the code a custom control kind names."""

from __future__ import annotations

import asyncio
import inspect
import json
from dataclasses import dataclass
from typing import Any, Awaitable, Callable, Optional, Protocol, Union

from pamoja._native import PamojaError, Profile

from ._nearest import unresolved

Payload = Union[str, bytes]
Params = dict[str, Union[float, bool, str]]


class Link(Protocol):
    """A link a node publishes its readings over: a ``LoopbackTransport``, an
    ``MqttClient``, a ``CoapClient``, a ``Ladder``, or anything else with the same
    ``send``. Connect it first."""

    def send(self, topic: str, payload: Payload) -> Awaitable[Any]:
        """Publish one message to a topic."""


class Policy(Protocol):
    """What decides each reading: a profile's built-in ``Controller``, or a policy of the
    program's own, which returns a :class:`Decision`."""

    def evaluate(self, reading: float) -> Any:
        """Decide what one reading calls for."""


@dataclass(frozen=True)
class CustomAlert:
    """A condition a policy of the program's own raises, named by a code it chose."""

    #: The condition's name, such as ``FrostRisk``.
    code: str
    #: The measurement behind it.
    value: float
    #: Always ``Custom``, as the built-in alerts name their own kind.
    kind: str = "Custom"


@dataclass(frozen=True)
class Decision:
    """What a policy of the program's own decided about one reading."""

    #: The setting the output should take, or ``None`` to leave it alone.
    actuator: Optional[bool] = None
    #: A condition the reading raised, or ``None``.
    alert: Optional[CustomAlert] = None


@dataclass(frozen=True)
class Tick:
    """One reading a node took, and what its policy decided about it."""

    #: The reading, in the unit the profile reads.
    reading: float
    #: The output setting and the alert the policy decided on.
    reaction: Any


class PolicyRegistry:
    """What resolves a profile's control kind to the code that decides it: a built-in kind
    to its ``Controller``, and a kind the library never shipped to the policy the factory
    registered under its name builds. One program then runs any manifest its registry
    covers."""

    def __init__(self) -> None:
        self._factories: dict[str, Callable[[Params], Policy]] = {}

    def register(self, kind: str, factory: Callable[[Params], Policy]) -> "PolicyRegistry":
        """Register the factory for a custom kind, replacing one of the same name.

        :param kind: The kind as a manifest names it, such as ``frost_guard``.
        :param factory: Builds the policy from the parameters beside the kind.
        :returns: The registry, for chaining.
        """
        self._factories[kind] = factory
        return self

    @property
    def kinds(self) -> list[str]:
        """The custom kinds registered, in name order."""
        return sorted(self._factories)

    def resolve(self, profile: Profile) -> Policy:
        """Resolve a profile's control kind to the policy that decides it.

        :param profile: The profile whose kind is resolved.
        :returns: A fresh policy, ready to evaluate readings.
        :raises PamojaError: When the kind is custom and no factory is registered for it,
            naming the kind and the one it was probably meant to be.
        """
        control = profile.control
        if control.kind != "Custom":
            return profile.controller()
        kind = control.custom_kind or ""
        factory = self._factories.get(kind)
        if factory is None:
            raise PamojaError(unresolved(kind, self.kinds))
        return factory(dict(control.params or {}))


class Node:
    """A profile assembled around the parts that make it run.

    Each :meth:`tick` reads, lets the profile's policy decide, switches the output when the
    policy calls for it, and publishes the reading to the profile's topic. :meth:`run`
    repeats that at the cadence the profile's power schedule sets for the battery's charge.
    """

    def __init__(
        self,
        profile: Profile,
        *,
        read: Callable[[], Union[float, Awaitable[float]]],
        link: Link,
        drive: Optional[Callable[[bool], Any]] = None,
        policy: Union[Policy, PolicyRegistry, None] = None,
        encode: Optional[Callable[[float], Payload]] = None,
    ) -> None:
        """Assemble a node.

        :param profile: The profile the node runs.
        :param read: Takes one reading, in the unit the profile reads.
        :param link: The link each reading is published over, connected before the node
            runs.
        :param drive: Switches the output a profile drives; required for a setpoint
            profile.
        :param policy: What decides each reading: the profile's own controller unless
            given, a policy of the program's own, or a registry that resolves the
            profile's control kind.
        :param encode: Writes a reading as the payload published; the number as JSON text
            unless given.
        :raises PamojaError: When the profile drives an output and no ``drive`` is given,
            or when its control kind is custom and no policy or registry decides it.
        """
        self.profile = profile
        self._read = read
        self._link = link
        self._drive = drive
        self._encode = encode or json.dumps
        if isinstance(policy, PolicyRegistry):
            self._policy = policy.resolve(profile)
        else:
            self._policy = policy if policy is not None else profile.controller()
        if profile.control.kind == "Setpoint" and drive is None:
            raise PamojaError(
                f"the profile `{profile.name}` switches an output, so the node needs `drive`"
            )
        self._plan = profile.power_plan()
        self._mode: Optional[str] = None

    @property
    def power_mode(self) -> Optional[str]:
        """The power mode the last :meth:`schedule` chose, or ``None`` before the first."""
        return self._mode

    async def tick(self) -> Tick:
        """Run one read, decide, act, and publish cycle.

        :returns: The reading and what the policy decided about it.
        :raises Exception: What the reading, the output, or the link raises.
        """
        reading = await _settle(self._read())
        reaction = self._policy.evaluate(reading)
        if reaction.actuator is not None and self._drive is not None:
            await _settle(self._drive(reaction.actuator))
        await self._link.send(self.profile.topic, self._encode(reading))
        return Tick(reading, reaction)

    def schedule(self, charge: float, charging: bool = False) -> tuple[str, float]:
        """Say what power mode the battery's charge puts the node in and how long to wait
        before the next tick.

        The node remembers the mode it chose, so a charge hovering at a threshold keeps the
        slower cadence until it clears the schedule's hysteresis.

        :param charge: The state of charge, from 0 to 1.
        :param charging: Whether the panel is delivering charge.
        :returns: The mode, and the wait in seconds.
        """
        if self._mode is None:
            mode = self._plan.mode_while_charging(charge, charging)
        else:
            mode = self._plan.next_mode_while_charging(self._mode, charge, charging)
        self._mode = mode
        return mode, self._plan.interval_for_us(mode) / 1_000_000

    async def run(
        self,
        *,
        battery: Optional[Callable[[], Any]] = None,
        on_tick: Optional[Callable[[Tick], Any]] = None,
        on_error: Optional[Callable[[BaseException], Any]] = None,
        ticks: Optional[int] = None,
        wait: Optional[Callable[[float], Awaitable[None]]] = None,
    ) -> None:
        """Tick, then wait the interval the battery's charge calls for, until cancelled.

        Cancel the task running it to stop; the tick under way is abandoned where it
        stands.

        :param battery: Reads the battery before each wait, as a charge from 0 to 1 or a
            ``(charge, charging)`` pair; a node without one samples at the active cadence.
        :param on_tick: Hears each tick, such as to log it or to act on an alert.
        :param on_error: Hears a tick that failed. The loop carries on at the same cadence;
            without one, the failure ends the loop and is raised.
        :param ticks: How many ticks to run before returning; the loop runs until cancelled
            unless given.
        :param wait: Waits between ticks, given seconds; :func:`asyncio.sleep` unless given,
            which a test replaces to run at once.
        """
        pause = wait or asyncio.sleep
        count = 0
        while ticks is None or count < ticks:
            try:
                tick = await self.tick()
                if on_tick is not None:
                    await _settle(on_tick(tick))
            except Exception as error:
                if on_error is None:
                    raise
                await _settle(on_error(error))
            charge = 1.0 if battery is None else await _settle(battery())
            charging = False
            if isinstance(charge, tuple):
                charge, charging = charge
            _, seconds = self.schedule(charge, charging)
            count += 1
            if ticks is None or count < ticks:
                await pause(seconds)


async def _settle(value: Any) -> Any:
    """Await a value when it is awaitable, so a callback may be plain or async."""
    if inspect.isawaitable(value):
        return await value
    return value
