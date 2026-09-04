using System.Runtime.InteropServices;

using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Mqtt;

/// <summary>
/// A <see cref="SafeHandle"/> over a native MQTT client pointer, so the handle is
/// always released exactly once even across finalization races.
/// </summary>
internal sealed class MqttClientHandle : SafeHandle
{
    /// <summary>Wraps a non-null native client pointer.</summary>
    /// <param name="handle">A pointer returned by <see cref="NativeMethods.pamoja_mqtt_client_new"/>.</param>
    public MqttClientHandle(IntPtr handle)
        : base(IntPtr.Zero, ownsHandle: true)
    {
        SetHandle(handle);
    }

    /// <inheritdoc/>
    public override bool IsInvalid => handle == IntPtr.Zero;

    /// <inheritdoc/>
    protected override bool ReleaseHandle()
    {
        NativeMethods.pamoja_mqtt_client_free(handle);
        return true;
    }
}
