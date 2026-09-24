using System.Text.Json;

using Pamoja.Kit;
using Pamoja.Native.Interop;

namespace Pamoja.Profile;

/// <summary>What a rule calls for when its condition sets or clears.</summary>
public enum RuleActionKind
{
    /// <summary>Switch the output the program holds under <see cref="RuleAction.Actuator"/>.</summary>
    Drive = 1,

    /// <summary>Publish <see cref="RuleAction.Payload"/> to <see cref="RuleAction.Topic"/>.</summary>
    Publish = 2,
}

/// <summary>One thing a rule calls for, as the rule file writes it.</summary>
/// <remarks>Only the values belonging to <see cref="Kind"/> are set.</remarks>
/// <param name="Kind">Whether to switch an output or publish a message.</param>
/// <param name="Actuator">The output to switch, for a drive.</param>
/// <param name="On">The setting to switch it to, for a drive.</param>
/// <param name="Topic">The topic to publish to, for a publish.</param>
/// <param name="Payload">The text to publish, for a publish.</param>
public readonly record struct RuleAction(
    RuleActionKind Kind,
    string? Actuator,
    bool? On,
    string? Topic,
    string? Payload);

/// <summary>What one rule did with one reading.</summary>
/// <param name="Rule">The rule's name.</param>
/// <param name="Edge">Whether its condition set or cleared.</param>
/// <param name="Reading">The reading that did it.</param>
/// <param name="Actions">
/// What the rule calls for on this edge: its <c>then</c> actions when it set and its
/// <c>otherwise</c> actions when it cleared, in the order the file gives them.
/// </param>
public sealed record RuleFired(
    string Rule,
    Edge Edge,
    float Reading,
    IReadOnlyList<RuleAction> Actions);

/// <summary>
/// The decisions a rule file makes, reading by reading, with no link and no outputs of
/// its own.
/// </summary>
/// <remarks>
/// The program moves the messages and drives the outputs: it hands each reading over
/// with the topic it arrived on, and learns which rules set or cleared and what each
/// calls for. This is the deciding half of the Rust <c>RuleEngine</c>, which owns a link
/// and outputs and so stays in Rust; the two decide alike.
/// </remarks>
public sealed class RuleEvaluator : IDisposable
{
    private readonly NativeHandle _handle;

    private RuleEvaluator(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_rule_evaluator_free, "rule evaluator");

    /// <summary>Loads a rule file and arms its rules, every condition starting cleared.</summary>
    /// <param name="text">The rule file.</param>
    /// <returns>The evaluator, which the caller disposes.</returns>
    /// <exception cref="PamojaException">
    /// The text is not a rule file, or holds a rule no engine could run, such as one that
    /// watches a filter or has nothing to do.
    /// </exception>
    public static RuleEvaluator FromJson(string text) =>
        new(NativeMethods.pamoja_rule_evaluator_from_json(text));

    /// <summary>Gets the topics the rules watch, each once, in name order: the topics to subscribe to.</summary>
    public IReadOnlyList<string> Topics =>
        _handle.Use(rules => Names(NativeMethods.pamoja_rule_evaluator_topics_json(rules)));

    /// <summary>Gets the actuators the rules drive, each once, in name order: the outputs the program has to have.</summary>
    public IReadOnlyList<string> Actuators =>
        _handle.Use(rules => Names(NativeMethods.pamoja_rule_evaluator_actuators_json(rules)));

    /// <summary>Writes the rules back out as the file a fleet shares.</summary>
    /// <returns>The rule file.</returns>
    public string ToJson() =>
        _handle.Use(rules => OwnedString.Read(NativeMethods.pamoja_rule_evaluator_to_json(rules)));

    /// <summary>Judges one reading from one topic against every rule that watches it.</summary>
    /// <param name="topic">The topic the reading arrived on.</param>
    /// <param name="reading">The reading, already decoded from the message.</param>
    /// <returns>
    /// What fired, in the order the rules are listed; empty when no rule watches the topic
    /// or the reading changed nothing.
    /// </returns>
    /// <exception cref="PamojaException">
    /// A rule watches the topic and the reading is not a finite number.
    /// </exception>
    public IReadOnlyList<RuleFired> Evaluate(string topic, float reading) =>
        _handle.Use(rules =>
        {
            string json = OwnedString.Read(
                NativeMethods.pamoja_rule_evaluator_evaluate(rules, topic, reading));
            using JsonDocument document = JsonDocument.Parse(json);
            var fired = new List<RuleFired>();
            foreach (JsonElement one in document.RootElement.EnumerateArray())
            {
                var actions = new List<RuleAction>();
                foreach (JsonElement action in one.GetProperty("actions").EnumerateArray())
                {
                    actions.Add(ActionOf(action));
                }

                fired.Add(new RuleFired(
                    one.GetProperty("rule").GetString()!,
                    one.GetProperty("edge").GetString() == "set" ? Edge.Set : Edge.Cleared,
                    one.GetProperty("reading").GetSingle(),
                    actions));
            }

            return fired;
        });

    /// <summary>Reports whether any rule watches a topic, so the program knows whether to decode a message.</summary>
    /// <param name="topic">The topic a message arrived on.</param>
    /// <returns><c>true</c> when some rule watches it.</returns>
    public bool Watches(string topic) =>
        _handle.Use(rules => NativeMethods.pamoja_rule_evaluator_watches(rules, topic));

    /// <summary>Reports whether a rule's condition currently holds.</summary>
    /// <param name="rule">The rule's name.</param>
    /// <returns>The condition's state, or <c>null</c> for a name no rule has.</returns>
    public bool? IsSet(string rule) =>
        _handle.Use(rules =>
            NativeMethods.pamoja_rule_evaluator_is_set(rules, rule, out bool set) == PamojaStatus.Ok
                ? set
                : (bool?)null);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads one action as the rule file writes it.</summary>
    private static RuleAction ActionOf(JsonElement action) =>
        action.GetProperty("do").GetString() == "drive"
            ? new RuleAction(
                RuleActionKind.Drive,
                action.GetProperty("actuator").GetString(),
                action.GetProperty("on").GetBoolean(),
                null,
                null)
            : new RuleAction(
                RuleActionKind.Publish,
                null,
                null,
                action.GetProperty("topic").GetString(),
                action.GetProperty("payload").GetString());

    /// <summary>Reads a JSON array of names the native side handed back.</summary>
    private static IReadOnlyList<string> Names(IntPtr json)
    {
        using JsonDocument document = JsonDocument.Parse(OwnedString.Read(json));
        return document.RootElement.EnumerateArray().Select(name => name.GetString()!).ToList();
    }
}
