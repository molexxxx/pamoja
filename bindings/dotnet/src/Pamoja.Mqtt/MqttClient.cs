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
/// <see cref="RecvAsync"/> or by iterating the client with <c>await foreach</c>.
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
    private readonly MqttClientHandle _handle;

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
            };

            IntPtr client = NativeMethods.pamoja_mqtt_client_new(ref config);
            if (client == IntPtr.Zero)
            {
                throw new PamojaException(
                    Status.LastError() ?? "failed to create the MQTT client");
            }

            _handle = new MqttClientHandle(client);
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
    public Task ConnectAsync() =>
        InvokeAsync(NativeMethods.pamoja_mqtt_client_connect);

    /// <summary>Publishes a payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body.</param>
    /// <returns>A task that completes once the payload is handed to the transport.</returns>
    /// <exception cref="PamojaException">The payload could not be sent.</exception>
    public Task PublishAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        ArgumentNullException.ThrowIfNull(topic);
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            bool added = false;
            _handle.DangerousAddRef(ref added);
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            IntPtr payloadPtr = IntPtr.Zero;
            try
            {
                PamojaStatus status;
                if (bytes.Length == 0)
                {
                    status = NativeMethods.pamoja_mqtt_client_publish(
                        _handle.DangerousGetHandle(), topicPtr, IntPtr.Zero, 0);
                }
                else
                {
                    payloadPtr = Marshal.AllocCoTaskMem(bytes.Length);
                    Marshal.Copy(bytes, 0, payloadPtr, bytes.Length);
                    status = NativeMethods.pamoja_mqtt_client_publish(
                        _handle.DangerousGetHandle(), topicPtr, payloadPtr, (nuint)bytes.Length);
                }

                Status.ThrowIfError(status);
            }
            finally
            {
                if (payloadPtr != IntPtr.Zero)
                {
                    Marshal.FreeCoTaskMem(payloadPtr);
                }

                Marshal.FreeCoTaskMem(topicPtr);
                if (added)
                {
                    _handle.DangerousRelease();
                }
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
        return Task.Run(() =>
        {
            bool added = false;
            _handle.DangerousAddRef(ref added);
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                PamojaStatus status = NativeMethods.pamoja_mqtt_client_subscribe(
                    _handle.DangerousGetHandle(), topicPtr);
                Status.ThrowIfError(status);
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
                if (added)
                {
                    _handle.DangerousRelease();
                }
            }
        });
    }

    /// <summary>Awaits the next message from any subscribed topic.</summary>
    /// <returns>The next message, or <c>null</c> once the connection has ended.</returns>
    /// <exception cref="PamojaException">The client is not connected.</exception>
    public Task<MqttMessage?> RecvAsync()
    {
        return Task.Run<MqttMessage?>(() =>
        {
            bool added = false;
            _handle.DangerousAddRef(ref added);
            try
            {
                PamojaStatus status = NativeMethods.pamoja_mqtt_client_recv(
                    _handle.DangerousGetHandle(), out IntPtr message);
                Status.ThrowIfError(status);

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
                        Marshal.Copy(
                            NativeMethods.pamoja_mqtt_message_payload(message), payload, 0, length);
                    }

                    return new MqttMessage(topic, payload);
                }
                finally
                {
                    NativeMethods.pamoja_mqtt_message_free(message);
                }
            }
            finally
            {
                if (added)
                {
                    _handle.DangerousRelease();
                }
            }
        });
    }

    /// <summary>Reports whether the client currently holds an active connection.</summary>
    /// <returns>A task resolving to the connection state.</returns>
    public Task<bool> IsConnectedAsync()
    {
        return Task.Run(() =>
        {
            bool added = false;
            _handle.DangerousAddRef(ref added);
            try
            {
                return NativeMethods.pamoja_mqtt_client_is_connected(_handle.DangerousGetHandle());
            }
            finally
            {
                if (added)
                {
                    _handle.DangerousRelease();
                }
            }
        });
    }

    /// <summary>Closes the connection and stops the background event loop.</summary>
    /// <returns>A task that completes once the client has disconnected.</returns>
    /// <exception cref="PamojaException">The disconnect failed.</exception>
    public Task DisconnectAsync() =>
        InvokeAsync(NativeMethods.pamoja_mqtt_client_disconnect);

    /// <summary>Yields messages from subscribed topics until the connection ends.</summary>
    /// <param name="cancellationToken">Stops iteration when cancelled.</param>
    /// <returns>An async stream over incoming messages.</returns>
    public async IAsyncEnumerable<MqttMessage> Messages(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            MqttMessage? message = await RecvAsync().ConfigureAwait(false);
            if (message is null)
            {
                yield break;
            }

            yield return message;
        }
    }

    /// <summary>Iterates incoming messages, so the client can be used with <c>await foreach</c>.</summary>
    /// <param name="cancellationToken">Stops iteration when cancelled.</param>
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
            // Disconnect is best-effort during disposal.
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

    /// <summary>
    /// Runs a unit-returning native call on the thread pool, holding the handle alive
    /// for the call and throwing on a non-OK status read on the same thread.
    /// </summary>
    private Task InvokeAsync(Func<IntPtr, PamojaStatus> call)
    {
        return Task.Run(() =>
        {
            bool added = false;
            _handle.DangerousAddRef(ref added);
            try
            {
                Status.ThrowIfError(call(_handle.DangerousGetHandle()));
            }
            finally
            {
                if (added)
                {
                    _handle.DangerousRelease();
                }
            }
        });
    }
}
