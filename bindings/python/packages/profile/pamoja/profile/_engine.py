"""A rule file run off a link: every reading that arrives on a watched topic is judged, and
the drives and publishes a set or cleared condition calls for are carried out."""

from __future__ import annotations

import asyncio
from typing import Any, Awaitable, Callable, Optional, Protocol, Union

from pamoja._native import PamojaError, RuleEvaluator, RuleFired

from ._node import Link, _settle


class ReceivingLink(Link, Protocol):
    """A link a rule engine listens on as well as publishes over: a ``LoopbackTransport``,
    an ``MqttClient``, a ``CoapClient``, or anything else with the same three calls.
    Connect it first."""

    def subscribe(self, topic: str) -> Awaitable[Any]:
        """Subscribe to one topic."""

    def recv(self) -> Awaitable[Any]:
        """Wait for the next message, with its ``topic``, ``text``, and ``number``."""


class RuleEngine:
    """Runs a rule file off a link.

    :meth:`listen` checks that every output a rule drives was given and subscribes to every
    watched topic; each :meth:`step` handles one message, switching the outputs by name and
    publishing over the same link; :meth:`run` repeats it until cancelled.
    """

    def __init__(
        self,
        rules: Union[str, RuleEvaluator],
        link: ReceivingLink,
        *,
        actuators: Optional[dict[str, Callable[[bool], Any]]] = None,
        decode: Optional[Callable[[Any], float]] = None,
    ) -> None:
        """Assemble an engine.

        :param rules: The rule file's text, or an evaluator already loaded from it.
        :param link: The link readings arrive on and actions publish over.
        :param actuators: The outputs the rules drive, each under the name the rule file
            gives it.
        :param decode: Reads a message as a reading; its ``number`` unless given, which
            raises ``ValueError`` for a payload that is not a number written out.
        :raises PamojaError: When the rule file is refused, with the rule and the reason.
        """
        self.evaluator = RuleEvaluator.from_json(rules) if isinstance(rules, str) else rules
        self._link = link
        self._actuators = dict(actuators or {})
        self._decode = decode or (lambda message: message.number)

    @property
    def topics(self) -> list[str]:
        """The topics the rules watch, each once, in name order."""
        return self.evaluator.topics

    @property
    def actuators(self) -> list[str]:
        """The outputs the rules drive, each once, in name order."""
        return self.evaluator.actuators

    def is_set(self, rule: str) -> Optional[bool]:
        """Whether a rule's condition holds.

        :param rule: The rule's name.
        :returns: The state, or ``None`` for a name no rule has.
        """
        return self.evaluator.is_set(rule)

    async def listen(self) -> None:
        """Check the engine was given every output a rule drives, and subscribe to every
        watched topic.

        :raises PamojaError: When a rule drives an output the engine was not given,
            naming it.
        """
        missing = [name for name in self.actuators if name not in self._actuators]
        if missing:
            raise PamojaError(
                "a rule drives `"
                + "`, `".join(missing)
                + "`, which the engine was not given under `actuators`"
            )
        for topic in self.topics:
            await self._link.subscribe(topic)

    async def step(self, timeout: Optional[float] = None) -> Optional[list[RuleFired]]:
        """Wait for one message and run every rule that watches its topic.

        :param timeout: How long to wait, in seconds; for good unless given.
        :returns: What fired, which is empty when the message set or cleared nothing, or
            ``None`` when no message arrived in time or the link has ended.
        :raises PamojaError: When a reading on a watched topic is not a finite number.
        :raises Exception: What decoding or an action raises.
        """
        try:
            message = await asyncio.wait_for(self._link.recv(), timeout)
        except asyncio.TimeoutError:
            return None
        if message is None:
            return None
        if not self.evaluator.watches(message.topic):
            return []
        fired = self.evaluator.evaluate(message.topic, self._decode(message))
        for one in fired:
            for action in one.actions:
                if action.kind == "drive":
                    await _settle(self._actuators[action.actuator](bool(action.on)))
                else:
                    await self._link.send(action.topic, action.payload)
        return fired

    async def run(
        self,
        *,
        on_fired: Optional[Callable[[list[RuleFired]], Any]] = None,
        on_error: Optional[Callable[[BaseException], Any]] = None,
        messages: Optional[int] = None,
    ) -> None:
        """Handle messages until cancelled.

        :param on_fired: Hears each message that set or cleared a rule, with what fired.
        :param on_error: Hears a message that could not be judged or an action that failed.
            The loop carries on with the next message; without one, the failure ends the
            loop and is raised.
        :param messages: How many messages to handle before returning; the loop runs until
            cancelled or the link ends unless given.
        """
        handled = 0
        while messages is None or handled < messages:
            try:
                fired = await self.step()
                if fired is None:
                    return
                handled += 1
                if fired and on_fired is not None:
                    await _settle(on_fired(fired))
            except Exception as error:
                handled += 1
                if on_error is None:
                    raise
                await _settle(on_error(error))
