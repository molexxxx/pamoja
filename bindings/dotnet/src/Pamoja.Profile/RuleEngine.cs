using Pamoja.Core;

namespace Pamoja.Profile;

/// <summary>Runs a rule file off a link.</summary>
/// <remarks>
/// <see cref="ListenAsync"/> checks that every output a rule drives was given and
/// subscribes to every watched topic; each <see cref="StepAsync"/> handles one message,
/// switching the outputs by name and publishing over the same link; and
/// <see cref="RunAsync"/> repeats it until cancelled. The engine owns the evaluator it
/// loads from a file's text, and disposing it releases that.
/// </remarks>
public sealed class RuleEngine : IDisposable
{
    private readonly ILink _link;
    private readonly IReadOnlyDictionary<string, Func<bool, ValueTask>> _actuators;
    private readonly Func<TransportMessage, float> _decode;
    private readonly bool _ownsEvaluator;

    /// <summary>Assembles an engine from a rule file's text.</summary>
    /// <param name="rules">The rule file's text.</param>
    /// <param name="link">The link readings arrive on and actions publish over.</param>
    /// <param name="actuators">The outputs the rules drive, each under the name the rule file gives it.</param>
    /// <param name="decode">Reads a message as a reading; its text as a number unless given.</param>
    /// <exception cref="PamojaException">The rule file is refused, with the rule and the reason.</exception>
    public RuleEngine(
        string rules,
        ILink link,
        IReadOnlyDictionary<string, Func<bool, ValueTask>>? actuators = null,
        Func<TransportMessage, float>? decode = null)
        : this(RuleEvaluator.FromJson(rules), link, actuators, decode, ownsEvaluator: true)
    {
    }

    /// <summary>Assembles an engine around an evaluator already loaded, which the caller keeps.</summary>
    /// <param name="rules">The rules, loaded.</param>
    /// <param name="link">The link readings arrive on and actions publish over.</param>
    /// <param name="actuators">The outputs the rules drive, each under the name the rule file gives it.</param>
    /// <param name="decode">Reads a message as a reading; its text as a number unless given.</param>
    public RuleEngine(
        RuleEvaluator rules,
        ILink link,
        IReadOnlyDictionary<string, Func<bool, ValueTask>>? actuators = null,
        Func<TransportMessage, float>? decode = null)
        : this(rules, link, actuators, decode, ownsEvaluator: false)
    {
    }

    private RuleEngine(
        RuleEvaluator rules,
        ILink link,
        IReadOnlyDictionary<string, Func<bool, ValueTask>>? actuators,
        Func<TransportMessage, float>? decode,
        bool ownsEvaluator)
    {
        ArgumentNullException.ThrowIfNull(rules);
        ArgumentNullException.ThrowIfNull(link);
        Evaluator = rules;
        _link = link;
        _actuators = actuators ?? new Dictionary<string, Func<bool, ValueTask>>();
        _decode = decode ?? (message => message.Number is double number ? (float)number : float.NaN);
        _ownsEvaluator = ownsEvaluator;
    }

    /// <summary>Gets the rules, deciding each reading as the engine hands it over.</summary>
    public RuleEvaluator Evaluator { get; }

    /// <summary>Gets the topics the rules watch, each once, in name order.</summary>
    public IReadOnlyList<string> Topics => Evaluator.Topics;

    /// <summary>Gets the outputs the rules drive, each once, in name order.</summary>
    public IReadOnlyList<string> Actuators => Evaluator.Actuators;

    /// <summary>Says whether a rule's condition holds.</summary>
    /// <param name="rule">The rule's name.</param>
    /// <returns>The state, or <c>null</c> for a name no rule has.</returns>
    public bool? IsSet(string rule) => Evaluator.IsSet(rule);

