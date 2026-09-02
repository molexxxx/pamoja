using System.Runtime.InteropServices;

using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>A message that arrived on a subscribed topic.</summary>
/// <param name="Topic">The topic it was published to.</param>
/// <param name="Payload">The raw payload bytes.</param>
public sealed record TransportMessage(string Topic, byte[] Payload);

/// <summary>One transport, ready to compose into a ladder or a wrapper.</summary>
/// <remarks>
/// A ladder rung, a fault injector, and a degraded link all take some transport,
/// which a C ABI cannot express, so one handle carries whichever kind was built.
/// Composing consumes it: the thing it is composed into owns it from then on, so
/// a spent transport throws rather than aliasing a link it no longer holds.
/// </remarks>
public sealed class Transport : IDisposable
{
    private IntPtr _handle;

    /// <summary>Wraps a native transport handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    /// <param name="what">What was being created, for the exception message.</param>
    internal Transport(IntPtr handle, string what)
    {
        if (handle == IntPtr.Zero)
        {
            throw new PamojaException(PamojaCore.LastError() ?? $"failed to create the {what}");
        }

        _handle = handle;
    }

    /// <summary>Whether this transport is still holdable, or has been handed on.</summary>
    public bool IsAvailable => _handle != IntPtr.Zero;

    /// <summary>Creates a transport that reaches a broker over MQTT.</summary>
    /// <param name="options">The broker settings.</param>
    /// <returns>The transport, ready to add as a rung.</returns>
    public static Transport Mqtt(MqttClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        IntPtr clientId = Marshal.StringToCoTaskMemUTF8(options.ClientId);
        IntPtr host = Marshal.StringToCoTaskMemUTF8(options.Host);
        try
        {
            PamojaMqttConfig config = new()
            {
                ClientId = clientId,
                Host = host,
                Port = options.Port,
                KeepAliveSecs = options.KeepAliveSecs ?? 0,
                Capacity = options.Capacity ?? 0,
                Qos = (PamojaQos)(options.Qos ?? Core.Qos.AtLeastOnce),
            };
            return new Transport(NativeMethods.pamoja_transport_mqtt(ref config), "MQTT transport");
        }
        finally
        {
            Marshal.FreeCoTaskMem(clientId);
            Marshal.FreeCoTaskMem(host);
        }
    }

    /// <summary>Creates a transport that reaches a peer over CoAP.</summary>
    /// <param name="options">The endpoint settings.</param>
    /// <returns>The transport, ready to add as a rung.</returns>
    public static Transport Coap(CoapClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        return options.WithNativeConfig(
            (ref PamojaCoapConfig config) => new Transport(
                NativeMethods.pamoja_transport_coap(ref config),
                "CoAP transport"));
    }

    /// <summary>Wraps a transport so a set number of its next sends fail.</summary>
    /// <remarks>
    /// This is how a caller checks that a ladder falls through to its next rung,
    /// or that a buffer fills, without unplugging anything.
    /// </remarks>
    /// <param name="inner">The transport to wrap, consumed by this call.</param>
    /// <param name="failures">How many upcoming sends to fail.</param>
    /// <returns>A transport owning the wrapped one.</returns>
    public static Transport Faulty(Transport inner, int failures)
    {
        ArgumentNullException.ThrowIfNull(inner);
        return new Transport(
            NativeMethods.pamoja_transport_faulty(inner.Take(), (nuint)failures),
            "faulty transport");
    }

    /// <summary>Wraps a transport in a link that loses packets and goes down.</summary>
    /// <param name="inner">The transport to wrap, consumed by this call.</param>
    /// <param name="dropEvery">Lose one send in every this many, or 0 to lose none.</param>
    /// <param name="up">How many sends the link stays up for, or 0 to never go down.</param>
    /// <param name="down">How many sends it then stays down for.</param>
    /// <returns>A transport owning the wrapped one.</returns>
    public static Transport Degraded(Transport inner, uint dropEvery = 0, uint up = 0, uint down = 0)
    {
        ArgumentNullException.ThrowIfNull(inner);
        return new Transport(
            NativeMethods.pamoja_transport_degraded(inner.Take(), dropEvery, up, down),
            "degraded transport");
    }

    /// <summary>Connects this transport.</summary>
    /// <exception cref="PamojaException">The link could not be established.</exception>
    public Task ConnectAsync() =>
        Task.Run(() => PamojaCore.ThrowIfError(
            NativeMethods.pamoja_transport_connect(Live())));

