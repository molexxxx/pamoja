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

    /// <summary>The payload as text: words, or a number written out.</summary>
    public string Text => System.Text.Encoding.UTF8.GetString(Payload.Span);

    /// <summary>The payload as a number written out as text, such as <c>21.5</c>, or <c>null</c> when it is not one.</summary>
    public double? Number =>
        double.TryParse(
            Text.Trim(),
            System.Globalization.NumberStyles.Float,
            System.Globalization.CultureInfo.InvariantCulture,
            out double number)
            ? number
            : null;
}
