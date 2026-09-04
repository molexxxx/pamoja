using Pamoja.Codec;
using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Sim;

/// <summary>Where a robot is and which way it faces.</summary>
/// <param name="X">Position along the world x axis, in metres.</param>
/// <param name="Y">Position along the world y axis, in metres.</param>
/// <param name="Theta">Heading from the world x axis, in radians.</param>
public readonly record struct Pose(float X, float Y, float Theta);

/// <summary>How fast a robot is asked to move.</summary>
/// <param name="Vx">Forward speed along the x axis.</param>
/// <param name="Vy">Leftward speed along the y axis.</param>
/// <param name="Omega">Yaw rate about the z axis, positive counter-clockwise.</param>
public readonly record struct Twist(float Vx, float Vy = 0.0f, float Omega = 0.0f);

/// <summary>A sensor that invents plausible readings.</summary>
/// <remarks>
/// Simulated devices matter more from a binding than from Rust: someone writing
/// against the SDK in C# can put a sensor, an actuator, and a lossy link into a
/// unit test and find out what their code does when a reading drifts or a packet
/// vanishes, without owning the device.
/// </remarks>
public sealed class SimulatedSensor : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a sensor that reads around a baseline.</summary>
    /// <param name="baseline">The value it reads before drift and noise.</param>
    /// <param name="driftPerRead">How much the baseline moves each read.</param>
    /// <param name="noise">The amplitude of the wobble around it.</param>
    /// <param name="seed">The seed for that wobble, so a run repeats.</param>
    /// <exception cref="PamojaException">The native sensor could not be created.</exception>
    public SimulatedSensor(
        float baseline,
        float driftPerRead = 0.0f,
        float noise = 0.0f,
        uint seed = 0)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_sim_sensor_new(baseline, driftPerRead, noise, seed),
            NativeMethods.pamoja_sim_sensor_free,
            "simulated sensor");
    }

    /// <summary>Takes the next reading.</summary>
    /// <returns>The reading.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<float> ReadAsync() => Task.Run(() =>
    {
        float reading = 0.0f;
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_sim_sensor_read(handle, out reading)));
        return reading;
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A sensor that reads back a recorded series.</summary>
/// <remarks>
/// This is how a caller replays a real capture, so a test asks what the code does
/// with readings that actually happened rather than ones it invented.
/// </remarks>
public sealed class Replay : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a replay over a recorded series.</summary>
    /// <param name="readings">The series to read back.</param>
    /// <param name="repeating">
    /// Start again at the beginning once exhausted, rather than holding the last
    /// reading.
    /// </param>
    /// <exception cref="PamojaException">The native replay could not be created.</exception>
    public Replay(ReadOnlySpan<float> readings, bool repeating = false)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_replay_new(readings, (nuint)readings.Length, repeating),
            NativeMethods.pamoja_replay_free,
            "replay");
    }

    /// <summary>Takes the next reading.</summary>
    /// <returns>The reading.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<float> ReadAsync() => Task.Run(() =>
    {
        float reading = 0.0f;
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_replay_read(handle, out reading)));
        return reading;
    });

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>An actuator that records every command instead of acting on one.</summary>
public sealed class RecordingActuator : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an actuator with nothing recorded yet.</summary>
    /// <exception cref="PamojaException">The native actuator could not be created.</exception>
    public RecordingActuator()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_recording_actuator_new(),
            NativeMethods.pamoja_recording_actuator_free,
            "recording actuator");
    }

    /// <summary>Gets how many commands have been recorded.</summary>
    public int Count =>
        checked((int)_handle.Use(NativeMethods.pamoja_recording_actuator_len));

    /// <summary>Gets the commands recorded so far, oldest first.</summary>
    public float[] Commands
    {
        get
        {
            float[] commands = new float[Count];
            if (commands.Length == 0)
            {
                return commands;
            }

            _handle.Use(handle => NativeMethods.pamoja_recording_actuator_commands(
                handle, commands, (nuint)commands.Length));
            return commands;
        }
    }

    /// <summary>Applies a command, which is recorded rather than acted on.</summary>
    /// <param name="command">The value commanded.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task ApplyAsync(float command) => Task.Run(() => PamojaCore.ThrowIfError(
        _handle.Use(handle =>
            NativeMethods.pamoja_recording_actuator_apply(handle, command))));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A robot that moves only in arithmetic.</summary>
public sealed class SimulatedRobot : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a robot at a starting pose.</summary>
    /// <param name="dt">The seconds each command advances it.</param>
    /// <param name="start">Where it begins, defaulting to the origin.</param>
    /// <exception cref="PamojaException">The native robot could not be created.</exception>
    public SimulatedRobot(float dt, Pose start = default)
    {
        PamojaPose pose = new() { X = start.X, Y = start.Y, Theta = start.Theta };
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_sim_robot_new(pose, dt),
            NativeMethods.pamoja_sim_robot_free,
            "simulated robot");
    }

    /// <summary>Gets where the robot has got to.</summary>
    public Pose Pose
    {
        get
        {
            PamojaPose pose = _handle.Use(NativeMethods.pamoja_sim_robot_pose);
            return new Pose(pose.X, pose.Y, pose.Theta);
        }
    }

    /// <summary>Drives the robot for one time step.</summary>
    /// <param name="command">The speeds to hold for one step.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task ApplyAsync(Twist command)
    {
        PamojaTwist twist = new()
        {
            Vx = command.Vx,
            Vy = command.Vy,
            Omega = command.Omega,
        };
        return Task.Run(() => PamojaCore.ThrowIfError(
            _handle.Use(handle => NativeMethods.pamoja_sim_robot_apply(handle, twist))));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