    /// <summary>Sends a payload to a topic over this transport.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <exception cref="PamojaException">The transport would not take it.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                PamojaCore.ThrowIfError(NativeMethods.pamoja_transport_send(
                    Live(), topicPtr, bytes, (nuint)bytes.Length));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Subscribes this transport to a topic.</summary>
    /// <param name="topic">The topic to subscribe to.</param>
    /// <exception cref="PamojaException">The subscription was refused.</exception>
    public Task SubscribeAsync(string topic) => Task.Run(() =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            PamojaCore.ThrowIfError(
                NativeMethods.pamoja_transport_subscribe(Live(), topicPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeMethods.pamoja_transport_free(_handle);
            _handle = IntPtr.Zero;
        }
    }

    /// <summary>Hands the native handle on, leaving this one spent.</summary>
    /// <returns>The pointer the caller now owns.</returns>
    /// <exception cref="PamojaException">This transport was already handed on.</exception>
    internal IntPtr Take()
    {
        IntPtr handle = Live();
        _handle = IntPtr.Zero;
        return handle;
    }

    /// <summary>Lends the native handle without giving it away.</summary>
    /// <returns>The pointer, still owned by this transport.</returns>
    /// <exception cref="PamojaException">This transport was already handed on.</exception>
    internal IntPtr Borrow() => Live();

    /// <summary>Returns the handle, refusing one that has been handed on.</summary>
    private IntPtr Live() => _handle != IntPtr.Zero
        ? _handle
        : throw new PamojaException(
            "this transport was already added to a ladder or a wrapper");
}

/// <summary>An in-process broker.</summary>
/// <remarks>
/// Every transport built from one broker shares its traffic, so a message one
/// publishes reaches the others that subscribed to the topic. It exists so a
/// caller can exercise a whole message flow with no broker, no network, and no
/// hardware, which is what makes that flow testable from a unit test.
/// </remarks>
public sealed class LoopbackBroker : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a broker with no traffic.</summary>
    /// <exception cref="PamojaException">The native broker could not be created.</exception>
    public LoopbackBroker()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_loopback_broker_new(),
            NativeMethods.pamoja_loopback_broker_free,
            "loopback broker");
    }

    /// <summary>Creates a link to this broker, for driving directly.</summary>
    /// <returns>The link.</returns>
    public LoopbackTransport Link() =>
        new(_handle.Use(NativeMethods.pamoja_loopback_transport_new));

    /// <summary>Creates a link to this broker as a composable transport.</summary>
    /// <returns>The transport, ready to add as a rung.</returns>
    public Transport Rung() => new(
        _handle.Use(NativeMethods.pamoja_transport_loopback),
        "loopback transport");

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>One in-process link to a broker.</summary>
public sealed class LoopbackTransport : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a native link handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    internal LoopbackTransport(IntPtr handle)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_loopback_transport_free, "loopback link");
    }

    /// <summary>Marks this link connected so it will carry traffic.</summary>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task ConnectAsync() => Task.Run(() => PamojaCore.ThrowIfError(
        _handle.Use(NativeMethods.pamoja_loopback_transport_connect)));

    /// <summary>Publishes a payload to a topic on the broker.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to publish.</param>
    /// <exception cref="PamojaException">The link is not connected.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Task.Run(() =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                PamojaCore.ThrowIfError(_handle.Use(handle =>
                    NativeMethods.pamoja_loopback_transport_send(
                        handle, topicPtr, bytes, (nuint)bytes.Length)));
            }
            finally
            {
                Marshal.FreeCoTaskMem(topicPtr);
            }
        });
    }

    /// <summary>Subscribes this link to a topic.</summary>
    /// <param name="topic">The topic to subscribe to.</param>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task SubscribeAsync(string topic) => Task.Run(() =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            PamojaCore.ThrowIfError(_handle.Use(handle =>
                NativeMethods.pamoja_loopback_transport_subscribe(handle, topicPtr)));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message on a subscribed topic.</summary>
    /// <returns>The message, or <c>null</c> once the link is closed.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public Task<TransportMessage?> ReceiveAsync() => Task.Run(() =>
    {
        IntPtr message = IntPtr.Zero;
        PamojaCore.ThrowIfError(_handle.Use(handle =>
            NativeMethods.pamoja_loopback_transport_recv(handle, out message)));
        return Messages.Take(message);
    });

    /// <summary>Reports whether this link is connected.</summary>
    /// <returns><c>true</c> when connected.</returns>
    public Task<bool> IsConnectedAsync() => Task.Run(() =>
        _handle.Use(NativeMethods.pamoja_loopback_transport_is_connected));

    /// <summary>Marks this link disconnected, so sends over it fail.</summary>
    public Task DisconnectAsync() => Task.Run(() => _handle.Use(handle =>
    {
        NativeMethods.pamoja_loopback_transport_disconnect(handle);
        return 0;
    }));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Reading a native message handle and releasing it.</summary>
internal static class Messages
{
    /// <summary>Copies a message out and releases the handle.</summary>
    /// <param name="message">The handle, or null when nothing arrived.</param>
    /// <returns>The message, or <c>null</c> when the handle was null.</returns>
    internal static TransportMessage? Take(IntPtr message)
    {
        if (message == IntPtr.Zero)
        {
            return null;
        }

        try
        {
            string topic =
                Marshal.PtrToStringUTF8(NativeMethods.pamoja_message_topic(message)) ?? string.Empty;
            int length = checked((int)NativeMethods.pamoja_message_payload_len(message));
            byte[] payload = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(NativeMethods.pamoja_message_payload(message), payload, 0, length);
            }

            return new TransportMessage(topic, payload);
        }
        finally
        {
            NativeMethods.pamoja_message_free(message);
        }
    }
}
