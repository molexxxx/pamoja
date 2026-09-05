# Pamoja.Coap

A CoAP client over UDP with confirmable delivery and observe. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/coap.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Coap
```

```csharp
using Pamoja.Coap;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CoapGuide.cs):

```csharp
// CoAP runs over UDP and opens no session, so connecting only binds a local
// socket. Nothing is listening on the far side here, and for a non-confirmable
// send nothing needs to be.
using var reporter = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.NonConfirmable,
});
await reporter.ConnectAsync();
Console.WriteLine($"reporter  connected: {await reporter.IsConnectedAsync()}");

// Non-confirmable delivery is at most once: the datagram leaves unacknowledged,
// which is what a battery-powered node sends when a missed reading costs nothing.
await reporter.SendAsync("sensors/1/temperature", "21.5"u8.ToArray());
Console.WriteLine("reporter  sent 21.5 and did not wait for an answer");

// A command is different: it has to arrive. Confirmable delivery retransmits until
// an acknowledgement comes back. RFC 7252 fixes the defaults at a two-second wait
// and four retransmissions; both are cut short here so the guide does not sit
// waiting.
using var commander = new CoapClient(new CoapClientOptions
{
    Host = "127.0.0.1",
    Port = 5683,
    Reliability = Reliability.Confirmable,
    AckTimeoutMs = 20,
    MaxRetransmits = 1,
});
await commander.ConnectAsync();
try
{
    await commander.SendAsync("actuators/valve", "open"u8.ToArray());
    Console.WriteLine("commander the valve acknowledged the command");
}
catch (PamojaException error)
{
    Console.WriteLine($"commander gave up unacknowledged: {error.Message}");
}

await reporter.DisconnectAsync();
Console.WriteLine($"reporter  disconnected: {!await reporter.IsConnectedAsync()}");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-coap`](https://crates.io/crates/pamoja-coap) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_coap/index.html), [docs.rs](https://docs.rs/pamoja-coap) |
| TypeScript | [`@pamoja/coap`](https://www.npmjs.com/package/@pamoja/coap) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_coap.html) |
| Python | [`pamoja-coap`](https://pypi.org/project/pamoja-coap/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/coap.html) |
| C# | [`Pamoja.Coap`](https://www.nuget.org/packages/Pamoja.Coap) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html) |

## Documentation

- [`Pamoja.Coap` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Coap.html), every type in this namespace.
- [The CoAP guide](https://pamoja.molex.cloud/docs/guides/coap.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
