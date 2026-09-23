using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Core;

/// <summary>A message that arrived on a subscribed topic.</summary>
/// <param name="Topic">The topic it was published to.</param>
/// <param name="Payload">The raw payload bytes.</param>
public sealed record TransportMessage(string Topic, byte[] Payload)
{
    /// <summary>Gets the payload as text: words, or a number written out.</summary>
    public string Text => System.Text.Encoding.UTF8.GetString(Payload);

    /// <summary>Gets the payload as a number written out as text, such as <c>21.5</c>, or <c>null</c> when it is not one.</summary>
    public double? Number =>
        double.TryParse(
            Text.Trim(),
            System.Globalization.NumberStyles.Float,
            System.Globalization.CultureInfo.InvariantCulture,
            out double number)
            ? number
            : null;
}

/// <summary>One transport, ready to compose into a ladder or a wrapper.</summary>
/// <remarks>
/// A ladder rung, a fault injector, and a degraded link all take some transport,
/// which a C ABI cannot express, so one handle carries whichever kind was built.
/// Composing consumes it: the thing it is composed into owns it from then on, so
/// a spent transport throws rather than aliasing a link it no longer holds. A link
/// written in .NET enters through <see cref="FromHandlers"/> and is one of these
/// from then on.
///
/// Calls on one transport run one at a time, so a send made while a receive is
/// waiting runs once the receive returns. A task that listens should have a link
/// of its own.
/// </remarks>
public sealed class Transport : IDisposable
{
    private const string Spent = "this transport was already added to a ladder or a wrapper";
    private const string Busy = "this transport is busy with a call";

    private NativeHandle? _handle;

    /// <summary>Wraps a native transport handle.</summary>
    /// <param name="handle">The pointer a native call produced.</param>
    /// <param name="what">What was being created, for the exception message.</param>
    /// <exception cref="PamojaException">The native call produced no transport.</exception>
    public Transport(IntPtr handle, string what)
    {
        _handle = NativeHandle.Create(
            handle, NativeMethods.pamoja_transport_free, what, serialized: true);
    }

    /// <summary>Whether this transport is still holdable, or has been handed on.</summary>
    public bool IsAvailable => _handle is { IsClosed: false };

    /// <summary>Wraps a link written in .NET as a transport.</summary>
    /// <remarks>
    /// The handlers are held for the life of the transport and every transport
    /// composed from it, and released once nothing will call them again. A
    /// <see cref="IReceivingTransportHandlers"/> is listened on; plain
    /// <see cref="ITransportHandlers"/> only send, and a ladder never listens on them.
    /// </remarks>
    /// <param name="handlers">The link.</param>
    /// <returns>A transport whose every operation is one of the handlers.</returns>
    /// <exception cref="PamojaException">The native transport could not be created.</exception>
    public static unsafe Transport FromHandlers(ITransportHandlers handlers)
    {
        ArgumentNullException.ThrowIfNull(handlers);
        GCHandle handle = GCHandle.Alloc(handlers);
        var callbacks = new PamojaTransportCallbacks
        {
            Connect = &HostThunks.Connect,
            Send = &HostThunks.Send,
            Subscribe = &HostThunks.Subscribe,
            Recv = handlers is IReceivingTransportHandlers ? &HostThunks.Receive : null,
            Release = &HostThunks.Release,
        };
        IntPtr native = NativeMethods.pamoja_transport_from_callbacks(
            ref callbacks, GCHandle.ToIntPtr(handle));
        if (native == IntPtr.Zero)
        {
            handle.Free();
            throw new PamojaException(Status.LastError() ?? "failed to create the host transport");
        }

        return new Transport(native, "host transport");
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
    public Task ConnectAsync() => Live().UseAsync(handle =>
        Status.ThrowIfError(NativeMethods.pamoja_transport_connect(handle)));

    /// <summary>Sends text to a topic over this transport: words, or a number written out.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="text">The text to send, as UTF-8.</param>
    /// <exception cref="PamojaException">The transport would not take it.</exception>
    public Task SendAsync(string topic, string text) =>
        SendAsync(topic, System.Text.Encoding.UTF8.GetBytes(text));

    /// <summary>Sends a payload to a topic over this transport.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <exception cref="PamojaException">The transport would not take it.</exception>
    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        byte[] bytes = payload.ToArray();
        return Live().UseAsync(handle =>
        {
            IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
            try
            {
                Status.ThrowIfError(NativeMethods.pamoja_transport_send(
                    handle, topicPtr, bytes, (nuint)bytes.Length));
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
    public Task SubscribeAsync(string topic) => Live().UseAsync(handle =>
    {
        IntPtr topicPtr = Marshal.StringToCoTaskMemUTF8(topic);
        try
        {
            Status.ThrowIfError(NativeMethods.pamoja_transport_subscribe(handle, topicPtr));
        }
        finally
        {
            Marshal.FreeCoTaskMem(topicPtr);
        }
    });

    /// <summary>Waits for the next message this transport delivers on a subscribed topic.</summary>
    /// <returns>The message, or <c>null</c> once the link has ended.</returns>
    /// <exception cref="PamojaException">The transport is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync() => Live().UseAsync(handle =>
    {
        Status.ThrowIfError(NativeMethods.pamoja_transport_recv(handle, out IntPtr message));
        return Messages.Take(message);
    });

    /// <summary>Waits a limited time for the next message on a subscribed topic.</summary>
    /// <remarks>
    /// When the time runs out nothing is lost: a message arriving afterwards waits
    /// for the next receive.
    /// </remarks>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The message, or <c>null</c> once the link has ended.</returns>
    /// <exception cref="TimeoutException">No message arrived in time.</exception>
    /// <exception cref="PamojaException">The transport is not connected.</exception>
    public Task<TransportMessage?> ReceiveAsync(TimeSpan timeout)
    {
        ulong milliseconds = Messages.Milliseconds(timeout);
        return Live().UseAsync(handle =>
        {
            Status.ThrowIfError(NativeMethods.pamoja_transport_recv_within(
                handle, milliseconds, out IntPtr message, out bool timedOut));
            return timedOut ? throw Messages.TimedOut(timeout) : Messages.Take(message);
        });
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        _handle?.Dispose();
        _handle = null;
    }

    /// <summary>Hands the native handle on, leaving this one spent.</summary>
    /// <returns>The pointer the caller now owns.</returns>
    /// <exception cref="PamojaException">
    /// This transport was already handed on, or a call on it is still running.
    /// </exception>
    public IntPtr Take()
    {
        IntPtr pointer = Live().Take(Busy);
        _handle = null;
        return pointer;
    }

    /// <summary>
    /// Lends the native handle to asynchronous work, such as a store draining into
    /// this transport, which runs once any call on the transport has returned.
    /// </summary>
    /// <typeparam name="TResult">The value the work produces.</typeparam>
    /// <param name="work">The work to run with the pointer.</param>
    /// <returns>Whatever the work produced.</returns>
    /// <exception cref="PamojaException">This transport was already handed on.</exception>
    public Task<TResult> LendAsync<TResult>(Func<IntPtr, Task<TResult>> work) =>
        Live().LendAsync(work);

    /// <summary>Returns the handle, refusing one that has been handed on.</summary>
    private NativeHandle Live() => _handle is { IsClosed: false } handle
        ? handle
        : throw new PamojaException(Spent);
}
