namespace Pamoja.Core;

/// <summary>
/// A link a program sends messages over and receives them from: the loopback, MQTT, CoAP,
/// a transport ladder, or a transport of the program's own.
/// </summary>
/// <remarks>
/// A profile's node publishes each reading through <see cref="SendAsync"/>, and a rule
/// engine listens with <see cref="SubscribeAsync"/> and <see cref="ReceiveAsync"/>, so
/// either runs over any link. Connect the link before handing it over.
/// </remarks>
public interface ILink
{
    /// <summary>Publishes one message to a topic.</summary>
    /// <param name="topic">The destination topic.</param>
    /// <param name="payload">The bytes to send.</param>
    /// <returns>A task that completes once the link has taken the message.</returns>
    Task SendAsync(string topic, ReadOnlyMemory<byte> payload);

    /// <summary>Subscribes to a topic.</summary>
    /// <param name="topic">The topic, or a filter in the syntax the link understands.</param>
    /// <returns>A task that completes once the subscription is placed.</returns>
    Task SubscribeAsync(string topic);

    /// <summary>Waits a limited time for the next message on a subscribed topic.</summary>
    /// <param name="timeout">How long to wait.</param>
    /// <returns>The message, or <c>null</c> once the link has ended.</returns>
    /// <exception cref="TimeoutException">No message arrived in time.</exception>
    Task<TransportMessage?> ReceiveAsync(TimeSpan timeout);
}
