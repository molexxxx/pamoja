namespace Pamoja.Native.Interop;

/// <summary>
/// MQTT delivery guarantee at the C ABI, mirroring <c>PamojaQos</c> in
/// <c>pamoja.h</c>. The values match the facade <c>Qos</c> in Pamoja.Mqtt.
/// </summary>
public enum PamojaQos
{
    /// <summary>Fire and forget; the broker does not acknowledge delivery.</summary>
    AtMostOnce = 0,

    /// <summary>Delivered at least once and acknowledged.</summary>
    AtLeastOnce = 1,

    /// <summary>Delivered exactly once via a four-step handshake.</summary>
    ExactlyOnce = 2,
}
