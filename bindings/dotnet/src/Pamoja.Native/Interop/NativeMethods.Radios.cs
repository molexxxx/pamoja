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

    /// <summary>The RegVersion value of an SX1276, SX1277, SX1278, or SX1279.</summary>
    public const byte Sx127xVersion = 0x12;

    /// <summary>The bit of an SX127x address byte that makes an access a write.</summary>
    public const byte Sx127xWrite = 0x80;

    /// <summary>RegDioMapping1 with DIO0 signaling RxDone.</summary>
    public const byte Sx127xDio0RxDone = 0x00;

    /// <summary>RegDioMapping1 with DIO0 signaling TxDone.</summary>
    public const byte Sx127xDio0TxDone = 0x40;

    /// <summary>RegDioMapping1 with DIO0 signaling CadDone.</summary>
    public const byte Sx127xDio0CadDone = 0x80;

    /// <summary>RegPaDac at its reset value.</summary>
    public const byte Sx127xPaDacDefault = 0x84;

    /// <summary>RegPaDac with the +20 dBm setting on PA_BOOST.</summary>
    public const byte Sx127xPaDacHighPower = 0x87;

    /// <summary>The RegImageCal bit that starts a calibration.</summary>
    public const byte Sx127xImageCalStart = 0x40;

    /// <summary>The RegImageCal bit set while a calibration runs.</summary>
    public const byte Sx127xImageCalRunning = 0x20;

    /// <summary>The SX127x sync word reserved for LoRaWAN.</summary>
    public const byte Sx127xSyncWordPublic = 0x34;

    /// <summary>The SX127x private sync word, and the chip's reset value.</summary>
    public const byte Sx127xSyncWordPrivate = 0x12;

    /// <summary>RegLna with maximum gain and the high frequency LNA boost.</summary>
    public const byte Sx127xLnaBoosted = 0x23;

    /// <summary>RegTcxo for a module clocked by a TCXO.</summary>
    public const byte Sx127xTcxoInputOn = 0x19;

    /// <summary>The SX127x register at 0x00.</summary>
    public const byte Sx127xRegFifo = 0x00;

    /// <summary>The SX127x register at 0x01.</summary>
    public const byte Sx127xRegOpMode = 0x01;

    /// <summary>The SX127x register at 0x06.</summary>
    public const byte Sx127xRegFrfMsb = 0x06;

    /// <summary>The SX127x register at 0x07.</summary>
    public const byte Sx127xRegFrfMid = 0x07;

    /// <summary>The SX127x register at 0x08.</summary>
    public const byte Sx127xRegFrfLsb = 0x08;

    /// <summary>The SX127x register at 0x09.</summary>
    public const byte Sx127xRegPaConfig = 0x09;

    /// <summary>The SX127x register at 0x0A.</summary>
    public const byte Sx127xRegPaRamp = 0x0A;

    /// <summary>The SX127x register at 0x0B.</summary>
    public const byte Sx127xRegOcp = 0x0B;

    /// <summary>The SX127x register at 0x0C.</summary>
    public const byte Sx127xRegLna = 0x0C;

    /// <summary>The SX127x register at 0x0D.</summary>
    public const byte Sx127xRegFifoAddrPtr = 0x0D;

    /// <summary>The SX127x register at 0x0E.</summary>
    public const byte Sx127xRegFifoTxBaseAddr = 0x0E;

    /// <summary>The SX127x register at 0x0F.</summary>
    public const byte Sx127xRegFifoRxBaseAddr = 0x0F;

    /// <summary>The SX127x register at 0x10.</summary>
    public const byte Sx127xRegFifoRxCurrentAddr = 0x10;

    /// <summary>The SX127x register at 0x11.</summary>
    public const byte Sx127xRegIrqFlagsMask = 0x11;

    /// <summary>The SX127x register at 0x12.</summary>
    public const byte Sx127xRegIrqFlags = 0x12;

    /// <summary>The SX127x register at 0x13.</summary>
    public const byte Sx127xRegRxNbBytes = 0x13;

    /// <summary>The SX127x register at 0x18.</summary>
    public const byte Sx127xRegModemStat = 0x18;

    /// <summary>The SX127x register at 0x19.</summary>
    public const byte Sx127xRegPktSnrValue = 0x19;

    /// <summary>The SX127x register at 0x1A.</summary>
    public const byte Sx127xRegPktRssiValue = 0x1A;

    /// <summary>The SX127x register at 0x1B.</summary>
    public const byte Sx127xRegRssiValue = 0x1B;

    /// <summary>The SX127x register at 0x1C.</summary>
    public const byte Sx127xRegHopChannel = 0x1C;

    /// <summary>The SX127x register at 0x1D.</summary>
    public const byte Sx127xRegModemConfig1 = 0x1D;

    /// <summary>The SX127x register at 0x1E.</summary>
    public const byte Sx127xRegModemConfig2 = 0x1E;

    /// <summary>The SX127x register at 0x1F.</summary>
    public const byte Sx127xRegSymbTimeoutLsb = 0x1F;

    /// <summary>The SX127x register at 0x20.</summary>
    public const byte Sx127xRegPreambleMsb = 0x20;

    /// <summary>The SX127x register at 0x21.</summary>
    public const byte Sx127xRegPreambleLsb = 0x21;

    /// <summary>The SX127x register at 0x22.</summary>
    public const byte Sx127xRegPayloadLength = 0x22;

    /// <summary>The SX127x register at 0x23.</summary>
    public const byte Sx127xRegMaxPayloadLength = 0x23;

    /// <summary>The SX127x register at 0x26.</summary>
    public const byte Sx127xRegModemConfig3 = 0x26;

    /// <summary>The SX127x register at 0x2C.</summary>
    public const byte Sx127xRegRssiWideband = 0x2C;

    /// <summary>The SX127x register at 0x2F.</summary>
    public const byte Sx127xRegIfFreq2 = 0x2F;

    /// <summary>The SX127x register at 0x30.</summary>
    public const byte Sx127xRegIfFreq1 = 0x30;

    /// <summary>The SX127x register at 0x31.</summary>
    public const byte Sx127xRegDetectOptimize = 0x31;

    /// <summary>The SX127x register at 0x33.</summary>
    public const byte Sx127xRegInvertIq = 0x33;

    /// <summary>The SX127x register at 0x36.</summary>
    public const byte Sx127xRegHighBwOptimize1 = 0x36;

    /// <summary>The SX127x register at 0x37.</summary>
    public const byte Sx127xRegDetectionThreshold = 0x37;

    /// <summary>The SX127x register at 0x39.</summary>
    public const byte Sx127xRegSyncWord = 0x39;

    /// <summary>The SX127x register at 0x3A.</summary>
    public const byte Sx127xRegHighBwOptimize2 = 0x3A;

    /// <summary>The SX127x register at 0x3B.</summary>
    public const byte Sx127xRegInvertIq2 = 0x3B;

    /// <summary>The SX127x register at 0x3B.</summary>
    public const byte Sx127xRegImageCal = 0x3B;

    /// <summary>The SX127x register at 0x40.</summary>
    public const byte Sx127xRegDioMapping1 = 0x40;

    /// <summary>The SX127x register at 0x41.</summary>
    public const byte Sx127xRegDioMapping2 = 0x41;

    /// <summary>The SX127x register at 0x42.</summary>
    public const byte Sx127xRegVersion = 0x42;

    /// <summary>The SX127x register at 0x4B.</summary>
    public const byte Sx127xRegTcxo = 0x4B;

    /// <summary>The SX127x register at 0x4D.</summary>
    public const byte Sx127xRegPaDac = 0x4D;

    /// <summary>The SX127x operating mode code 0.</summary>
    public const byte Sx127xModeSleep = 0;

    /// <summary>The SX127x operating mode code 1.</summary>
    public const byte Sx127xModeStandby = 1;

    /// <summary>The SX127x operating mode code 2.</summary>
    public const byte Sx127xModeFsTx = 2;

    /// <summary>The SX127x operating mode code 3.</summary>
    public const byte Sx127xModeTx = 3;

    /// <summary>The SX127x operating mode code 4.</summary>
    public const byte Sx127xModeFsRx = 4;

    /// <summary>The SX127x operating mode code 5.</summary>
    public const byte Sx127xModeRxContinuous = 5;

    /// <summary>The SX127x operating mode code 6.</summary>
    public const byte Sx127xModeRxSingle = 6;

    /// <summary>The SX127x operating mode code 7.</summary>
    public const byte Sx127xModeCad = 7;

    /// <summary>The SX127x interrupt flag 0x01.</summary>
    public const byte Sx127xIrqCadDetected = 0x01;

    /// <summary>The SX127x interrupt flag 0x02.</summary>
    public const byte Sx127xIrqFhssChangeChannel = 0x02;

    /// <summary>The SX127x interrupt flag 0x04.</summary>
    public const byte Sx127xIrqCadDone = 0x04;

    /// <summary>The SX127x interrupt flag 0x08.</summary>
    public const byte Sx127xIrqTxDone = 0x08;

    /// <summary>The SX127x interrupt flag 0x10.</summary>
    public const byte Sx127xIrqValidHeader = 0x10;

    /// <summary>The SX127x interrupt flag 0x20.</summary>
    public const byte Sx127xIrqPayloadCrcError = 0x20;

    /// <summary>The SX127x interrupt flag 0x40.</summary>
    public const byte Sx127xIrqRxDone = 0x40;

    /// <summary>The SX127x interrupt flag 0x80.</summary>
    public const byte Sx127xIrqRxTimeout = 0x80;

    /// <summary>The SX127x interrupt flag 0xFF.</summary>
    public const byte Sx127xIrqAll = 0xFF;

    /// <summary>Reports whether an LLCC68 supports a link's spreading factor at its bandwidth.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_sx126x_llcc68_supports(PamojaLoraLink link);

    /// <summary>Returns the 24-bit RegFrf word an SX127x takes for a frequency.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sx127x_frequency_word(uint frequencyHz);

    /// <summary>Returns the frequency an SX127x RegFrf word selects.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_sx127x_frequency_from_word(uint word);

    /// <summary>Returns the SX127x address byte that reads a register.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_read_address(byte address);

    /// <summary>Returns the SX127x address byte that writes a register.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_write_address(byte address);

    /// <summary>Returns the RegOpMode value for a LoRa operating mode code.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_lora_op_mode(byte mode);

    /// <summary>Returns the RegOpMode value for an FSK operating mode code.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_fsk_op_mode(byte mode);

    /// <summary>Returns the operating mode code a RegOpMode value holds.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_mode_from_op_mode(byte opMode);

    /// <summary>Returns the LoRa modem registers for a link at a carrier.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sx127x_modem(
        PamojaLoraLink link,
        uint frequencyHz,
        ushort symbolTimeout,
        out PamojaSx127xModem outModem);

    /// <summary>Returns a single reception timeout in a link's symbols.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_sx127x_symbol_timeout(PamojaLoraLink link, ulong timeoutUs);

    /// <summary>Returns the SX127x amplifier settings for an output power.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx127xTxPower pamoja_sx127x_tx_power_for_output(
        [MarshalAs(UnmanagedType.U1)] bool paBoost,
        sbyte outputDbm);

    /// <summary>Returns the SX127x amplifier settings that keep a link under an EIRP ceiling.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx127xTxPower pamoja_sx127x_tx_power_under_ceiling(
        [MarshalAs(UnmanagedType.U1)] bool paBoost,
        PamojaLoraLinkBudget budget,
        int eirpCeilingCentiDbm);

    /// <summary>Returns RegOcp for a current limit.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_ocp_register(ushort milliamps);

    /// <summary>Returns RegInvertIQ for the IQ polarity of each path.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_invert_iq(
        [MarshalAs(UnmanagedType.U1)] bool receive,
        [MarshalAs(UnmanagedType.U1)] bool transmit);

    /// <summary>Returns RegInvertIQ2 for the path in use.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_invert_iq_2([MarshalAs(UnmanagedType.U1)] bool inverted);

    /// <summary>Returns the writes of the 500 kHz sensitivity erratum.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sx127x_high_bw_optimize(
        PamojaLoraLink link,
        uint frequencyHz,
        out PamojaSx127xHighBwOptimize outOptimize);

    /// <summary>Returns the receive settings of the spurious reception erratum.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sx127x_spurious_reception(
        PamojaLoraLink link,
        out PamojaSx127xSpuriousReception outErratum);

    /// <summary>Returns RegImageCal to start a calibration.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_image_cal_start(byte current);

    /// <summary>Returns RegDetectOptimize with AutomaticIFOn set or clear.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_sx127x_automatic_if(
        byte current,
        [MarshalAs(UnmanagedType.U1)] bool automaticIf);

    /// <summary>Decodes RegPktSnrValue and RegPktRssiValue for a carrier.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx127xPacketStatus pamoja_sx127x_packet_status_from_bytes(
        byte pktSnr,
        byte pktRssi,
        uint frequencyHz);

    /// <summary>Decodes RegRssiValue for a carrier, in hundredths of a dBm.</summary>
    [LibraryImport(Library)]
    public static partial int pamoja_sx127x_rssi_centi_dbm(byte value, uint frequencyHz);

    /// <summary>Decodes RegModemStat.</summary>
    [LibraryImport(Library)]
    public static partial PamojaSx127xModemStatus pamoja_sx127x_modem_status_from_byte(byte value);
}
