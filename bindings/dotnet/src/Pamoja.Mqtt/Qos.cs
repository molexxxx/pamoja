namespace Pamoja.Mqtt;

/// <summary>MQTT delivery guarantee, mirroring the protocol's quality-of-service levels.</summary>
public enum Qos
{
    /// <summary>Fire and forget; the broker does not acknowledge delivery.</summary>
    AtMostOnce = 0,

    /// <summary>Delivered at least once and acknowledged.</summary>
    AtLeastOnce = 1,

    /// <summary>Delivered exactly once via a four-step handshake.</summary>
    ExactlyOnce = 2,
}
