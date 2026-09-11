using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the LoRa radio chips of the pamoja C ABI, the SX126x
/// command set and decoders and the duty-cycle guard, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The most bytes an SX126x command or command header takes.</summary>
    public const int Sx126xCommandMax = 10;

    /// <summary>The SetRx timeout that keeps the receiver listening until told otherwise.</summary>
    public const uint Sx126xRxContinuous = 0xFF_FFFF;

    /// <summary>The LoRa sync word public LoRaWAN networks use.</summary>
    public const ushort Sx126xSyncWordPublic = 0x3444;

    /// <summary>The LoRa sync word private networks use.</summary>
    public const ushort Sx126xSyncWordPrivate = 0x1424;

    /// <summary>The register the two bytes of the LoRa sync word are written to.</summary>
    public const ushort Sx126xRegisterLoraSyncWord = 0x0740;

    /// <summary>The interrupt raised when a frame has been sent.</summary>
    public const ushort Sx126xIrqTxDone = 1 << 0;

    /// <summary>The interrupt raised when a frame has been received.</summary>
    public const ushort Sx126xIrqRxDone = 1 << 1;

    /// <summary>The interrupt raised when a preamble is detected.</summary>
    public const ushort Sx126xIrqPreambleDetected = 1 << 2;

    /// <summary>The interrupt raised when a valid sync word is detected.</summary>
    public const ushort Sx126xIrqSyncWordValid = 1 << 3;

    /// <summary>The interrupt raised when a valid LoRa header is received.</summary>
    public const ushort Sx126xIrqHeaderValid = 1 << 4;

    /// <summary>The interrupt raised when a LoRa header fails its CRC.</summary>
    public const ushort Sx126xIrqHeaderError = 1 << 5;

    /// <summary>The interrupt raised when a received payload fails its CRC.</summary>
    public const ushort Sx126xIrqCrcError = 1 << 6;

    /// <summary>The interrupt raised when channel activity detection finishes.</summary>
    public const ushort Sx126xIrqCadDone = 1 << 7;

    /// <summary>The interrupt raised when channel activity detection hears a signal.</summary>
    public const ushort Sx126xIrqCadDetected = 1 << 8;

    /// <summary>The interrupt raised when a transmit or receive timeout expires.</summary>
    public const ushort Sx126xIrqTimeout = 1 << 9;

    /// <summary>The interrupt raised on an LR-FHSS frequency hop.</summary>
    public const ushort Sx126xIrqLrFhssHop = 1 << 14;

    /// <summary>Every interrupt the chip defines.</summary>
    public const ushort Sx126xIrqAll = 0x43FF;

    /// <summary>The device error for a failed RC64k calibration.</summary>
    public const ushort Sx126xErrorRc64kCalibration = 1 << 0;

    /// <summary>The device error for a failed RC13M calibration.</summary>
    public const ushort Sx126xErrorRc13mCalibration = 1 << 1;

    /// <summary>The device error for a failed PLL calibration.</summary>
    public const ushort Sx126xErrorPllCalibration = 1 << 2;

    /// <summary>The device error for a failed ADC calibration.</summary>
    public const ushort Sx126xErrorAdcCalibration = 1 << 3;

    /// <summary>The device error for a failed image calibration.</summary>
    public const ushort Sx126xErrorImageCalibration = 1 << 4;

    /// <summary>The device error for a crystal oscillator that failed to start.</summary>
    public const ushort Sx126xErrorXoscStart = 1 << 5;

    /// <summary>The device error for a PLL that failed to lock.</summary>
    public const ushort Sx126xErrorPllLock = 1 << 6;

    /// <summary>The device error for a power-amplifier ramp that failed.</summary>
    public const ushort Sx126xErrorPaRamp = 1 << 8;

    /// <summary>Returns the RF frequency word for a frequency in hertz.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sx126x_frequency_word(uint frequencyHz);

    /// <summary>Returns a timeout in microseconds as the chip's 15.625 microsecond steps.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sx126x_timeout_steps(ulong timeoutUs);

    /// <summary>Returns the two CalibrateImage codes for a band, high byte first.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sx126x_image_calibration(uint lowHz, uint highHz);

    /// <summary>Returns the shortest ramp time the chip offers at least as long as the one given.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sx126x_ramp_time_us(uint atLeastUs);

    /// <summary>Returns the amplifier setting that delivers an output power.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xTxPower pamoja_sx126x_tx_power_for_output(
        [MarshalAs(UnmanagedType.U1)] bool highPower,
        sbyte outputDbm);

    /// <summary>Returns the strongest amplifier setting that keeps a link under an EIRP ceiling.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xTxPower pamoja_sx126x_tx_power_under_ceiling(
        [MarshalAs(UnmanagedType.U1)] bool highPower,
        PamojaLoraLinkBudget budget,
        int eirpCeilingCentiDbm);

    /// <summary>Builds SetStandby into the RC oscillator standby.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_standby();

    /// <summary>Builds SetPacketType for LoRa.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_packet_type_lora();

    /// <summary>Builds SetRfFrequency for a frequency in hertz.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_rf_frequency(uint frequencyHz);

    /// <summary>Builds CalibrateImage for a band.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_calibrate_image(uint lowHz, uint highHz);

    /// <summary>Builds SetPaConfig for an amplifier setting.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_pa_config(PamojaSx126xTxPower power);

    /// <summary>Builds SetTxParams for an amplifier setting and a ramp time.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_tx_params(
        PamojaSx126xTxPower power,
        uint rampUs);

    /// <summary>Builds SetModulationParams for a LoRa link.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sx126x_set_lora_modulation_params(
        PamojaLoraLink link,
        out PamojaSx126xCommand outCommand);

    /// <summary>Builds SetPacketParams for a LoRa link and a payload length.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_lora_packet_params(
        PamojaLoraLink link,
        byte payloadLen,
        [MarshalAs(UnmanagedType.U1)] bool invertIq);

    /// <summary>Builds SetDioIrqParams from an interrupt mask and the mask each DIO line raises.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_dio_irq_params(
        ushort irq,
        ushort dio1,
        ushort dio2,
        ushort dio3);

    /// <summary>Builds ClearIrqStatus for a set of interrupts.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_clear_irq_status(ushort irq);

    /// <summary>Builds SetTx with a timeout in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_tx(ulong timeoutUs);

    /// <summary>Builds SetRx with a timeout in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_rx(ulong timeoutUs);

    /// <summary>Builds SetRx that listens until told otherwise.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_rx_continuous();

    /// <summary>Builds SetSleep, keeping the configuration for a warm start or not.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_set_sleep(
        [MarshalAs(UnmanagedType.U1)] bool warmStart);

    /// <summary>Builds the WriteRegister header the values follow.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_write_register_header(ushort address);

    /// <summary>Builds the WriteBuffer header the payload follows.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xCommand pamoja_sx126x_write_buffer_header(byte offset);

    /// <summary>Builds GetStatus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_status();

    /// <summary>Builds GetIrqStatus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_irq_status();

    /// <summary>Builds GetRxBufferStatus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_rx_buffer_status();

    /// <summary>Builds GetPacketStatus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_packet_status();

    /// <summary>Builds GetRssiInst.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_rssi_inst();

    /// <summary>Builds GetDeviceErrors.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_get_device_errors();

    /// <summary>Builds ReadRegister for a run of registers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_read_register(ushort address, byte len);

    /// <summary>Builds ReadBuffer for a run of buffer bytes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xQuery pamoja_sx126x_read_buffer(byte offset, byte len);

    /// <summary>Decodes a status byte.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xStatus pamoja_sx126x_status_from_byte(byte value);

    /// <summary>Decodes a GetIrqStatus answer into its interrupt bits.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sx126x_irq_from_bytes(byte high, byte low);

    /// <summary>Decodes a GetDeviceErrors answer into its error bits.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sx126x_device_errors_from_bytes(byte high, byte low);

    /// <summary>Decodes a GetPacketStatus answer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xPacketStatus pamoja_sx126x_packet_status_from_bytes(
        byte rssiPkt,
        byte snrPkt,
        byte signalRssiPkt);

    /// <summary>Decodes a GetRxBufferStatus answer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx126xRxBufferStatus pamoja_sx126x_rx_buffer_status_from_bytes(
        byte payloadLen,
        byte start);

    /// <summary>Decodes a GetRssiInst answer, in hundredths of a dBm.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_sx126x_rssi_inst_centi_dbm(byte value);

    /// <summary>Creates a duty-cycle guard, ready to transmit at once.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_radio_duty_cycle_new(uint permille);

    /// <summary>Records a transmission and returns its time on air in microseconds.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_radio_duty_cycle_transmitted(
        IntPtr guard,
        ulong startedUs,
        PamojaLoraLink link,
        nuint payloadLen);

    /// <summary>Returns the silence still owed in microseconds, or the maximum when transmitting is forbidden.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_radio_duty_cycle_wait_us(IntPtr guard, ulong nowUs);

    /// <summary>Reports whether a transmission may start now.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_radio_duty_cycle_ready(IntPtr guard, ulong nowUs);

    /// <summary>Returns the earliest start for the next transmission, or the maximum when transmitting is forbidden.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_radio_duty_cycle_earliest_us(IntPtr guard);

    /// <summary>Releases a duty-cycle guard handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_radio_duty_cycle_free(IntPtr guard);
}
