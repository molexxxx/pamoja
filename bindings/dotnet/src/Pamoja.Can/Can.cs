using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Can;

/// <summary>CAN bus framing, and the J1939 identifier that rides on top of it.</summary>
/// <remarks>
/// CAN is how the moving parts of a machine talk to each other: motor controllers,
/// servos, battery management, and the engines and farm equipment that speak
/// J1939. This is the identifier and payload layer; the controller hardware
/// handles the wire itself.
/// </remarks>
public static class Can
{
    /// <summary>Builds a classic CAN 2.0 frame.</summary>
    /// <param name="id">The arbitration identifier, masked to the width selected.</param>
    /// <param name="data">The payload, at most eight bytes.</param>
    /// <param name="extended">Whether the identifier is a 29-bit extended one.</param>
    /// <returns>The frame.</returns>
    /// <exception cref="PamojaException">
    /// The payload is longer than a classic frame carries.
    /// </exception>
    public static CanFrame Frame(uint id, ReadOnlySpan<byte> data, bool extended = false)
    {
        Status.ThrowIfError(NativeMethods.pamoja_can_frame_new(
            id, extended, data, (nuint)data.Length, out IntPtr frame));
        return Describe(frame);
    }

    /// <summary>Builds a CAN-FD frame, which carries up to 64 bytes.</summary>
    /// <param name="id">The arbitration identifier.</param>
    /// <param name="data">
    /// The payload, at one of the discrete CAN-FD lengths: 0 to 8, then 12, 16, 20,
    /// 24, 32, 48, or 64 bytes.
    /// </param>
    /// <param name="extended">Whether the identifier is a 29-bit extended one.</param>
    /// <returns>The frame.</returns>
    /// <exception cref="PamojaException">
    /// The payload length is not one CAN-FD can carry.
    /// </exception>
    public static CanFrame FdFrame(uint id, ReadOnlySpan<byte> data, bool extended = false)
    {
        Status.ThrowIfError(NativeMethods.pamoja_can_frame_fd(
            id, extended, data, (nuint)data.Length, out IntPtr frame));
        return Describe(frame);
    }

    /// <summary>Builds a remote transmission request, which asks another node to send.</summary>
    /// <param name="id">The arbitration identifier.</param>
    /// <param name="length">The data length being requested, clamped to eight bytes.</param>
    /// <param name="extended">Whether the identifier is a 29-bit extended one.</param>
    /// <returns>The frame, which carries no payload of its own.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static CanFrame RemoteFrame(uint id, int length, bool extended = false)
    {
        Status.ThrowIfError(NativeMethods.pamoja_can_frame_remote(
            id, extended, (nuint)length, out IntPtr frame));
        return Describe(frame);
    }

    /// <summary>Returns the data length code that encodes a payload length.</summary>
    /// <param name="length">The payload length in bytes.</param>
    /// <returns>The code, rounding up to the next length CAN-FD can carry.</returns>
    public static byte LenToDlc(int length) => NativeMethods.pamoja_can_len_to_dlc((nuint)length);

    /// <summary>Returns the payload length a data length code encodes.</summary>
    /// <param name="dlc">The data length code.</param>
    /// <returns>The length in bytes.</returns>
    public static int DlcToLen(byte dlc) => checked((int)NativeMethods.pamoja_can_dlc_to_len(dlc));

    /// <summary>Decodes the J1939 fields out of an extended CAN identifier.</summary>
    /// <param name="id">The identifier as it arrived.</param>
    /// <param name="extended">Whether it is a 29-bit extended identifier.</param>
    /// <returns>
    /// The decoded message, or <c>null</c> for a standard identifier, which J1939
    /// does not use.
    /// </returns>
    public static J1939Message? DecodeJ1939(uint id, bool extended = true)
    {
        if (!NativeMethods.pamoja_can_j1939_decode(id, extended, out PamojaJ1939Id message))
        {
            return null;
        }

        return new J1939Message(
            message.Pgn,
            message.Priority,
            message.Source,
            message.PduFormat,
            message.Addressed != 0 ? message.Destination : null,
            message.Addressed == 0);
    }

    /// <summary>Composes the identifier of a J1939 broadcast, which every node reads.</summary>
    /// <remarks>
    /// Most parameter groups are broadcast, so this is the common case; it saves a
    /// caller knowing that a broadcast is addressed to <c>0xFF</c>.
    /// </remarks>
    /// <param name="priority">The message priority, 0 (highest) to 7.</param>
    /// <param name="pgn">The parameter group number.</param>
    /// <param name="source">The address of the sending node.</param>
    /// <returns>The 29-bit identifier.</returns>
    public static uint BroadcastJ1939(J1939Priority priority, uint pgn, byte source)
    {
        return NativeMethods.pamoja_can_j1939_broadcast((byte)priority, pgn, source);
    }

    /// <summary>Composes the extended CAN identifier a set of J1939 fields describes.</summary>
    /// <param name="priority">The message priority, 0 (highest) to 7.</param>
    /// <param name="pgn">The parameter group number.</param>
    /// <param name="source">The address of the sending node.</param>
    /// <param name="destination">
    /// The destination address, used only for an addressed (PDU1) parameter group
    /// and ignored for a broadcast (PDU2) one.
    /// </param>
    /// <returns>The 29-bit identifier.</returns>
    public static uint ComposeJ1939(byte priority, uint pgn, byte source, byte destination = 0) =>
        NativeMethods.pamoja_can_j1939_compose(priority, pgn, source, destination);

    /// <summary>Reads every field out of a native frame handle and releases it.</summary>
    /// <param name="frame">The handle a native constructor produced.</param>
    /// <returns>The frame as a value, so callers never hold a native resource.</returns>
    private static CanFrame Describe(IntPtr frame)
    {
        try
        {
            // The payload pointer has its own length: a remote frame reports the
            // length it requests while carrying no bytes.
            int dataLength = checked((int)NativeMethods.pamoja_can_frame_data_len(frame));
            byte[] data = new byte[dataLength];
            if (dataLength > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_can_frame_data(frame), data, 0, dataLength);
            }

            return new CanFrame(
                NativeMethods.pamoja_can_frame_id(frame),
                NativeMethods.pamoja_can_frame_is_extended(frame),
                NativeMethods.pamoja_can_frame_is_fd(frame),
                NativeMethods.pamoja_can_frame_is_remote(frame),
                checked((int)NativeMethods.pamoja_can_frame_len(frame)),
                NativeMethods.pamoja_can_frame_dlc(frame),
                data);
        }
        finally
        {
            NativeMethods.pamoja_can_frame_free(frame);
        }
    }
}
