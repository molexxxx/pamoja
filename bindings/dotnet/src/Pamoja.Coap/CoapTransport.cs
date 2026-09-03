using System.Runtime.InteropServices;

using Pamoja.Native.Interop;

using Pamoja.Core;

namespace Pamoja.Coap;

/// <summary>Builds the <see cref="Transport"/> a ladder rung uses to reach a peer over CoAP.</summary>
public static class CoapTransport
{
    /// <summary>Creates a transport that reaches a peer over CoAP.</summary>
    /// <param name="options">The endpoint settings.</param>
    /// <returns>The transport, ready to add as a rung.</returns>
    public static Transport Open(CoapClientOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        return options.WithNativeConfig(
            (ref PamojaCoapConfig config) => new Transport(
                NativeMethods.pamoja_transport_coap(ref config),
                "CoAP transport"));
    }

}
