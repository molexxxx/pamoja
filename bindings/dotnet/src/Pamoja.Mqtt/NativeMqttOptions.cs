using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

namespace Pamoja.Mqtt;

/// <summary>
/// The native form of <see cref="MqttClientOptions"/>, holding the memory its pointers
/// reach until it is disposed.
/// </summary>
internal sealed class NativeMqttOptions : IDisposable
{
    private readonly List<IntPtr> _held = new();

    /// <summary>The settings as the C ABI reads them.</summary>
    public PamojaMqttConfig Config;

    /// <summary>Marshals the options into native memory.</summary>
    /// <param name="options">The broker settings.</param>
    /// <exception cref="ArgumentException">A password comes without a username, or a client certificate without its key.</exception>
    /// <exception cref="ArgumentOutOfRangeException">A QoS is not one of the <see cref="Qos"/> values.</exception>
    public NativeMqttOptions(MqttClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        if (options.Password is not null && options.Username is null)
        {
            throw new ArgumentException("a password needs a username: MQTT sends no password alone", nameof(options));
        }

        if (options.Tls is { } tls && (tls.CertificatePem is null) != (tls.KeyPem is null))
        {
            throw new ArgumentException("a client certificate and its key come together", nameof(options));
        }

        try
        {
            Config = new PamojaMqttConfig
            {
                ClientId = Text(options.ClientId),
                Host = Text(options.Host),
                Port = options.Port,
                KeepAliveSecs = options.KeepAliveSecs ?? 0,
                Capacity = options.Capacity ?? 0,
                Qos = (PamojaQos)NamedValue.Require(options.Qos ?? Qos.AtLeastOnce, nameof(options.Qos)),
                MaxPacketSize = options.MaxPacketSize ?? 0,
                Username = options.Username is null ? IntPtr.Zero : Text(options.Username),
                Password = options.Password is null ? IntPtr.Zero : Text(options.Password),
                Will = options.Will is null ? IntPtr.Zero : Will(options.Will),
                Tls = options.Tls is null ? IntPtr.Zero : Tls(options.Tls),
            };
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    /// <summary>Releases the native memory.</summary>
    public void Dispose()
    {
        foreach (IntPtr held in _held)
        {
            Marshal.FreeCoTaskMem(held);
        }

        _held.Clear();
    }

    private IntPtr Will(MqttWill will)
    {
        ArgumentNullException.ThrowIfNull(will.Topic);
        (IntPtr payload, nuint length) = Bytes(will.Payload.Span);
        return Struct(new PamojaMqttWill
        {
            Topic = Text(will.Topic),
            Payload = payload,
            PayloadLen = length,
            Qos = (PamojaQos)NamedValue.Require(will.Qos, nameof(will.Qos)),
            Retain = will.Retain ? (byte)1 : (byte)0,
        });
    }

    private IntPtr Tls(MqttTls tls)
    {
        (IntPtr ca, nuint caLength) = Pem(tls.CaPem);
        (IntPtr certificate, nuint certificateLength) = Pem(tls.CertificatePem);
        (IntPtr key, nuint keyLength) = Pem(tls.KeyPem);
        return Struct(new PamojaMqttTls
        {
            CaPem = ca,
            CaPemLen = caLength,
            CertificatePem = certificate,
            CertificatePemLen = certificateLength,
            KeyPem = key,
            KeyPemLen = keyLength,
        });
    }

    private (IntPtr, nuint) Pem(string? pem) =>
        pem is null ? (IntPtr.Zero, 0) : Bytes(System.Text.Encoding.UTF8.GetBytes(pem));

    private IntPtr Text(string text) => Hold(Marshal.StringToCoTaskMemUTF8(text));

    private (IntPtr, nuint) Bytes(ReadOnlySpan<byte> bytes)
    {
        if (bytes.IsEmpty)
        {
            return (IntPtr.Zero, 0);
        }

        IntPtr copy = Hold(Marshal.AllocCoTaskMem(bytes.Length));
        Marshal.Copy(bytes.ToArray(), 0, copy, bytes.Length);
        return (copy, (nuint)bytes.Length);
    }

    private IntPtr Struct<T>(T value)
        where T : struct
    {
        IntPtr memory = Hold(Marshal.AllocCoTaskMem(Marshal.SizeOf<T>()));
        Marshal.StructureToPtr(value, memory, false);
        return memory;
    }

    private IntPtr Hold(IntPtr pointer)
    {
        _held.Add(pointer);
        return pointer;
    }
}
