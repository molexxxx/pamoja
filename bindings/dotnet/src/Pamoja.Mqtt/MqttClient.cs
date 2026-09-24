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
/// <see cref="PublishAsync(string, ReadOnlyMemory{byte}, MqttPublishOptions)"/>,
/// <see cref="SubscribeAsync"/>, and read inbound messages with
/// <see cref="RecvAsync()"/> or by iterating the client with <c>await foreach</c>.
///
/// A receive waits apart from the rest of the client, so one task can wait for
/// commands while another publishes readings on the same client.
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
    private readonly Qos _qos;

    /// <summary>Creates a disconnected client from the given options.</summary>
    /// <param name="options">The broker connection settings.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is null.</exception>
    /// <exception cref="PamojaException">The native client could not be created.</exception>
    /// <exception cref="ArgumentOutOfRangeException">A QoS is not one of the <see cref="Qos"/> values.</exception>
    /// <exception cref="ArgumentException">A password comes without a username, or a client certificate without its key.</exception>
    public MqttClient(MqttClientOptions options)
    {
        using var native = new NativeMqttOptions(options);
        _qos = options.Qos ?? Qos.AtLeastOnce;

        // The native client locks its own state and receives apart from it, so calls on
        // one client may run together.
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mqtt_client_new(ref native.Config),
            NativeMethods.pamoja_mqtt_client_free,
            "MQTT client",
            serialized: false);
    }

    /// <summary>Connects to the broker and starts the background event loop.</summary>
    /// <returns>A task that completes once connected.</returns>
    /// <exception cref="PamojaException">The connection could not be established.</exception>
    public Task ConnectAsync() => _handle.UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_connect(handle)));

    /// <summary>Publishes a payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body.</param>
    /// <param name="options">The quality of service and retain flag for this message.</param>
    /// <returns>
    /// A task that completes once the payload is queued for the broker, before the broker
    /// acknowledges it; <see cref="PublishConfirmedAsync(string, ReadOnlyMemory{byte}, MqttPublishOptions)"/>
    /// waits for that.
    /// </returns>
    /// <exception cref="PamojaException">The payload could not be sent.</exception>
    public Task PublishAsync(string topic, ReadOnlyMemory<byte> payload, MqttPublishOptions options = default) =>
        Publish(topic, payload, options, confirmed: false);

    /// <summary>Publishes a UTF-8 string payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body, encoded as UTF-8.</param>
    /// <param name="options">The quality of service and retain flag for this message.</param>
    /// <returns>A task that completes once the payload is queued for the broker.</returns>
    /// <exception cref="PamojaException">The payload could not be sent.</exception>
    public Task PublishAsync(string topic, string payload, MqttPublishOptions options = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        return PublishAsync(topic, System.Text.Encoding.UTF8.GetBytes(payload), options);
    }

    /// <summary>Publishes a payload to a topic and waits for the broker to acknowledge it.</summary>
    /// <remarks>
    /// The broker answers with a <c>PUBACK</c> at <see cref="Qos.AtLeastOnce"/> and a
    /// <c>PUBCOMP</c> at <see cref="Qos.ExactlyOnce"/>; at <see cref="Qos.AtMostOnce"/> MQTT
    /// acknowledges nothing, so the task completes once the connection has taken the message.
    /// </remarks>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body.</param>
    /// <param name="options">The quality of service and retain flag for this message.</param>
    /// <returns>A task that completes once the broker holds the message.</returns>
    /// <exception cref="PamojaException">
    /// The payload could not be sent, or the connection ended before the acknowledgment, when
    /// the message may or may not have arrived.
    /// </exception>
    public Task PublishConfirmedAsync(string topic, ReadOnlyMemory<byte> payload, MqttPublishOptions options = default) =>
        Publish(topic, payload, options, confirmed: true);

    /// <summary>Publishes a UTF-8 string payload and waits for the broker to acknowledge it.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The message body, encoded as UTF-8.</param>
    /// <param name="options">The quality of service and retain flag for this message.</param>
    /// <returns>A task that completes once the broker holds the message.</returns>
    /// <exception cref="PamojaException">The payload could not be sent or was not acknowledged.</exception>
    public Task PublishConfirmedAsync(string topic, string payload, MqttPublishOptions options = default)
    {
        ArgumentNullException.ThrowIfNull(payload);
        return PublishConfirmedAsync(topic, System.Text.Encoding.UTF8.GetBytes(payload), options);
    }

    private Task Publish(string topic, ReadOnlyMemory<byte> payload, MqttPublishOptions options, bool confirmed)
    {
        ArgumentNullException.ThrowIfNull(topic);
        byte[] bytes = payload.ToArray();
        var qos = (PamojaQos)NamedValue.Require(options.Qos ?? _qos, nameof(options.Qos));
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

                Status.ThrowIfError(NativeMethods.pamoja_mqtt_client_publish_with(
                    handle, topicPtr, payloadPtr, (nuint)bytes.Length, qos, options.Retain, confirmed));
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
