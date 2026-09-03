using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the pamoja C ABI, mirroring <c>pamoja.h</c>
/// one-to-one.
/// </summary>
/// <remarks>
/// This is the low-level escape hatch (the .NET analog of <c>@pamoja/core/raw</c>
/// and <c>pamoja.raw</c>). The hand-written <c>MqttClient</c> facade is the
/// default entry point; anything it does not surface is reachable here. All
/// pointers are passed as <see cref="IntPtr"/>; the caller owns lifetime and string
/// encoding, exactly as the C header specifies.
/// </remarks>
public static partial class NativeMethods
{
    private const string Library = "pamoja_ffi";

    /// <summary>Returns the calling thread's most recent error message, or null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_last_error_message();

    /// <summary>Returns the version string of the native pamoja library.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_version();

    /// <summary>Creates a disconnected MQTT client, or returns null on failure.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mqtt_client_new(ref PamojaMqttConfig config);

    /// <summary>Connects to the broker and starts the background event loop.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mqtt_client_connect(IntPtr client);

    /// <summary>Publishes a payload to a topic.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mqtt_client_publish(
        IntPtr client,
        IntPtr topic,
        IntPtr payload,
        nuint payloadLen);

    /// <summary>Subscribes to a topic filter.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mqtt_client_subscribe(IntPtr client, IntPtr topic);

    /// <summary>Awaits the next message; sets <paramref name="outMessage"/> to null at end of stream.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mqtt_client_recv(IntPtr client, out IntPtr outMessage);

    /// <summary>Reports whether the client currently holds an active connection.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_mqtt_client_is_connected(IntPtr client);

    /// <summary>Closes the connection and stops the background event loop.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mqtt_client_disconnect(IntPtr client);

    /// <summary>Releases a client handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mqtt_client_free(IntPtr client);

    /// <summary>Returns a pointer to a message's null-terminated UTF-8 topic.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mqtt_message_topic(IntPtr message);

    /// <summary>Returns a pointer to a message's payload bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mqtt_message_payload(IntPtr message);

    /// <summary>Returns the length in bytes of a message's payload.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mqtt_message_payload_len(IntPtr message);

    /// <summary>Releases a message handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mqtt_message_free(IntPtr message);
}
