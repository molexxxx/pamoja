using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Mavlink;

/// <summary>What a mission receiver answered an incoming frame with.</summary>
public enum MavlinkReceiverKind : uint
{
    /// <summary>A request for the next item.</summary>
    Request = 1,

    /// <summary>The acknowledgement that ends the transfer.</summary>
    Ack = 2,
}

/// <summary>What a mission sender did with an incoming frame.</summary>
public enum MavlinkSenderKind : uint
{
    /// <summary>There is a frame to send: the count, a requested item, or an error.</summary>
    Reply = 1,

    /// <summary>The receiver acknowledged the transfer; nothing more to send.</summary>
    Finished = 2,
}

/// <summary>What an incoming acknowledgement means for the command in flight.</summary>
public enum MavlinkAckKind : uint
{
    /// <summary>The acknowledgement was for a different command; keep waiting.</summary>
    Unrelated = 1,

    /// <summary>The command is still running; keep waiting.</summary>
    InProgress = 2,

    /// <summary>The command finished with a <c>MAV_RESULT</c>.</summary>
    Final = 3,
}

/// <summary>The fields of a setpoint the autopilot should act on.</summary>
[Flags]
public enum MavlinkTypeMask : uint
{
    /// <summary>No fields; every one is ignored.</summary>
    None = 0,

    /// <summary>The position fields.</summary>
    Position = 1 << 0,

    /// <summary>The velocity fields.</summary>
    Velocity = 1 << 1,

    /// <summary>The acceleration fields.</summary>
    Acceleration = 1 << 2,

    /// <summary>The yaw field.</summary>
    Yaw = 1 << 3,

    /// <summary>The yaw rate field.</summary>
    YawRate = 1 << 4,

    /// <summary>Treat the acceleration fields as a force.</summary>
    Force = 1 << 5,
}

/// <summary>What one incoming frame produced for a <see cref="MavlinkMissionReceiver"/>.</summary>
/// <param name="Kind">What the receiver answered with.</param>
/// <param name="Accepted">
/// The <c>MISSION_ITEM_INT</c> the frame carried, if it was the one expected next, as a
/// message read by field name. The caller owns it.
/// </param>
/// <param name="Reply">The frame to send back. The caller owns it.</param>
public readonly record struct MavlinkReceiverStep(
    MavlinkReceiverKind Kind,
    MavlinkMessage? Accepted,
    MavlinkFrame Reply);

/// <summary>What one incoming frame produced for a <see cref="MavlinkMissionSender"/>.</summary>
/// <param name="Kind">What the sender did.</param>
/// <param name="Reply">The frame to send back, when there is one. The caller owns it.</param>
/// <param name="Result">The receiver's <c>MAV_MISSION_RESULT</c>, when the transfer finished.</param>
public readonly record struct MavlinkSenderStep(
    MavlinkSenderKind Kind,
    MavlinkFrame? Reply,
    byte? Result);

/// <summary>What an incoming acknowledgement means for a <see cref="MavlinkCommand"/>.</summary>
/// <param name="Kind">How the acknowledgement relates to the command in flight.</param>
/// <param name="Value">
/// The progress percent when in progress (255 when the autopilot does not report one), or
/// the <c>MAV_RESULT</c> when final.
/// </param>
public readonly record struct MavlinkAckOutcome(MavlinkAckKind Kind, byte? Value);

/// <summary>
/// Requests a plan's items in order and collects them, ending with an acknowledgement.
/// </summary>
/// <remarks>
/// The machine holds the protocol's rules and nothing else: no IO, no timers. Feed it the
/// frames off a link and send back what it hands you; a frame it does not handle comes back
/// as <c>null</c> rather than as an error, so one link's traffic can be routed through several
/// machines in turn.
/// </remarks>
public sealed class MavlinkMissionReceiver : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a receiver for a plan from a target vehicle.</summary>
    /// <param name="targetSystem">The sending system's id.</param>
    /// <param name="targetComponent">The sending component's id.</param>
    /// <param name="missionType">The <c>MAV_MISSION_TYPE</c> to transfer.</param>
    public MavlinkMissionReceiver(byte targetSystem, byte targetComponent, byte missionType = 0) =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_mission_receiver_new(targetSystem, targetComponent, missionType),
            NativeMethods.pamoja_mavlink_mission_receiver_free,
            "mission receiver");

    private IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>Builds the frame that starts a download.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <returns>The <c>MISSION_REQUEST_LIST</c> frame.</returns>
    public MavlinkFrame RequestList(MavlinkHeader header)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_mission_receiver_request_list(
                Handle,
                header.ToNative(),
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Handles an incoming frame, if it is one this transfer is waiting for.</summary>
    /// <param name="frame">The frame off the link.</param>
    /// <param name="header">The addressing fields to stamp on the reply.</param>
    /// <returns>
    /// The step taken, or <c>null</c> if the frame carries a message this transfer does not
    /// handle.
    /// </returns>
    /// <remarks>
    /// A <c>MISSION_COUNT</c> opens the transfer and a <c>MISSION_ITEM_INT</c> advances it.
    /// </remarks>
    public MavlinkReceiverStep? OnFrame(MavlinkFrame frame, MavlinkHeader header)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_mission_receiver_on_frame(
                Handle,
                frame.Handle,
                header.ToNative(),
                out uint kind,
                out IntPtr accepted,
                out IntPtr reply));
        if (kind == NativeMethods.MavlinkStepIgnored)
        {
            return null;
        }

        return new MavlinkReceiverStep(
            (MavlinkReceiverKind)kind,
            accepted == IntPtr.Zero ? null : new MavlinkMessage(accepted),
            new MavlinkFrame(reply));
    }

    /// <summary>Whether every item has been received and the acknowledgement produced.</summary>
    public bool Complete => NativeMethods.pamoja_mavlink_mission_receiver_is_complete(Handle) != 0;

    /// <summary>The next sequence number the receiver expects.</summary>
    public ushort Expected => NativeMethods.pamoja_mavlink_mission_receiver_expected(Handle);

    /// <summary>Releases the receiver.</summary>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Holds a plan and answers a receiver's requests for its items.</summary>
