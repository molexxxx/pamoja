using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the field-I/O capabilities of the pamoja C ABI -
/// serial framing, Modbus RTU, CAN, and on-board bus addressing - mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Frames a payload as a SLIP packet (RFC 1055).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_slip_encode(
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outBuffer);

    /// <summary>Reads the payload back out of a SLIP frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_slip_decode(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out IntPtr outBuffer);

    /// <summary>Frames a payload as a COBS packet.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_cobs_encode(
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outBuffer);

    /// <summary>Reads the payload back out of a COBS frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_serial_cobs_decode(
        ReadOnlySpan<byte> frame,
        nuint frameLen,
        out IntPtr outBuffer);

    /// <summary>Returns the largest SLIP frame a payload of this length can produce.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_serial_slip_max_encoded_len(nuint payloadLen);

    /// <summary>Returns the largest COBS frame a payload of this length can produce.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_serial_cobs_max_encoded_len(nuint payloadLen);

    /// <summary>Creates a streaming SLIP decoder.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_slip_decoder_new();

    /// <summary>Feeds a chunk of the byte stream to a SLIP decoder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_slip_decoder_feed(
        IntPtr decoder,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outFrames);

    /// <summary>Returns how many corrupt frames a SLIP decoder has discarded.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_slip_decoder_discarded(IntPtr decoder);

    /// <summary>Discards any partly assembled SLIP frame.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_slip_decoder_reset(IntPtr decoder);

    /// <summary>Releases a SLIP decoder handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_slip_decoder_free(IntPtr decoder);

    /// <summary>Creates a streaming COBS decoder.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_cobs_decoder_new();

    /// <summary>Feeds a chunk of the byte stream to a COBS decoder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cobs_decoder_feed(
        IntPtr decoder,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outFrames);

    /// <summary>Returns how many corrupt frames a COBS decoder has discarded.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_cobs_decoder_discarded(IntPtr decoder);

    /// <summary>Discards any partly assembled COBS frame.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_cobs_decoder_reset(IntPtr decoder);

    /// <summary>Releases a COBS decoder handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_cobs_decoder_free(IntPtr decoder);

    /// <summary>Returns how many frames a decoder call produced.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_frames_count(IntPtr frames);

    /// <summary>Returns a pointer to one frame's payload bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_frames_data(IntPtr frames, nuint index);

    /// <summary>Returns the length in bytes of one frame's payload.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_frames_len(IntPtr frames, nuint index);

    /// <summary>Releases a decoded frame set. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_frames_free(IntPtr frames);

    /// <summary>Computes the CRC-16/MODBUS that every RTU frame ends with.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_modbus_crc16(ReadOnlySpan<byte> bytes, nuint bytesLen);

    /// <summary>Builds a read-coils request frame (function 0x01).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_coils(
        byte address,
        ushort start,
        ushort count,
        out IntPtr outBuffer);

    /// <summary>Builds a read-discrete-inputs request frame (function 0x02).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_discrete_inputs(
        byte address,
        ushort start,
        ushort count,
        out IntPtr outBuffer);

    /// <summary>Builds a read-holding-registers request frame (function 0x03).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_holding_registers(
        byte address,
        ushort start,
        ushort count,
        out IntPtr outBuffer);

    /// <summary>Builds a read-input-registers request frame (function 0x04).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_input_registers(
        byte address,
        ushort start,
        ushort count,
        out IntPtr outBuffer);

    /// <summary>Builds the reply to a read-holding-registers request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_holding_registers_reply(
        byte address,
        ReadOnlySpan<ushort> values,
        nuint valuesLen,
        out IntPtr outBuffer);

    /// <summary>Builds the reply to a read-input-registers request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_read_input_registers_reply(
        byte address,
        ReadOnlySpan<ushort> values,
        nuint valuesLen,
        out IntPtr outBuffer);

    /// <summary>Builds a write-single-coil request frame (function 0x05).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_write_single_coil(
        byte address,
        ushort coil,
        [MarshalAs(UnmanagedType.U1)] bool on,
        out IntPtr outBuffer);

    /// <summary>Builds a write-single-register request frame (function 0x06).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_write_single_register(
        byte address,
        ushort register,
        ushort value,
        out IntPtr outBuffer);

    /// <summary>Builds a write-multiple-registers request frame (function 0x10).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_write_multiple_registers(
        byte address,
        ushort start,
        ReadOnlySpan<ushort> values,
        nuint count,
        out IntPtr outBuffer);

    /// <summary>Builds a write-multiple-coils request frame (function 0x0F).</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_write_multiple_coils(
        byte address,
        ushort start,
        ReadOnlySpan<byte> values,
        nuint count,
        out IntPtr outBuffer);

    /// <summary>Builds a request frame from a raw function code and data.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_raw(
        byte address,
        byte function,
        ReadOnlySpan<byte> data,
        nuint dataLen,
        out IntPtr outBuffer);

    /// <summary>Parses a received RTU frame, verifying its CRC.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_frame_parse(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outFrame);

    /// <summary>Returns the unit address a frame is addressed to or came from.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_modbus_frame_address(IntPtr frame);

    /// <summary>Returns a frame's function code as it appeared on the wire.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_modbus_frame_function(IntPtr frame);

    /// <summary>Returns the exception code a device reported, or 0 for none.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_modbus_frame_exception(IntPtr frame);

    /// <summary>Returns a pointer to a frame's PDU.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_modbus_frame_pdu(IntPtr frame);

    /// <summary>Returns the length in bytes of a frame's PDU.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_modbus_frame_pdu_len(IntPtr frame);

    /// <summary>Reads the 16-bit registers out of a read-registers response.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_frame_registers(
        IntPtr frame,
        out IntPtr outRegisters);

    /// <summary>Reads the coils out of a read-bits response, one byte per coil.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_modbus_frame_coils(
        IntPtr frame,
        ushort count,
        out IntPtr outBuffer);

    /// <summary>Releases a parsed frame handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_modbus_frame_free(IntPtr frame);

    /// <summary>Returns a pointer to the registers a device returned.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_registers_data(IntPtr registers);

    /// <summary>Returns how many registers a device returned.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_registers_len(IntPtr registers);

    /// <summary>Releases a register series. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_registers_free(IntPtr registers);

    /// <summary>Builds a classic CAN 2.0 frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_frame_new(
        uint id,
        [MarshalAs(UnmanagedType.U1)] bool extended,
        ReadOnlySpan<byte> data,
        nuint dataLen,
        out IntPtr outFrame);

    /// <summary>Builds a CAN-FD frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_frame_fd(
        uint id,
        [MarshalAs(UnmanagedType.U1)] bool extended,
        ReadOnlySpan<byte> data,
        nuint dataLen,
        out IntPtr outFrame);

    /// <summary>Builds a remote transmission request.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_can_frame_remote(
        uint id,
        [MarshalAs(UnmanagedType.U1)] bool extended,
        nuint len,
        out IntPtr outFrame);

    /// <summary>Returns a frame's identifier.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_can_frame_id(IntPtr frame);

    /// <summary>Reports whether a frame carries a 29-bit extended identifier.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_frame_is_extended(IntPtr frame);

    /// <summary>Reports whether a frame is CAN-FD rather than classic CAN 2.0.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_frame_is_fd(IntPtr frame);

    /// <summary>Reports whether a frame is a remote transmission request.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_frame_is_remote(IntPtr frame);

    /// <summary>Returns a frame's data length, which a remote frame only requests.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_can_frame_len(IntPtr frame);

    /// <summary>Returns a frame's data length code.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_can_frame_dlc(IntPtr frame);

    /// <summary>Returns a pointer to a frame's payload bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_can_frame_data(IntPtr frame);

    /// <summary>Returns how many bytes the payload pointer covers.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_can_frame_data_len(IntPtr frame);

    /// <summary>Releases a CAN frame handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_can_frame_free(IntPtr frame);

    /// <summary>Returns the data length code that encodes a payload length.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_can_len_to_dlc(nuint len);

    /// <summary>Returns the payload length a data length code encodes.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_can_dlc_to_len(byte dlc);

    /// <summary>Decodes the J1939 fields out of an extended CAN identifier.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_j1939_decode(
        uint id,
        [MarshalAs(UnmanagedType.U1)] bool extended,
        out PamojaJ1939Id outMessage);

    /// <summary>Composes the extended identifier a set of J1939 fields describes.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_can_j1939_compose(
        byte priority,
        uint pgn,
        byte source,
        byte destination);

    /// <summary>Composes the identifier of a J1939 broadcast.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_can_j1939_broadcast(byte priority, uint pgn, byte source);

    /// <summary>The byte a J1939 sender writes for a signal it is not reporting.</summary>
    public const byte J1939NotAvailable = 0xFF;

    /// <summary>The destination address every node on the bus reads.</summary>
    public const byte J1939BroadcastAddress = 0xFF;

    /// <summary>The priority a control message takes, ahead of ordinary traffic.</summary>
    public const byte J1939PriorityControl = 3;

    /// <summary>The priority ordinary traffic takes.</summary>
    public const byte J1939PriorityDefault = 6;

    /// <summary>The priority that yields to everything else on the bus.</summary>
    public const byte J1939PriorityLowest = 7;

    /// <summary>Builds a J1939 payload with every signal marked not available.</summary>
    [LibraryImport(Library)]
    public static partial PamojaJ1939Signals pamoja_can_signals_new();

    /// <summary>Writes a one-byte signal at the offset its group defines.</summary>
    [LibraryImport(Library)]
    public static partial PamojaJ1939Signals pamoja_can_signals_set_u8(
        PamojaJ1939Signals signals,
        nuint at,
        byte value);

    /// <summary>Writes a two-byte little-endian signal at the offset its group defines.</summary>
    [LibraryImport(Library)]
    public static partial PamojaJ1939Signals pamoja_can_signals_set_u16(
        PamojaJ1939Signals signals,
        nuint at,
        ushort value);

    /// <summary>Reads a one-byte signal at the offset its group defines.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_signals_u8(
        PamojaJ1939Signals signals,
        nuint at,
        out byte outValue);

    /// <summary>Reads a two-byte little-endian signal at the offset its group defines.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_can_signals_u16(
        PamojaJ1939Signals signals,
        nuint at,
        out ushort outValue);

    /// <summary>The lowest 7-bit address the I2C specification keeps for itself.</summary>
    public const byte I2cReservedFrom = 0x78;

    /// <summary>The first 7-bit address above the reserved block at the bottom.</summary>
    public const byte I2cReservedBelow = 0x08;

    /// <summary>Validates a 7-bit I2C address.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_address_seven_bit(
        byte address,
        out PamojaI2cAddress outAddress);

    /// <summary>Validates a 10-bit I2C address.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_address_ten_bit(
        ushort address,
        out PamojaI2cAddress outAddress);

    /// <summary>Returns how many bytes an address frame occupies.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_i2c_address_frame_len(PamojaI2cAddress address);

    /// <summary>Writes the address bytes a controller puts on the bus.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_i2c_address_frame(
        PamojaI2cAddress address,
        PamojaI2cDirection direction,
        Span<byte> outFrame,
        nuint outFrameCap,
        out nuint outLen);

    /// <summary>Reports whether an address falls in a reserved range.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_i2c_address_is_reserved(PamojaI2cAddress address);

    /// <summary>Reports whether an address is the general call address 0x00.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_i2c_address_is_general_call(PamojaI2cAddress address);

    /// <summary>Returns the clock polarity and phase an SPI mode number names.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_spi_mode_cpol_cpha(
        byte mode,
        [MarshalAs(UnmanagedType.U1)] out bool outCpol,
        [MarshalAs(UnmanagedType.U1)] out bool outCpha);

    /// <summary>Returns the SPI mode number a clock polarity and phase name.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_spi_mode_from_cpol_cpha(
        [MarshalAs(UnmanagedType.U1)] bool cpol,
        [MarshalAs(UnmanagedType.U1)] bool cpha);

    /// <summary>Returns the level a boolean names.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPinLevel pamoja_pin_level_from_bool(
        [MarshalAs(UnmanagedType.U1)] bool high);

    /// <summary>Returns the opposite level.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPinLevel pamoja_pin_level_inverted(PamojaPinLevel level);

    /// <summary>Reports whether a transition fires an interrupt trigger.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_pin_edge_triggered_by(
        PamojaPinEdge edge,
        PamojaPinLevel from,
        PamojaPinLevel to);

    /// <summary>Returns the physical level representing a logical state.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPinLevel pamoja_pin_polarity_level(
        PamojaPinPolarity polarity,
        [MarshalAs(UnmanagedType.U1)] bool asserted);

    /// <summary>Reports whether a physical level means the signal is asserted.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_pin_polarity_is_asserted(
        PamojaPinPolarity polarity,
        PamojaPinLevel level);
}
