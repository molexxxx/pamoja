using System.Globalization;
using System.Text;

using Pamoja.Core;
using Pamoja.Power;

namespace Pamoja.Profile;

/// <summary>
/// What decides each reading: a profile's built-in <see cref="Controller"/>, or a policy of
/// the program's own.
/// </summary>
/// <remarks>
/// A policy of the program's own raises its own conditions as an <see cref="Alert"/> of
/// kind <see cref="AlertKind.Custom"/> with a code and a value.
/// </remarks>
public interface IPolicy
{
    /// <summary>Decides what one reading calls for.</summary>
    /// <param name="reading">The reading, in the unit the profile reads.</param>
    /// <returns>The output setting, if any, and the alert the reading raised.</returns>
    Reaction Evaluate(float reading);
}

/// <summary>
/// What resolves a profile's control kind to the code that decides it: a built-in kind to
/// its <see cref="Controller"/>, and a kind the library never shipped to the policy the
/// factory registered under its name builds.
/// </summary>
/// <remarks>One program then runs any manifest its registry covers.</remarks>
public sealed class PolicyRegistry
{
    private readonly SortedDictionary<string, Func<IReadOnlyDictionary<string, object>, IPolicy>> _factories =
        new(StringComparer.Ordinal);

    /// <summary>Gets the custom kinds registered, in name order.</summary>
    public IReadOnlyList<string> Kinds => [.. _factories.Keys];

    /// <summary>Registers the factory for a custom kind, replacing one of the same name.</summary>
    /// <param name="kind">The kind as a manifest names it, such as <c>frost_guard</c>.</param>
    /// <param name="factory">
    /// Builds the policy from the parameters beside the kind, each a <see cref="double"/>,
    /// a <see cref="bool"/>, or a <see cref="string"/>.
    /// </param>
    /// <returns>The registry, for chaining.</returns>
    public PolicyRegistry Register(string kind, Func<IReadOnlyDictionary<string, object>, IPolicy> factory)
    {
        ArgumentNullException.ThrowIfNull(kind);
        ArgumentNullException.ThrowIfNull(factory);
        _factories[kind] = factory;
        return this;
    }

    /// <summary>Resolves a profile's control kind to the policy that decides it.</summary>
    /// <param name="profile">The profile whose kind is resolved.</param>
    /// <returns>
    /// A fresh policy; a built-in kind's <see cref="Controller"/> is the caller's to dispose.
    /// </returns>
    /// <exception cref="PamojaException">
    /// The kind is custom and no factory is registered for it, naming the kind and the one
    /// it was probably meant to be.
    /// </exception>
    public IPolicy Resolve(Profile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ControlPolicy control = profile.Control;
        if (control.Kind != ControlKind.Custom)
        {
            return profile.Controller();
        }

        string kind = control.CustomKind ?? string.Empty;
        return _factories.TryGetValue(kind, out var factory)
            ? factory(control.Params ?? new Dictionary<string, object>())
            : throw new PamojaException(Nearest.Unresolved(kind, Kinds));
    }
}

/// <summary>One reading a node took, and what its policy decided about it.</summary>
/// <param name="Reading">The reading, in the unit the profile reads.</param>
/// <param name="Reaction">The output setting and the alert the policy decided on.</param>
public readonly record struct Tick(float Reading, Reaction Reaction);

/// <summary>A profile assembled around the parts that make it run.</summary>
/// <remarks>
/// Each <see cref="TickAsync"/> reads, lets the profile's policy decide, switches the
/// output when the policy calls for it, and publishes the reading to the profile's topic.
/// <see cref="RunAsync"/> repeats that at the cadence the profile's power schedule sets for
/// the battery's charge. A node owns the controller it built for the profile, and
/// disposing it releases that.
/// </remarks>
public sealed class Node : IDisposable
{
    private readonly Func<ValueTask<float>> _read;
    private readonly ILink _link;
    private readonly Func<bool, ValueTask>? _drive;
    private readonly IPolicy _policy;
    private readonly bool _ownsPolicy;
    private readonly Func<float, byte[]> _encode;
    private readonly PowerPlan _plan;

    /// <summary>Assembles a node.</summary>
    /// <param name="profile">The profile the node runs.</param>
    /// <param name="read">Takes one reading, in the unit the profile reads.</param>
    /// <param name="link">The link each reading is published over, connected before the node runs.</param>
    /// <param name="drive">Switches the output a profile drives; required for a setpoint profile.</param>
    /// <param name="policy">
    /// What decides each reading: the profile's own controller unless given, a policy of
    /// the program's own, or what <see cref="PolicyRegistry.Resolve"/> returned.
    /// </param>
    /// <param name="encode">Writes a reading as the payload published; the number as JSON text unless given.</param>
    /// <exception cref="PamojaException">
    /// The profile drives an output and no <paramref name="drive"/> is given, or its
    /// control kind is custom and no policy decides it.
    /// </exception>
    public Node(
        Profile profile,
        Func<ValueTask<float>> read,
        ILink link,
        Func<bool, ValueTask>? drive = null,
        IPolicy? policy = null,
        Func<float, byte[]>? encode = null)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(read);
        ArgumentNullException.ThrowIfNull(link);
        if (profile.Control.Kind == ControlKind.Setpoint && drive is null)
        {
            throw new PamojaException($"the profile `{profile.Name}` switches an output, so the node needs `drive`");
        }

