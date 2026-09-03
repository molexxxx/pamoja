namespace Pamoja.Mqtt;

/// <summary>A message received from a subscribed topic.</summary>
public sealed class MqttMessage
{
    /// <summary>Creates a message from a topic and its payload.</summary>
    /// <param name="topic">The topic the message was published to.</param>
    /// <param name="payload">The raw payload bytes.</param>
    public MqttMessage(string topic, ReadOnlyMemory<byte> payload)
    {
        Topic = topic;
        Payload = payload;
    }

    /// <summary>The topic the message was published to.</summary>
    public string Topic { get; }

    /// <summary>The raw payload bytes.</summary>
    public ReadOnlyMemory<byte> Payload { get; }
}
