using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// How a PCA9685's sixteen outputs are wired. Mirrors <c>PamojaPca9685Outputs</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPca9685Outputs
{
    /// <summary>1 for totem-pole outputs, the power-on setting; 0 for open-drain.</summary>
    public byte TotemPole;
    /// <summary>1 to invert the output logic.</summary>
    public byte Inverted;
    /// <summary>1 to change the outputs on each register write's acknowledge.</summary>
    public byte ChangeOnAck;
}

/// <summary>
/// How a PCA9685 driver programs the part. Mirrors <c>PamojaPca9685Settings</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaPca9685Settings
{
    /// <summary>The PWM frequency every channel shares, in hertz.</summary>
    public uint FrequencyHz;
    /// <summary>The clock the prescaler divides, in hertz.</summary>
    public uint OscillatorHz;
    /// <summary>How the outputs are wired.</summary>
    public PamojaPca9685Outputs Outputs;
}