public sealed class MavlinkMissionSender : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a sender for a plan bound for a target vehicle, with no items yet.</summary>
    /// <param name="targetSystem">The receiving system's id.</param>
    /// <param name="targetComponent">The receiving component's id.</param>
    /// <param name="missionType">The <c>MAV_MISSION_TYPE</c> of the plan.</param>
    public MavlinkMissionSender(byte targetSystem, byte targetComponent, byte missionType = 0) =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_mission_sender_new(targetSystem, targetComponent, missionType),
            NativeMethods.pamoja_mavlink_mission_sender_free,
            "mission sender");

    private IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>Appends an item to the plan.</summary>
    /// <param name="item">A <c>MISSION_ITEM_INT</c> payload.</param>
    /// <remarks>
    /// The sender stamps the sequence number, target ids, and mission type onto each item as
    /// it is handed out, so the payload need only carry the item's content: command, frame,
    /// position, and parameters. Build one by field name with the <c>MISSION_ITEM_INT</c>
    /// schema and pass its <see cref="MavlinkMessage.Payload"/>.
    /// </remarks>
    public void AddItem(ReadOnlySpan<byte> item) =>
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_mission_sender_add_item(Handle, item, (nuint)item.Length));

    /// <summary>Appends an item to the plan.</summary>
    /// <param name="item">A <c>MISSION_ITEM_INT</c> message.</param>
    public void AddItem(MavlinkMessage item) => AddItem(item.Payload);

    /// <summary>The number of items in the plan.</summary>
    public int Count => NativeMethods.pamoja_mavlink_mission_sender_len(Handle);

    /// <summary>Builds the frame that opens an upload.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <returns>The <c>MISSION_COUNT</c> frame.</returns>
    public MavlinkFrame CountFrame(MavlinkHeader header)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_mission_sender_count(Handle, header.ToNative(), out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Handles an incoming frame, if it is one this transfer answers.</summary>
    /// <param name="frame">The frame off the link.</param>
    /// <param name="header">The addressing fields to stamp on the reply.</param>
    /// <returns>
    /// The step taken, or <c>null</c> if the frame carries a message this transfer does not
    /// handle.
    /// </returns>
    /// <remarks>
    /// A <c>MISSION_REQUEST_LIST</c> is answered with the count, a <c>MISSION_REQUEST_INT</c>
    /// (or the older <c>MISSION_REQUEST</c>) with the item asked for, and a request past the
    /// end of the plan with a <c>MISSION_ACK</c> reporting an invalid sequence. A
    /// <c>MISSION_ACK</c> from the receiver ends the transfer.
    /// </remarks>
    public MavlinkSenderStep? OnFrame(MavlinkFrame frame, MavlinkHeader header)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_mission_sender_on_frame(
                Handle,
                frame.Handle,
                header.ToNative(),
                out uint kind,
                out byte result,
                out IntPtr reply));
        return kind switch
        {
            NativeMethods.MavlinkStepIgnored => null,
            NativeMethods.MavlinkSenderReply => new MavlinkSenderStep(
                MavlinkSenderKind.Reply,
                new MavlinkFrame(reply),
                null),
            _ => new MavlinkSenderStep(MavlinkSenderKind.Finished, null, result),
        };
    }

    /// <summary>Releases the sender.</summary>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Tracks one command awaiting its acknowledgement.</summary>
