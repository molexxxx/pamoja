using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The P/Invoke declarations for the MAVLink service protocols, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The frame was not one the machine handles.</summary>
    public const uint MavlinkStepIgnored = 0;

    /// <summary>A receiver answered with a request for the next item.</summary>
    public const uint MavlinkReceiverRequest = 1;

    /// <summary>A receiver answered with the acknowledgement that ends the transfer.</summary>
    public const uint MavlinkReceiverAck = 2;

    /// <summary>A sender answered with a frame to send.</summary>
    public const uint MavlinkSenderReply = 1;

    /// <summary>A sender saw the receiver's acknowledgement.</summary>
    public const uint MavlinkSenderFinished = 2;

    /// <summary>An acknowledgement was for a different command.</summary>
    public const uint MavlinkAckUnrelated = 1;

    /// <summary>The command is still running.</summary>
    public const uint MavlinkAckInProgress = 2;

    /// <summary>The command finished.</summary>
    public const uint MavlinkAckFinal = 3;

    /// <summary>The retransmissions the mission protocol recommends before giving up.</summary>
    public const byte MavlinkMaxRetries = 5;

    /// <summary>Creates a receiver for a plan from a target vehicle.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_mission_receiver_new(
        byte targetSystem,
        byte targetComponent,
        byte missionType);

    /// <summary>Builds the frame that starts a download.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_mission_receiver_request_list(
        IntPtr receiver,
        PamojaMavlinkHeader header,
        out IntPtr outFrame);

    /// <summary>Handles an incoming frame, if it is one the transfer is waiting for.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_mission_receiver_on_frame(
        IntPtr receiver,
        IntPtr frame,
        PamojaMavlinkHeader header,
        out uint outKind,
        out IntPtr outAccepted,
        out IntPtr outReply);

    /// <summary>Reports whether the transfer has finished.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_mission_receiver_is_complete(IntPtr receiver);

    /// <summary>Returns the next sequence number the receiver expects.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mavlink_mission_receiver_expected(IntPtr receiver);

    /// <summary>Releases a receiver.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_mission_receiver_free(IntPtr receiver);

    /// <summary>Creates a sender for a plan bound for a target vehicle.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_mission_sender_new(
        byte targetSystem,
        byte targetComponent,
        byte missionType);

    /// <summary>Appends an item to the plan.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_mission_sender_add_item(
        IntPtr sender,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Returns the number of items in the plan.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mavlink_mission_sender_len(IntPtr sender);

    /// <summary>Builds the frame that opens an upload.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_mission_sender_count(
        IntPtr sender,
        PamojaMavlinkHeader header,
        out IntPtr outFrame);

    /// <summary>Handles an incoming frame, if it is one the transfer answers.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_mission_sender_on_frame(
        IntPtr sender,
        IntPtr frame,
        PamojaMavlinkHeader header,
        out uint outKind,
        out byte outResult,
        out IntPtr outReply);

    /// <summary>Releases a sender.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_mission_sender_free(IntPtr sender);

    /// <summary>Starts tracking a command.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_command_new(ushort command, byte maxRetries);

    /// <summary>Returns the command id being tracked.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mavlink_command_id(IntPtr command);

    /// <summary>Returns the confirmation count to stamp on the command being sent.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_command_confirmation(IntPtr command);

    /// <summary>Classifies an incoming frame against the command in flight.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_command_on_frame(
        IntPtr command,
        IntPtr frame,
        out uint outKind,
        out byte outValue);

    /// <summary>Records a timeout and reports whether the command may be resent.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_command_on_timeout(
        IntPtr command,
        out byte outConfirmation);

    /// <summary>Releases a command tracker.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_command_free(IntPtr command);

    /// <summary>Builds a setpoint type_mask from the fields to use.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mavlink_offboard_type_mask(uint flags);

    /// <summary>Builds a local-frame position setpoint frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_offboard_local_position(
        PamojaMavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        float x,
        float y,
        float z,
        out IntPtr outFrame);

    /// <summary>Builds a local-frame velocity setpoint frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_offboard_local_velocity(
        PamojaMavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        float vx,
        float vy,
        float vz,
        out IntPtr outFrame);

    /// <summary>Builds a global-frame position setpoint frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_offboard_global_position(
        PamojaMavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        int latInt,
        int lonInt,
        float alt,
        out IntPtr outFrame);
}