        Profile = profile;
        _read = read;
        _link = link;
        _drive = drive;
        _ownsPolicy = policy is null;
        _policy = policy ?? profile.Controller();
        _encode = encode ?? (reading => Encoding.UTF8.GetBytes(reading.ToString("R", CultureInfo.InvariantCulture)));
        _plan = profile.PowerPlan;
    }

    /// <summary>Gets the profile the node runs.</summary>
    public Profile Profile { get; }

    /// <summary>Gets the power mode the last <see cref="Schedule"/> chose, or <c>null</c> before the first.</summary>
    public PowerMode? PowerMode { get; private set; }

    /// <summary>Runs one read, decide, act, and publish cycle.</summary>
    /// <returns>The reading and what the policy decided about it.</returns>
    public async Task<Tick> TickAsync()
    {
        float reading = await _read().ConfigureAwait(false);
        Reaction reaction = _policy.Evaluate(reading);
        if (reaction.Actuator is bool on && _drive is not null)
        {
            await _drive(on).ConfigureAwait(false);
        }

        await _link.SendAsync(Profile.Topic, _encode(reading)).ConfigureAwait(false);
        return new Tick(reading, reaction);
    }

    /// <summary>
    /// Says what power mode the battery's charge puts the node in and how long to wait
    /// before the next tick.
    /// </summary>
    /// <remarks>
    /// The node remembers the mode it chose, so a charge hovering at a threshold keeps the
    /// slower cadence until it clears the schedule's hysteresis.
    /// </remarks>
    /// <param name="charge">The state of charge, from 0 to 1.</param>
    /// <param name="charging">Whether the panel is delivering charge.</param>
    /// <returns>The mode, and the wait.</returns>
    public (PowerMode Mode, TimeSpan Wait) Schedule(float charge, bool charging = false)
    {
        PowerMode mode = PowerMode is { } current
            ? _plan.NextModeWhileCharging(current, charge, charging)
            : _plan.ModeWhileCharging(charge, charging);
        PowerMode = mode;
        return (mode, TimeSpan.FromMicroseconds(_plan.IntervalForUs(mode)));
    }

    /// <summary>Ticks, then waits the interval the battery's charge calls for, until cancelled.</summary>
    /// <param name="battery">
    /// Reads the battery before each wait, as a charge from 0 to 1 and whether the panel is
    /// charging; a node without one samples at the active cadence.
    /// </param>
    /// <param name="onTick">Hears each tick, such as to log it or to act on an alert.</param>
    /// <param name="onError">
    /// Hears a tick that failed. The loop carries on at the same cadence; without one, the
    /// failure ends the loop and is thrown.
    /// </param>
    /// <param name="ticks">How many ticks to run before returning; until cancelled unless given.</param>
    /// <param name="wait">Waits between ticks; <see cref="Task.Delay(TimeSpan, CancellationToken)"/> unless given, which a test replaces to run at once.</param>
    /// <param name="cancellationToken">Stops the loop between ticks.</param>
    /// <returns>A task that completes once cancelled or the ticks run out.</returns>
    public async Task RunAsync(
        Func<ValueTask<(float Charge, bool Charging)>>? battery = null,
        Func<Tick, ValueTask>? onTick = null,
        Func<Exception, ValueTask>? onError = null,
        int? ticks = null,
        Func<TimeSpan, CancellationToken, Task>? wait = null,
        CancellationToken cancellationToken = default)
    {
        wait ??= Task.Delay;
        for (int count = 0; ticks is null || count < ticks; count++)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                return;
            }

            try
            {
                Tick tick = await TickAsync().ConfigureAwait(false);
                if (onTick is not null)
                {
                    await onTick(tick).ConfigureAwait(false);
                }
            }
            catch (Exception error) when (onError is not null)
            {
                await onError(error).ConfigureAwait(false);
            }

            (float charge, bool charging) = battery is null ? (1f, false) : await battery().ConfigureAwait(false);
            (_, TimeSpan interval) = Schedule(charge, charging);
            if (ticks is null || count + 1 < ticks)
            {
                try
                {
                    await wait(interval, cancellationToken).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    return;
                }
            }
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_ownsPolicy && _policy is IDisposable owned)
        {
            owned.Dispose();
        }
    }
}
