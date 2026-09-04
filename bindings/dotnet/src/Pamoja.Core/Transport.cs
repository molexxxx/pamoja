using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

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
    public Transport(IntPtr handle, string what)
    {
        if (handle == IntPtr.Zero)
        {
            throw new PamojaException(Status.LastError() ?? $"failed to create the {what}");
        }

        _handle = handle;
    }

    /// <summary>Whether this transport is still holdable, or has been handed on.</summary>
    public bool IsAvailable => _handle != IntPtr.Zero;

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
        Task.Run(() => Status.ThrowIfError(
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
                Status.ThrowIfError(NativeMethods.pamoja_transport_send(
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
            Status.ThrowIfError(
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
    public IntPtr Take()
    {
        IntPtr handle = Live();
        _handle = IntPtr.Zero;
        return handle;
    }

    /// <summary>Lends the native handle without giving it away.</summary>
    /// <returns>The pointer, still owned by this transport.</returns>
    /// <exception cref="PamojaException">This transport was already handed on.</exception>
    public IntPtr Borrow() => Live();

    /// <summary>Returns the handle, refusing one that has been handed on.</summary>
    private IntPtr Live() => _handle != IntPtr.Zero
        ? _handle
        : throw new PamojaException(
            "this transport was already added to a ladder or a wrapper");
}
