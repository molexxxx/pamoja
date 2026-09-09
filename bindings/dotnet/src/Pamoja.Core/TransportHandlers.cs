namespace Pamoja.Core;

/// <summary>The operations a link written in .NET supplies to stand as a transport.</summary>
/// <remarks>
/// Implement this for a link pamoja does not ship, a vendor SDK, a proprietary radio,
/// or a cloud client, and hand it to <see cref="Transport.FromHandlers"/>. The result
/// composes like any transport: as a ladder rung, under a fault injector, or driven
/// directly. Each method is awaited on a thread pamoja owns, one call at a time per
/// transport; an exception is reported to the caller as a failed operation with the
/// exception's message.
/// </remarks>
public interface ITransportHandlers
{
    /// <summary>Establishes the link.</summary>
    /// <returns>A task that completes once the link carries traffic.</returns>
    Task ConnectAsync();

    /// <summary>Publishes a payload to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <returns>A task that completes once the link has taken the payload.</returns>
    Task SendAsync(string topic, ReadOnlyMemory<byte> payload);

    /// <summary>Subscribes to a topic filter.</summary>
    /// <param name="topic">The filter, in the syntax the link understands.</param>
    /// <returns>A task that completes once the subscription is registered.</returns>
    Task SubscribeAsync(string topic);
}

/// <summary>A link that also delivers what it subscribed to.</summary>
/// <remarks>
/// A transport built from one of these is listened on: from the moment it connects,
/// <see cref="ReceiveAsync"/> is awaited again as soon as it returns, from a thread
/// pamoja owns, and what it delivers is queued for the receiving side. Return
/// <c>null</c> once the link has ended and no further messages will arrive.
/// </remarks>
public interface IReceivingTransportHandlers : ITransportHandlers
{
    /// <summary>Waits for the next message on a subscribed topic.</summary>
    /// <returns>The message, or <c>null</c> once the link has ended.</returns>
    Task<TransportMessage?> ReceiveAsync();
}
