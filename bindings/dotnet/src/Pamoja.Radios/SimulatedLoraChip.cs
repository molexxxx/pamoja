using Pamoja.Lora;
using Pamoja.Native;
using Pamoja.Native.Interop;

using NativeStatus = Pamoja.Native.Interop.Status;

namespace Pamoja.Radios;

/// <summary>What a simulated chip is tuned to.</summary>
/// <param name="FrequencyHz">
/// The carrier frequency in hertz, as the chip's synthesizer steps it: within a hertz of the one
/// asked for on an SX126x, and within 61 Hz on an SX127x.
/// </param>
/// <param name="Link">The spreading factor, bandwidth, coding rate, preamble, header, and CRC.</param>
/// <param name="OutputDbm">The output power the amplifier was asked for, in dBm.</param>
/// <param name="SyncWord">The sync word byte.</param>
public sealed record LoraTuning(uint FrequencyHz, LoraLink Link, sbyte OutputDbm, byte SyncWord);

/// <summary>A frame a simulated chip put on the air.</summary>
/// <param name="Tuning">What the chip was tuned to when the frame went out.</param>
/// <param name="Payload">The frame's payload.</param>
public sealed record LoraSentFrame(LoraTuning Tuning, byte[] Payload);

/// <summary>A simulated SX126x or SX127x, which a <see cref="LoraRadio"/> drives with no radio attached.</summary>
/// <remarks>
/// The program says what arrives on the air with <see cref="Hear"/>, and reads back what the
/// chip was tuned to and what it sent with <see cref="Tuning"/> and <see cref="Sent"/>.
/// Nothing is timed: a transmission is done as soon as it starts, and a reception with a
/// timeout ends at once when nothing is waiting.
/// </remarks>
public sealed class SimulatedLoraChip : IDisposable
{
    private readonly NativeHandle _handle;

    private SimulatedLoraChip(IntPtr chip)
    {
        _handle = new NativeHandle(chip, NativeMethods.pamoja_lora_sim_chip_free, serialized: true);
        Family = NativeMethods.pamoja_lora_sim_chip_family(chip) == NativeMethods.LoraRadioSx126x
            ? LoraRadioFamily.Sx126x
            : LoraRadioFamily.Sx127x;
    }

    /// <summary>The family of the chip.</summary>
    public LoraRadioFamily Family { get; }

    /// <summary>How many frames wait on the air for the chip to receive.</summary>
    public int Waiting =>
        checked((int)_handle.Use(NativeMethods.pamoja_lora_sim_chip_waiting));

