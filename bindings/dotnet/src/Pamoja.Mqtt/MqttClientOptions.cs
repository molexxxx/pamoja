namespace Pamoja.Mqtt;

/// <summary>Connection settings for an <see cref="MqttClient"/>.</summary>
/// <remarks>
/// Constructed with object-initializer syntax; only <see cref="ClientId"/>,
/// <see cref="Host"/>, and <see cref="Port"/> are required, and the optional fields
/// fall back to the core defaults when left null.
/// </remarks>
public sealed class MqttClientOptions
{
    /// <summary>The MQTT client identifier presented to the broker.</summary>
    public required string ClientId { get; init; }

    /// <summary>The broker hostname or IP address.</summary>
    public required string Host { get; init; }

    /// <summary>The broker TCP port, conventionally 1883 for plaintext MQTT.</summary>
    public required ushort Port { get; init; }

    /// <summary>Keep-alive interval in seconds. Defaults to 30 when null.</summary>
    public uint? KeepAliveSecs { get; init; }

    /// <summary>Bound on outstanding client requests. Defaults to 64 when null.</summary>
    public uint? Capacity { get; init; }

    /// <summary>Default quality of service. Defaults to <see cref="Qos.AtLeastOnce"/> when null.</summary>
    public Qos? Qos { get; init; }
}
