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

    /// <summary>
    /// The largest packet the connection sends or accepts, in bytes. Defaults to 10,240
    /// when null.
    /// </summary>
    /// <remarks>
    /// A packet is a message's topic and payload plus a few bytes of framing. A publish
    /// that would be larger is refused and the connection stays up, but a larger packet
    /// arriving from the broker ends the connection, so every client that shares a
    /// topic needs a limit that fits it.
    /// </remarks>
    public uint? MaxPacketSize { get; init; }

    /// <summary>The name to sign in to the broker with.</summary>
    public string? Username { get; init; }

    /// <summary>The password to sign in with, which needs a <see cref="Username"/>.</summary>
    /// <remarks>It travels in the clear unless the connection uses <see cref="Tls"/>.</remarks>
    public string? Password { get; init; }

    /// <summary>A message the broker publishes if the connection ends without a goodbye.</summary>
    public MqttWill? Will { get; init; }

    /// <summary>TLS settings; a connection with them is secured, conventionally on port 8883.</summary>
    public MqttTls? Tls { get; init; }
}

/// <summary>
/// A message the broker publishes on the client's behalf if its connection ends without a
/// disconnect: the network dropped, the power failed, or the keep-alive ran out.
/// </summary>
/// <remarks>A client that calls <see cref="MqttClient.DisconnectAsync"/> leaves no will behind.</remarks>
/// <param name="Topic">The topic the broker publishes it to, with no wildcard.</param>
/// <param name="Payload">What it publishes.</param>
public sealed record MqttWill(string Topic, ReadOnlyMemory<byte> Payload)
{
    /// <summary>Creates a will whose payload is text, encoded as UTF-8.</summary>
    /// <param name="topic">The topic the broker publishes it to.</param>
    /// <param name="payload">What it publishes.</param>
    public MqttWill(string topic, string payload)
        : this(topic, System.Text.Encoding.UTF8.GetBytes(payload))
    {
    }

    /// <summary>The quality of service it is published at.</summary>
    public Qos Qos { get; init; } = Qos.AtMostOnce;

    /// <summary>Whether the broker retains it for clients that subscribe later.</summary>
    public bool Retain { get; init; }
}

/// <summary>How a connection is secured with TLS.</summary>
/// <remarks>
/// Without <see cref="CaPem"/> the client trusts the certificate authorities the operating
/// system trusts. A client certificate and its key are given together, for a broker that
/// asks each client to prove who it is.
/// </remarks>
public sealed class MqttTls
{
    /// <summary>The certificate authorities to trust, as PEM.</summary>
    public string? CaPem { get; init; }

    /// <summary>A client certificate to present, as PEM.</summary>
    public string? CertificatePem { get; init; }

    /// <summary>The client certificate's private key, as PEM.</summary>
    public string? KeyPem { get; init; }
}

/// <summary>How one message is published.</summary>
/// <param name="Qos">The quality of service for this message, or null for the client's.</param>
/// <param name="Retain">
/// Whether the broker keeps it for clients that subscribe later. An empty retained message
/// clears the one the broker holds.
/// </param>
public readonly record struct MqttPublishOptions(Qos? Qos = null, bool Retain = false);
