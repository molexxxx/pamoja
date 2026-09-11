using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for a LoRa radio on a Linux board, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the same
/// <see cref="NativeMethods"/> class and the same low-level escape hatch. Every part must
/// be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The family of an SX1261, SX1262, SX1268, or LLCC68.</summary>
    public const byte LoraRadioSx126x = 0;

    /// <summary>The family of an SX1276, SX1277, SX1278, or SX1279.</summary>
    public const byte LoraRadioSx127x = 1;

    /// <summary>A reception outcome: a frame arrived and checked.</summary>
    public const byte LoraRadioFrame = 0;

    /// <summary>A reception outcome: no frame arrived before the timeout.</summary>
    public const byte LoraRadioTimeout = 1;

    /// <summary>A reception outcome: a frame arrived whose check failed, and was dropped.</summary>
    public const byte LoraRadioCorrupt = 2;

    /// <summary>A reception outcome: nothing has arrived since the last frame was taken.</summary>
    public const byte LoraRadioNothing = 3;

    /// <summary>The sync word of a public network such as LoRaWAN.</summary>
    public const byte LoraRadioSyncWordPublic = 0x34;

    /// <summary>The sync word of a private network, and both families' reset value.</summary>
    public const byte LoraRadioSyncWordPrivate = 0x12;

    /// <summary>The SPI clock a radio opens at when the call passes 0, in hertz.</summary>
    public const uint LoraRadioDefaultSpiHz = 2_000_000;

    /// <summary>Returns a configuration with a private sync word and standard IQ both ways.</summary>
    [LibraryImport(Library)]
    public static partial PamojaLoraRadioConfig pamoja_lora_radio_config_default(
        uint frequencyHz,
        PamojaLoraLink link,
        sbyte outputDbm);

    /// <summary>Opens an SX1261, SX1262, SX1268, or LLCC68 module and resets it.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_lora_radio_open_sx126x(
        string spi,
        uint spiHz,
        string gpioChip,
        uint busyLine,
        uint resetLine,
        PamojaSx126xBoard board,
        out IntPtr outRadio);

    /// <summary>Opens an SX1276, SX1277, SX1278, or SX1279 module and resets it into LoRa mode.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_lora_radio_open_sx127x(
        string spi,
        uint spiHz,
        string gpioChip,
        uint resetLine,
        [MarshalAs(UnmanagedType.U1)] bool paBoost,
        [MarshalAs(UnmanagedType.U1)] bool tcxo,
        out IntPtr outRadio);

    /// <summary>Returns the family of a radio's chip, or 255 for a null radio.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_lora_radio_family(IntPtr radio);

    /// <summary>Tunes a radio to a configuration.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_configure(
        IntPtr radio,
        PamojaLoraRadioConfig config);

    /// <summary>Sends one frame and waits for it to leave.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_transmit(
        IntPtr radio,
        ReadOnlySpan<byte> payload,
        nuint len,
        out ulong outAirtimeUs);

    /// <summary>Listens for one frame for up to a timeout in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_receive(
        IntPtr radio,
        Span<byte> buffer,
        nuint capacity,
        ulong timeoutUs,
        out PamojaLoraRadioReception outReception);

    /// <summary>Starts listening, frame after frame, until another call changes the mode.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_listen(IntPtr radio);

    /// <summary>Takes the frame a listening radio has received, if one has arrived.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_take_frame(
        IntPtr radio,
        Span<byte> buffer,
        nuint capacity,
        out PamojaLoraRadioReception outReception);

    /// <summary>Puts a radio in standby, which stops a transmission or a reception.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_standby(IntPtr radio);

    /// <summary>Puts a radio to sleep until the next call wakes it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_sleep(IntPtr radio);

    /// <summary>Reads one register of a radio's chip.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_read_register(
        IntPtr radio,
        ushort address,
        out byte outValue);

    /// <summary>Writes one register of a radio's chip.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_lora_radio_write_register(
        IntPtr radio,
        ushort address,
        byte value);

    /// <summary>Closes a radio's device files and releases it.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_lora_radio_free(IntPtr radio);
}