public sealed class MavlinkCommand : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Starts tracking a command.</summary>
    /// <param name="command">The <c>MAV_CMD</c> id being sent.</param>
    /// <param name="maxRetries">
    /// How many times the command may be resent after a timeout before the caller gives up.
    /// </param>
    public MavlinkCommand(ushort command, byte maxRetries = NativeMethods.MavlinkMaxRetries) =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_command_new(command, maxRetries),
            NativeMethods.pamoja_mavlink_command_free,
            "command");

    private IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>The command id being tracked.</summary>
    public ushort Command => NativeMethods.pamoja_mavlink_command_id(Handle);

    /// <summary>
    /// The <c>confirmation</c> count to stamp on the command being sent: zero for the first
    /// transmission, incremented on each retransmission.
    /// </summary>
    public byte Confirmation => NativeMethods.pamoja_mavlink_command_confirmation(Handle);

    /// <summary>Classifies an incoming frame against the command in flight.</summary>
    /// <param name="frame">The frame off the link.</param>
    /// <returns>The outcome, or <c>null</c> if the frame is not a <c>COMMAND_ACK</c>.</returns>
    public MavlinkAckOutcome? OnFrame(MavlinkFrame frame)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_command_on_frame(
                Handle,
                frame.Handle,
                out uint kind,
                out byte value));
        return kind switch
        {
            NativeMethods.MavlinkStepIgnored => null,
            NativeMethods.MavlinkAckUnrelated => new MavlinkAckOutcome(MavlinkAckKind.Unrelated, null),
            _ => new MavlinkAckOutcome((MavlinkAckKind)kind, value),
        };
    }

    /// <summary>Records a timeout and reports whether the command may be resent.</summary>
    /// <returns>
    /// The new confirmation count to stamp on the resend, or <c>null</c> once the retry
    /// budget is exhausted.
    /// </returns>
    public byte? OnTimeout() =>
        NativeMethods.pamoja_mavlink_command_on_timeout(Handle, out byte confirmation) != 0
            ? confirmation
            : null;

    /// <summary>Releases the tracker.</summary>
    public void Dispose() => _handle.Dispose();
}

/// <summary>The setpoint constructors for offboard control.</summary>
public static class MavlinkOffboard
{
    /// <summary>Builds a setpoint <c>type_mask</c> from the fields to use.</summary>
    /// <param name="fields">The fields the autopilot should act on; the rest are ignored.</param>
    /// <returns>The mask, as the <c>type_mask</c> field of a setpoint carries it.</returns>
    public static ushort TypeMask(MavlinkTypeMask fields) =>
        NativeMethods.pamoja_mavlink_offboard_type_mask((uint)fields);

    /// <summary>Builds a local-frame position setpoint frame.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="timeBootMs">The sender's boot timestamp, in milliseconds.</param>
    /// <param name="coordinateFrame">The <c>MAV_FRAME</c> of the setpoint.</param>
    /// <param name="targetSystem">The target system id.</param>
    /// <param name="targetComponent">The target component id.</param>
    /// <param name="x">The position along x, in metres in the chosen frame.</param>
    /// <param name="y">The position along y.</param>
    /// <param name="z">The position along z.</param>
    /// <returns>The <c>SET_POSITION_TARGET_LOCAL_NED</c> frame.</returns>
    public static MavlinkFrame LocalPosition(
        MavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        float x,
        float y,
        float z)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_offboard_local_position(
                header.ToNative(),
                timeBootMs,
                coordinateFrame,
                targetSystem,
                targetComponent,
                x,
                y,
                z,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Builds a local-frame velocity setpoint frame.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="timeBootMs">The sender's boot timestamp, in milliseconds.</param>
    /// <param name="coordinateFrame">The <c>MAV_FRAME</c> of the setpoint.</param>
    /// <param name="targetSystem">The target system id.</param>
    /// <param name="targetComponent">The target component id.</param>
    /// <param name="vx">The velocity along x, in metres per second in the chosen frame.</param>
    /// <param name="vy">The velocity along y.</param>
    /// <param name="vz">The velocity along z.</param>
    /// <returns>The <c>SET_POSITION_TARGET_LOCAL_NED</c> frame.</returns>
    public static MavlinkFrame LocalVelocity(
        MavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        float vx,
        float vy,
        float vz)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_offboard_local_velocity(
                header.ToNative(),
                timeBootMs,
                coordinateFrame,
                targetSystem,
                targetComponent,
                vx,
                vy,
                vz,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Builds a global-frame position setpoint frame.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="timeBootMs">The sender's boot timestamp, in milliseconds.</param>
    /// <param name="coordinateFrame">The <c>MAV_FRAME</c> of the setpoint.</param>
    /// <param name="targetSystem">The target system id.</param>
    /// <param name="targetComponent">The target component id.</param>
    /// <param name="latInt">The latitude, in degrees times ten million.</param>
    /// <param name="lonInt">The longitude, in degrees times ten million.</param>
    /// <param name="alt">The altitude, in metres.</param>
    /// <returns>The <c>SET_POSITION_TARGET_GLOBAL_INT</c> frame.</returns>
    public static MavlinkFrame GlobalPosition(
        MavlinkHeader header,
        uint timeBootMs,
        byte coordinateFrame,
        byte targetSystem,
        byte targetComponent,
        int latInt,
        int lonInt,
        float alt)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_offboard_global_position(
                header.ToNative(),
                timeBootMs,
                coordinateFrame,
                targetSystem,
                targetComponent,
                latInt,
                lonInt,
                alt,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }
}
