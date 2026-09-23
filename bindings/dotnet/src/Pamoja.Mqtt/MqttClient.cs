using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Mqtt;

/// <summary>
/// An MQTT client transport, the ergonomic facade over the native pamoja core.
/// </summary>
/// <remarks>
/// The native C ABI is synchronous, so every operation runs on the thread pool and
/// is awaited here; failures surface as <see cref="PamojaException"/>. Construct the
/// client with broker settings, <see cref="ConnectAsync"/>, then
/// <see cref="PublishAsync(string, ReadOnlyMemory{byte})"/>,
/// <see cref="SubscribeAsync"/>, and read inbound messages with
/// <see cref="RecvAsync()"/> or by iterating the client with <c>await foreach</c>.
///
/// Calls on one client run one at a time, so a publish made while a receive is
/// waiting runs once the receive returns. A client that both listens and publishes
/// receives with a time limit, or iterates, and publishes between messages.
/// </remarks>
/// <example>
/// <code>
/// await using var client = new MqttClient(new MqttClientOptions
/// {
///     ClientId = "sensor-1",
///     Host = "localhost",
///     Port = 1883,
/// });
/// await client.ConnectAsync();
/// await client.SubscribeAsync("sensors/+/temperature");
/// await client.PublishAsync("sensors/1/temperature", "21.5");
/// await foreach (var message in client)
/// {
///     Console.WriteLine($"{message.Topic}: {message.Payload.Length} bytes");
/// }
/// </code>
/// </example>
public sealed class MqttClient : IAsyncEnumerable<MqttMessage>, IAsyncDisposable, IDisposable
{
    private static readonly TimeSpan CancellationCheck = TimeSpan.FromMilliseconds(250);

    private readonly NativeHandle _handle;

    /// <summary>Creates a disconnected client from the given options.</summary>
    /// <param name="options">The broker connection settings.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is null.</exception>
    /// <exception cref="PamojaException">The native client could not be created.</exception>
    public MqttClient(MqttClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);