    /// <summary>A simulated SX1261, SX1262, SX1268, or LLCC68, out of reset.</summary>
    /// <param name="board">How the module wires the chip.</param>
    /// <returns>The chip.</returns>
    /// <exception cref="ArgumentOutOfRangeException">DIO3 cannot supply the TCXO voltage.</exception>
    public static SimulatedLoraChip Sx126x(Sx126xBoard board)
    {
        ArgumentNullException.ThrowIfNull(board);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_lora_sim_chip_sx126x(
            LoraRadio.NativeBoard(board), out IntPtr chip));
        return new SimulatedLoraChip(chip);
    }

    /// <summary>A simulated SX1276, SX1277, SX1278, or SX1279, out of reset.</summary>
    /// <param name="board">How the module wires the chip.</param>
    /// <returns>The chip.</returns>
    public static SimulatedLoraChip Sx127x(Sx127xBoard board)
    {
        ArgumentNullException.ThrowIfNull(board);
        NativeStatus.ThrowIfError(NativeMethods.pamoja_lora_sim_chip_sx127x(
            board.Output == Sx127xPaOutput.PaBoost, board.Tcxo, out IntPtr chip));
        return new SimulatedLoraChip(chip);
    }

    /// <summary>Returns a radio wired to the chip, reset as opening a module resets it.</summary>
    /// <returns>The radio, in standby and ready for <see cref="LoraRadio.Configure"/>.</returns>
    public LoraRadio Radio()
    {
        using NativeLease chip = _handle.Lease();
        PamojaStatus status = NativeMethods.pamoja_lora_sim_chip_radio(chip.Pointer, out IntPtr radio);
        return LoraRadio.Opened(status, radio);
    }

    /// <summary>Puts a frame on the air for the chip to receive the next time it listens.</summary>
    /// <param name="payload">The frame's payload; past 255 bytes it is cut to 255.</param>
    /// <param name="rssiDbm">The strength the chip hears the frame at, in dBm.</param>
    /// <param name="snrDb">The signal-to-noise ratio it hears it with, in dB.</param>
    /// <exception cref="ArgumentOutOfRangeException">A level is not a finite number.</exception>
    public void Hear(ReadOnlySpan<byte> payload, double rssiDbm, double snrDb)
    {
        int rssi = Level(rssiDbm, nameof(rssiDbm));
        int snr = Level(snrDb, nameof(snrDb));
        using NativeLease chip = _handle.Lease();
        NativeStatus.ThrowIfError(NativeMethods.pamoja_lora_sim_chip_hear(
            chip.Pointer, payload, (nuint)payload.Length, rssi, snr));
    }

    /// <summary>Puts a frame on the air whose CRC fails, which the chip reports and drops.</summary>
    /// <param name="rssiDbm">The strength the chip hears the frame at, in dBm.</param>
    /// <param name="snrDb">The signal-to-noise ratio it hears it with, in dB.</param>
    /// <exception cref="ArgumentOutOfRangeException">A level is not a finite number.</exception>
    public void HearCorrupt(double rssiDbm, double snrDb)
    {
        int rssi = Level(rssiDbm, nameof(rssiDbm));
        int snr = Level(snrDb, nameof(snrDb));
        NativeStatus.ThrowIfError(_handle.Use(chip =>
            NativeMethods.pamoja_lora_sim_chip_hear_corrupt(chip, rssi, snr)));
    }

    /// <summary>Returns what the chip is tuned to now.</summary>
    /// <returns>The tuning.</returns>
    public LoraTuning Tuning()
    {
        PamojaLoraTuning tuning = default;
        NativeStatus.ThrowIfError(_handle.Use(chip =>
            NativeMethods.pamoja_lora_sim_chip_tuning(chip, out tuning)));
        return TuningOf(tuning);
    }

    /// <summary>Returns every frame the chip has put on the air, oldest first.</summary>
    /// <returns>The frames, each with what the chip was tuned to when it went out.</returns>
    public IReadOnlyList<LoraSentFrame> Sent()
    {
        using NativeLease chip = _handle.Lease();
        int count = checked((int)NativeMethods.pamoja_lora_sim_chip_sent_count(chip.Pointer));
        var frames = new List<LoraSentFrame>(count);
        for (int index = 0; index < count; index++)
        {
            NativeStatus.ThrowIfError(NativeMethods.pamoja_lora_sim_chip_sent_tuning(
                chip.Pointer, (nuint)index, out PamojaLoraTuning tuning));
            byte[] payload = OwnedBuffer.Take(
                NativeMethods.pamoja_lora_sim_chip_sent_payload(chip.Pointer, (nuint)index));
            frames.Add(new LoraSentFrame(TuningOf(tuning), payload));
        }

        return frames;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();

    /// <summary>Reads a tuning the C ABI carries.</summary>
    /// <param name="tuning">The tuning as the C ABI carries it.</param>
    /// <returns>The tuning.</returns>
    private static LoraTuning TuningOf(PamojaLoraTuning tuning) => new(
        tuning.FrequencyHz,
        NativeLora.FromLink(tuning.Link),
        tuning.OutputDbm,
        tuning.SyncWord);

    /// <summary>Resolves a level to hundredths of a decibel, refusing one that is not finite.</summary>
    /// <param name="db">The level in decibels.</param>
    /// <param name="name">The argument's name, for the exception.</param>
    /// <returns>The level in hundredths of a decibel.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="db"/> is not a finite number.</exception>
    private static int Level(double db, string name) =>
        double.IsFinite(db)
            ? NativeLora.Centi(db)
            : throw new ArgumentOutOfRangeException(name, db, $"{name} must be a finite number of decibels");
}
