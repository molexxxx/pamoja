using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>The bytes of one SX126x command, carried inline inside a blittable struct.</summary>
[InlineArray(Length)]
public struct PamojaSx126xBytes
{
    /// <summary>The room a command has, in bytes.</summary>
    public const int Length = NativeMethods.Sx126xCommandMax;

    private byte _element0;
}

/// <summary>
/// One SX126x command or command header, mirroring <c>PamojaSx126xCommand</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xCommand
{
    /// <summary>The opcode and its parameters; only the first <see cref="Len"/> are used.</summary>
    public PamojaSx126xBytes Bytes;

    /// <summary>How many of <see cref="Bytes"/> the command uses.</summary>
    public byte Len;

    /// <summary>Copies the bytes the command uses out as an array.</summary>
    /// <returns>The opcode followed by its parameters.</returns>
    public readonly byte[] ToArray()
    {
        PamojaSx126xBytes copy = Bytes;
        return ((ReadOnlySpan<byte>)copy)[..Math.Min((int)Len, PamojaSx126xBytes.Length)].ToArray();
    }
}

/// <summary>
/// An SX126x command that reads data back, mirroring <c>PamojaSx126xQuery</c> in
/// <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xQuery
{
    /// <summary>The bytes to clock out before the answer, opcode first.</summary>
    public PamojaSx126xCommand Command;

    /// <summary>How many bytes of answer follow the command in the same transaction.</summary>
    public uint AnswerLen;
}

/// <summary>
/// An SX126x amplifier setting, mirroring <c>PamojaSx126xTxPower</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xTxPower
{
    /// <summary>The SetPaConfig <c>paDutyCycle</c> byte.</summary>
    public byte PaDutyCycle;

    /// <summary>The SetPaConfig <c>hpMax</c> byte.</summary>
    public byte HpMax;

    /// <summary>The SetPaConfig <c>deviceSel</c> byte.</summary>
    public byte DeviceSel;

    /// <summary>The SetPaConfig <c>paLut</c> byte.</summary>
    public byte PaLut;

    /// <summary>The SetTxParams power, in dBm.</summary>
    public sbyte SettingDbm;
}

/// <summary>
/// The operating mode a status byte reports, mirroring <c>PamojaSx126xChipMode</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaSx126xChipMode : byte
{
    /// <summary>A value the datasheet reserves.</summary>
    Other = 0,

    /// <summary>Standby on the RC oscillator.</summary>
    StandbyRc = 2,

    /// <summary>Standby on the crystal oscillator.</summary>
    StandbyXosc = 3,

    /// <summary>Frequency synthesis.</summary>
    Fs = 4,

    /// <summary>Receiving.</summary>
    Rx = 5,

    /// <summary>Transmitting.</summary>
    Tx = 6,
}

/// <summary>
/// The outcome a status byte reports, mirroring <c>PamojaSx126xCommandStatus</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaSx126xCommandStatus : byte
{
    /// <summary>A value the datasheet reserves.</summary>
    Other = 0,

    /// <summary>Data is available to the host.</summary>
    DataAvailable = 2,

    /// <summary>The command timed out.</summary>
    Timeout = 3,

    /// <summary>The chip could not process the command.</summary>
    ProcessingError = 4,

    /// <summary>The chip failed to execute the command.</summary>
    ExecutionFailure = 5,

    /// <summary>A transmission finished.</summary>
    TxDone = 6,
}

/// <summary>
/// A decoded status byte, mirroring <c>PamojaSx126xStatus</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xStatus
{
    /// <summary>The operating mode.</summary>
    public PamojaSx126xChipMode ChipMode;

    /// <summary>The outcome of the last command.</summary>
    public PamojaSx126xCommandStatus CommandStatus;

    /// <summary><c>1</c> when the outcome is a failure, <c>0</c> otherwise.</summary>
    public byte Error;
}

/// <summary>
/// The signal levels of a received LoRa frame, mirroring <c>PamojaSx126xPacketStatus</c>
/// in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xPacketStatus
{
    /// <summary>The average RSSI over the frame, in hundredths of a dBm.</summary>
    public int RssiCentiDbm;

    /// <summary>The estimated signal-to-noise ratio, in hundredths of a dB.</summary>
    public int SnrCentiDb;

    /// <summary>The RSSI of the despread LoRa signal, in hundredths of a dBm.</summary>
    public int SignalRssiCentiDbm;
}

/// <summary>
/// Where a received payload sits in the chip's buffer, mirroring
/// <c>PamojaSx126xRxBufferStatus</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaSx126xRxBufferStatus
{
    /// <summary>The payload length in bytes.</summary>
    public byte PayloadLen;

    /// <summary>The buffer offset the payload starts at.</summary>
    public byte Start;
}
