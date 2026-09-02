using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The P/Invoke declarations for the async transport capabilities of the pamoja
/// C ABI - CoAP, the loopback broker, store-and-forward buffers, the transport
/// ladder, the event bus, and the simulated devices - mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Returns the topic a message arrived on.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_message_topic(IntPtr message);

    /// <summary>Returns a pointer to a message payload.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_message_payload(IntPtr message);

    /// <summary>Returns the length in bytes of a message payload.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_message_payload_len(IntPtr message);

    /// <summary>Releases a message handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_message_free(IntPtr message);

    /// <summary>Creates an MQTT transport for composing.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_transport_mqtt(ref PamojaMqttConfig config);

    /// <summary>Creates a CoAP transport for composing.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_transport_coap(ref PamojaCoapConfig config);

    /// <summary>Creates a loopback transport for composing.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_transport_loopback(IntPtr broker);

    /// <summary>Wraps a transport so a set number of its next sends fail.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_transport_faulty(IntPtr transport, nuint failures);

    /// <summary>Wraps a transport in a link that loses packets and goes down.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_transport_degraded(
        IntPtr transport,
        uint dropEvery,
        uint up,
        uint down);

    /// <summary>Connects a transport.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_transport_connect(IntPtr transport);

    /// <summary>Sends a payload over a transport.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_transport_send(
        IntPtr transport,
        IntPtr topic,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Subscribes a transport to a topic.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_transport_subscribe(IntPtr transport, IntPtr topic);

    /// <summary>Releases a transport handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_transport_free(IntPtr transport);

    /// <summary>Creates a disconnected CoAP endpoint.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_coap_client_new(ref PamojaCoapConfig config);

    /// <summary>Binds the local socket of a CoAP endpoint.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_coap_client_connect(IntPtr client);

    /// <summary>Sends a payload to a CoAP resource path.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_coap_client_send(
        IntPtr client,
        IntPtr topic,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Observes a CoAP resource path.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_coap_client_subscribe(IntPtr client, IntPtr topic);

    /// <summary>Waits for the next message on an observed CoAP path.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_coap_client_recv(
        IntPtr client,
        out IntPtr outMessage);

    /// <summary>Reports whether a CoAP endpoint is bound.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_coap_client_is_connected(IntPtr client);

    /// <summary>Releases the socket a CoAP endpoint holds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_coap_client_disconnect(IntPtr client);

    /// <summary>Releases a CoAP endpoint handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_coap_client_free(IntPtr client);

    /// <summary>Creates an in-process broker.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_loopback_broker_new();

    /// <summary>Releases a broker handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_loopback_broker_free(IntPtr broker);

    /// <summary>Creates a link to a broker.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_loopback_transport_new(IntPtr broker);

    /// <summary>Marks a loopback link connected.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_loopback_transport_connect(IntPtr transport);

    /// <summary>Publishes a payload on a loopback link.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_loopback_transport_send(
        IntPtr transport,
        IntPtr topic,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Subscribes a loopback link to a topic.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_loopback_transport_subscribe(
        IntPtr transport,
        IntPtr topic);

    /// <summary>Waits for the next message on a loopback link.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_loopback_transport_recv(
        IntPtr transport,
        out IntPtr outMessage);

    /// <summary>Reports whether a loopback link is connected.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_loopback_transport_is_connected(IntPtr transport);

    /// <summary>Marks a loopback link disconnected.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_loopback_transport_disconnect(IntPtr transport);

    /// <summary>Releases a loopback link handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_loopback_transport_free(IntPtr transport);

    /// <summary>Creates a buffer held in memory.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_store_memory(nuint capacity);

    /// <summary>Opens a buffer backed by a directory.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_store_file(IntPtr dir);

    /// <summary>Adds a record to the end of a buffer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_store_append(
        IntPtr store,
        ReadOnlySpan<byte> record,
        nuint recordLen);

    /// <summary>Reads the oldest record without removing it.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_store_peek(IntPtr store, out IntPtr outRecord);

    /// <summary>Removes and returns the oldest record.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_store_pop(IntPtr store, out IntPtr outRecord);

    /// <summary>Reports how many records a buffer holds.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_store_len(IntPtr store, out nuint outLen);

    /// <summary>Sends every held record over a transport.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_store_drain_to(
        IntPtr store,
        IntPtr transport,
        IntPtr topic,
        out nuint outSent);

    /// <summary>Releases a store handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_store_free(IntPtr store);

    /// <summary>Creates a ladder buffering into a store.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ladder_new(IntPtr store);

    /// <summary>Adds a rung to a ladder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ladder_rung(IntPtr ladder, IntPtr transport);

    /// <summary>Connects every rung of a ladder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ladder_connect(IntPtr ladder);

    /// <summary>Sends a payload through a ladder.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ladder_send(
        IntPtr ladder,
        IntPtr topic,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out PamojaDelivery outDelivery);

    /// <summary>Replays the buffer of a ladder over its rungs.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ladder_flush(IntPtr ladder, out nuint outSent);

    /// <summary>Reports how many messages a ladder has buffered.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ladder_buffered(IntPtr ladder, out nuint outCount);

    /// <summary>Releases a ladder handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ladder_free(IntPtr ladder);

    /// <summary>Creates an event bus.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_event_bus_new(nuint capacity);

    /// <summary>Takes another endpoint on an event bus.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_event_bus_subscribe(IntPtr bus);

    /// <summary>Publishes an event to every subscriber.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_event_bus_publish(
        IntPtr bus,
        ReadOnlySpan<byte> payload,
        nuint payloadLen);

    /// <summary>Waits for the next event on an endpoint.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_event_bus_next(IntPtr bus, out IntPtr outEvent);

    /// <summary>Releases an event bus endpoint.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_event_bus_free(IntPtr bus);

    /// <summary>Creates a sensor that reads around a baseline.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_sim_sensor_new(
        float baseline,
        float driftPerRead,
        float noise,
        uint seed);

    /// <summary>Takes the next reading from a simulated sensor.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sim_sensor_read(IntPtr sensor, out float outReading);

    /// <summary>Releases a simulated sensor handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_sim_sensor_free(IntPtr sensor);

    /// <summary>Creates a sensor that reads back a recorded series.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_replay_new(
        ReadOnlySpan<float> readings,
        nuint count,
        [MarshalAs(UnmanagedType.U1)] bool repeating);

    /// <summary>Takes the next reading from a replay.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_replay_read(IntPtr replay, out float outReading);

    /// <summary>Releases a replay handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_replay_free(IntPtr replay);

    /// <summary>Creates an actuator that records every command.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_recording_actuator_new();

    /// <summary>Applies a command, which is recorded rather than acted on.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_recording_actuator_apply(
        IntPtr actuator,
        float command);

    /// <summary>Reports how many commands an actuator has been given.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_recording_actuator_len(IntPtr actuator);

    /// <summary>Copies out the commands an actuator recorded.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_recording_actuator_commands(
        IntPtr actuator,
        Span<float> outCommands,
        nuint capacity);

    /// <summary>Releases a recording actuator handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_recording_actuator_free(IntPtr actuator);

    /// <summary>Creates a robot that moves only in arithmetic.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_sim_robot_new(PamojaPose start, float dt);

    /// <summary>Drives a simulated robot for one time step.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_sim_robot_apply(IntPtr robot, PamojaTwist command);

    /// <summary>Reads where a simulated robot has got to.</summary>
    [LibraryImport(Library)]
    public static partial PamojaPose pamoja_sim_robot_pose(IntPtr robot);

    /// <summary>Releases a simulated robot handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_sim_robot_free(IntPtr robot);
}