    /// <summary>
    /// Checks the engine was given every output a rule drives, and subscribes to every
    /// watched topic.
    /// </summary>
    /// <returns>A task that completes once the engine is listening.</returns>
    /// <exception cref="PamojaException">A rule drives an output the engine was not given, naming it.</exception>
    public async Task ListenAsync()
    {
        string[] missing = Actuators.Where(name => !_actuators.ContainsKey(name)).ToArray();
        if (missing.Length > 0)
        {
            throw new PamojaException(
                $"a rule drives `{string.Join("`, `", missing)}`, which the engine was not given under `actuators`");
        }

        foreach (string topic in Topics)
        {
            await _link.SubscribeAsync(topic).ConfigureAwait(false);
        }
    }

    /// <summary>Waits a limited time for one message and runs every rule that watches its topic.</summary>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>
    /// What fired, which is empty when the message set or cleared nothing, or <c>null</c>
    /// when no message arrived in time or the link has ended.
    /// </returns>
    /// <exception cref="PamojaException">A reading on a watched topic is not a finite number.</exception>
    public async Task<IReadOnlyList<RuleFired>?> StepAsync(TimeSpan timeout)
    {
        (TransportMessage? message, _) = await NextAsync(timeout).ConfigureAwait(false);
        return message is null ? null : await JudgeAsync(message).ConfigureAwait(false);
    }

    /// <summary>Receives the next message, saying whether the wait ran out when none came.</summary>
    private async Task<(TransportMessage? Message, bool TimedOut)> NextAsync(TimeSpan timeout)
    {
        try
        {
            return (await _link.ReceiveAsync(timeout).ConfigureAwait(false), false);
        }
        catch (TimeoutException)
        {
            return (null, true);
        }
    }

    /// <summary>Judges one message and carries out what the rules that fired call for.</summary>
    private async Task<IReadOnlyList<RuleFired>> JudgeAsync(TransportMessage message)
    {
        if (!Evaluator.Watches(message.Topic))
        {
            return [];
        }

        IReadOnlyList<RuleFired> fired = Evaluator.Evaluate(message.Topic, _decode(message));
        foreach (RuleFired one in fired)
        {
            foreach (RuleAction action in one.Actions)
            {
                if (action.Kind == RuleActionKind.Drive)
                {
                    await _actuators[action.Actuator!](action.On == true).ConfigureAwait(false);
                }
                else
                {
                    await _link.SendAsync(action.Topic!, System.Text.Encoding.UTF8.GetBytes(action.Payload ?? string.Empty))
                        .ConfigureAwait(false);
                }
            }
        }

        return fired;
    }

    /// <summary>Handles messages until cancelled.</summary>
    /// <param name="onFired">Hears each message that set or cleared a rule, with what fired.</param>
    /// <param name="onError">
    /// Hears a message that could not be judged or an action that failed. The loop carries
    /// on with the next message; without one, the failure ends the loop and is thrown.
    /// </param>
    /// <param name="messages">How many messages to handle before returning; until cancelled unless given.</param>
    /// <param name="poll">How long each wait for a message lasts before the token is checked again.</param>
    /// <param name="cancellationToken">Stops the loop between messages.</param>
    /// <returns>A task that completes once cancelled, the messages run out, or the link ends.</returns>
    public async Task RunAsync(
        Func<IReadOnlyList<RuleFired>, ValueTask>? onFired = null,
        Func<Exception, ValueTask>? onError = null,
        int? messages = null,
        TimeSpan? poll = null,
        CancellationToken cancellationToken = default)
    {
        TimeSpan wait = poll ?? TimeSpan.FromMilliseconds(500);
        int handled = 0;
        while ((messages is null || handled < messages) && !cancellationToken.IsCancellationRequested)
        {
            try
            {
                (TransportMessage? message, bool timedOut) = await NextAsync(wait).ConfigureAwait(false);
                if (message is null)
                {
                    if (timedOut)
                    {
                        continue;
                    }

                    return;
                }

                handled++;
                IReadOnlyList<RuleFired> fired = await JudgeAsync(message).ConfigureAwait(false);
                if (fired.Count > 0 && onFired is not null)
                {
                    await onFired(fired).ConfigureAwait(false);
                }
            }
            catch (Exception error) when (onError is not null)
            {
                handled++;
                await onError(error).ConfigureAwait(false);
            }
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_ownsEvaluator)
        {
            Evaluator.Dispose();
        }
    }
}