        IntPtr clientId = Marshal.StringToCoTaskMemUTF8(options.ClientId);
        IntPtr host = Marshal.StringToCoTaskMemUTF8(options.Host);
        try
        {
            var config = new PamojaMqttConfig
            {
                ClientId = clientId,
                Host = host,
                Port = options.Port,
                KeepAliveSecs = options.KeepAliveSecs ?? 0,
                Capacity = options.Capacity ?? 0,
                Qos = (PamojaQos)(int)(options.Qos ?? Qos.AtLeastOnce),
                MaxPacketSize = options.MaxPacketSize ?? 0,
            };

            _handle = NativeHandle.Create(
                NativeMethods.pamoja_mqtt_client_new(ref config),
                NativeMethods.pamoja_mqtt_client_free,
                "MQTT client",
                serialized: true);
        }
        finally
        {
            Marshal.FreeCoTaskMem(clientId);
            Marshal.FreeCoTaskMem(host);
        }
    }

    /// <summary>Connects to the broker and starts the background event loop.</summary>
    /// <returns>A task that completes once connected.</returns>
    /// <exception cref="PamojaException">The connection could not be established.</exception>
    public Task ConnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_connect(handle)));

    /// <summary>Publishes a payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body.</param>
    /// <returns>A task that completes once the payload is handed to the transport.</returns>
    /// <exception cref="PamojaException">The payload could not be sent.</exception>
    public Task PublishAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        ArgumentNullException.ThrowIfNull(topic);
        byte[] bytes = payload.ToArray();
        return _handle.UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            IntPtr payloadPtr = IntPtr.Zero;
            try
            {
                if (bytes.Length > 0)
                {
                    payloadPtr = Marshal.AllocCoTaskMem(bytes.Length);
                    Marshal.Copy(bytes, 0, payloadPtr, bytes.Length);
                }

                Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_publish(
                    handle, topicPtr, payloadPtr, (nuint)bytes.Length));
            }
            finally
            {
                if (payloadPtr != IntPtr.Zero)
                {
                    Marshal.FreeCoTaskMem(payloadPtr);
                }

                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Publishes a UTF-8 string payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body, encoded as UTF-8.</param>
    /// <returns>A task that completes once the payload is handed to the transport.</returns>
    /// <exception cref="PamojaException">The payload could not be sent.</exception>
    public Task PublishAsync(string topic, string payload)
    {
        ArgumentNullException.ThrowIfNull(payload);
        return PublishAsync(topic, System.Text.Encoding.UTF8.GetBytes(payload));
    }

    /// <summary>Subscribes to a topic filter.</summary>
    /// <param name="topic">The topic or wildcard filter to subscribe to.</param>
    /// <returns>A task that completes once the subscription is registered.</returns>
    /// <exception cref="PamojaException">The subscription was rejected.</exception>
    public Task SubscribeAsync(string topic)
    {
        ArgumentNullException.ThrowIfNull(topic);
        return _handle.UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_subscribe(handle, topicPtr));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Awaits the next message from any subscribed topic.</summary>
    /// <returns>The next message, or <c>null</c> once the connection has ended.</returns>
    /// <exception cref="PamojaException">
    /// The client is not connected, or, once, saying why, the connection ended on its own
    /// because the broker went away or another client connected with the same id.
    /// </exception>
    public Task<MqttMessage?> RecvAsync() => _handle.UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_recv(handle, out IntPtr message));
        return TakeMessage(message);
    });

    /// <summary>Waits a limited time for the next message from any subscribed topic.</summary>
    /// <remarks>
    /// When the time runs out nothing is lost: a message arriving afterwards waits
    /// for the next receive.
    /// </remarks>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The next message, or <c>null</c> once the connection has ended.</returns>
    /// <exception cref="TimeoutException">No message arrived in time.</exception>
    /// <exception cref="PamojaException">
    /// The client is not connected, or the connection ended on its own.
    /// </exception>
    public Task<MqttMessage?> RecvAsync(TimeSpan timeout)
    {
        ulong milliseconds = Pamoja.Core.Messages.Milliseconds(timeout);
        return _handle.UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_recv_within(
                handle, milliseconds, out IntPtr message, out bool timedOut));
            return timedOut ? throw Pamoja.Core.Messages.TimedOut(timeout) : TakeMessage(message);
        });
    }

    /// <summary>Reports whether the client currently holds an active connection.</summary>
    /// <returns>A task resolving to the connection state.</returns>
    public Task<bool> IsConnectedAsync() =>
        _handle.UseAsync(NativeMethods.pamoja_mqtt_client_is_connected);

    /// <summary>Closes the connection and stops the background event loop.</summary>
    /// <returns>A task that completes once the client has disconnected.</returns>
    /// <exception cref="PamojaException">The disconnect failed.</exception>
    public Task DisconnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_disconnect(handle)));

    /// <summary>Yields messages from subscribed topics until the connection ends.</summary>
    /// <remarks>
    /// The client is free between messages, so the body of an <c>await foreach</c>
    /// can publish, and canceling stops the wait within a quarter of a second. A
    /// connection that ends on its own throws its reason out of the loop.
    /// </remarks>
    /// <param name="cancellationToken">Stops iteration when canceled.</param>
    /// <returns>An async stream over incoming messages.</returns>
    public async IAsyncEnumerable<MqttMessage> Messages(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            MqttMessage? message;
            try
            {
                message = await RecvAsync(CancellationCheck).ConfigureAwait(false);
            }
            catch (TimeoutException)
            {
                continue;
            }

            if (message is null)
            {
                yield break;
            }

            yield return message;
        }
    }

    /// <summary>Iterates incoming messages, so the client can be used with <c>await foreach</c>.</summary>
    /// <param name="cancellationToken">Stops iteration when canceled.</param>
    /// <returns>An async enumerator over incoming messages.</returns>
    public IAsyncEnumerator<MqttMessage> GetAsyncEnumerator(
        CancellationToken cancellationToken = default) =>
        Messages(cancellationToken).GetAsyncEnumerator(cancellationToken);

    /// <summary>Disconnects (best-effort) and releases the native client.</summary>
    /// <returns>A task that completes once the client has been released.</returns>
    public async ValueTask DisposeAsync()
    {
        try
        {
            await DisconnectAsync().ConfigureAwait(false);
        }
        catch (PamojaException)
        {
        }

        _handle.Dispose();
        GC.SuppressFinalize(this);
    }

    /// <summary>Releases the native client.</summary>
    public void Dispose()
    {
        _handle.Dispose();
        GC.SuppressFinalize(this);
    }

    /// <summary>Copies a native message out and releases it.</summary>
    private static MqttMessage? TakeMessage(IntPtr message)
    {
        if (message == IntPtr.Zero)
        {
            return null;
        }

        try
        {
            string topic =
                Marshal.PtrToStringUTF8(NativeMethods.pamoja_mqtt_message_topic(message))
                ?? string.Empty;
            int length = checked((int)NativeMethods.pamoja_mqtt_message_payload_len(message));
            byte[] payload = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_mqtt_message_payload(message), payload, 0, length);
            }

            return new MqttMessage(topic, payload);
        }
        finally
        {
            NativeMethods.pamoja_mqtt_message_free(message);
        }
    }
}
