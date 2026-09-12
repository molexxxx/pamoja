# Your own link

The links pamoja ships are the common ones: MQTT, CoAP, Zenoh, the in-process
broker. The link a node actually has may be none of them. A cellular modem with the
vendor's own SDK, a proprietary radio with its own framing, a serial line to a
gateway, a cloud client that only speaks the cloud's protocol. None of that should
keep the node off a ladder or out of a profile.

The transport contract is small on purpose: connect, send, subscribe, and, for a
link that delivers, receive. In Rust a link implements `Transport` and `Receive`
for its own type. In the three bindings it is an object with those methods handed
to `Transport.fromHandlers`, `Transport.from_handlers`, or `Transport.FromHandlers`,
and each method may be plain or asynchronous. Either way the result is a transport
like any other: it goes on a ladder as a rung, sits under a fault injector, or is
driven directly, and nothing downstream can tell it from a shipped one.

A link without a receive is an uplink. A LoRa radio that only reports, a satellite
messenger, a one-way beacon: it still sends in its turn on a ladder, and the ladder
never subscribes or listens on it.

## What the example does

It writes a link over two queues, standing in for a vendor SDK, and puts it on a
ladder as the only rung. The link records what it is asked to send and what it is
told to listen for, and delivers whatever the vendor side puts in its inbox.

It proves:

- The link is written against the contract, not against a pamoja type: nothing in
  it names a broker or a protocol.
- A reading sent through the ladder reaches the link with its topic and bytes.
- A subscription placed on the ladder reaches the link.
- A message the link delivers comes back through the ladder's own receive, so a
  node written against the ladder gets commands from a link pamoja has never
  heard of.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example link" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example link</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- link" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- link</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/link.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/link.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- link" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- link</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/guides/link.rs#parts -->
From [`examples/guides/link.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/link.rs):

```rust
/// The vendor's side of a link: what it was given to send, what it was told to
/// listen for, and what it has to deliver.
#[derive(Default)]
struct Vendor {
    sent: Vec<Message>,
    filters: Vec<String>,
    inbox: VecDeque<Message>,
}

/// A link over the vendor's queues, standing in for a radio or cloud SDK. Nothing
/// about it names a broker: it needs only the operations the contract asks for.
struct QueueLink {
    vendor: Arc<Mutex<Vendor>>,
    delivered: Arc<Notify>,
    connected: bool,
}

impl Transport for QueueLink {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut vendor = self.vendor.lock().expect("the vendor");
        vendor.sent.push(Message::new(topic, payload));
        Ok(())
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let mut vendor = self.vendor.lock().expect("the vendor");
        vendor.filters.push(topic.to_owned());
        Ok(())
    }
}

impl Receive for QueueLink {
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        loop {
            if let Some(message) = self.vendor.lock().expect("the vendor").inbox.pop_front() {
                return Ok(Some(message));
            }
            self.delivered.notified().await;
        }
    }
}
```
<!-- end -->

<!-- snippet: examples/guides/link.rs#example -->
From [`examples/guides/link.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/link.rs):

```rust
use pamoja_ladder::TransportLadder;
use pamoja_sync::MemoryStore;

// The vendor side is shared with the link so the example can watch it, the way a
// real SDK exposes its own handles.
let vendor = Arc::new(Mutex::new(Vendor::default()));
let delivered = Arc::new(Notify::new());
let link = QueueLink {
    vendor: Arc::clone(&vendor),
    delivered: Arc::clone(&delivered),
    connected: false,
};

// The link is a rung like any shipped transport, and the ladder is the link a
// node is written against.
let mut ladder = TransportLadder::new(MemoryStore::new()).rung(link);
ladder.connect().await?;

// A reading out through the ladder lands in the link, topic and bytes intact.
ladder
    .send_text("sensors/1", "21.5")
    .await
    ?;
let carried = vendor.lock().expect("the vendor").sent[0].clone();
let reading = carried.text().expect("text");
println!("link carried: {} {reading}", carried.topic);

// A subscription placed on the ladder reaches the link.
ladder.subscribe("commands/#").await?;
let filter = vendor.lock().expect("the vendor").filters[0].clone();
println!("link subscribed to: {filter}");

// What the link delivers comes back through the ladder.
vendor
    .lock()
    .expect("the vendor")
    .inbox
    .push_back(Message::new("commands/1", b"open"));
delivered.notify_one();
let command = ladder.recv().await?.expect("a command");
let order = command.text().expect("text");
println!("command over the ladder: {} {order}", command.topic);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/link.ts#example -->
From [`bindings/node/guides/link.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/link.ts):

```typescript
import { Transport, type TransportHandlers, type TransportMessage } from '@pamoja/core'
import { Ladder } from '@pamoja/ladder'
import { Store } from '@pamoja/sync'

// A link over two queues, standing in for a radio or cloud SDK. Nothing about it
// names a broker: it needs only the operations the contract asks for, and `recv` is
// what makes it a link that delivers rather than an uplink.
class QueueLink implements TransportHandlers {
  sent: { topic: string; text: string }[] = []
  filters: string[] = []
  private inbox: TransportMessage[] = []
  private waiting: ((message: TransportMessage) => void)[] = []

  async connect(): Promise<void> {}

  async send(topic: string, payload: Buffer): Promise<void> {
    this.sent.push({ topic, text: payload.toString() })
  }

  async subscribe(topic: string): Promise<void> {
    this.filters.push(topic)
  }

  recv(): Promise<TransportMessage> {
    const next = this.inbox.shift()
    return next ? Promise.resolve(next) : new Promise((resolve) => this.waiting.push(resolve))
  }

  // The vendor side: a message arriving from the radio, which the link hands on in
  // the shape the contract asks for.
  deliver(topic: string, text: string): void {
    const message: TransportMessage = { topic, payload: Buffer.from(text) }
    const waiter = this.waiting.shift()
    if (waiter) waiter(message)
    else this.inbox.push(message)
  }
}

async function main() {
  // The link is a rung like any shipped transport, and the ladder is the link a node
  // is written against.
  const link = new QueueLink()
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.fromHandlers(link))
  await ladder.connect()

  // A reading out through the ladder lands in the link, topic and bytes intact.
  await ladder.send('sensors/1', '21.5')
  const carried = link.sent[0]
  console.log(`link carried: ${carried.topic} ${carried.text}`)

  // A subscription placed on the ladder reaches the link.
  await ladder.subscribe('commands/#')
  const filter = link.filters[0]
  console.log(`link subscribed to: ${filter}`)

  // What the link delivers comes back through the ladder.
  link.deliver('commands/1', 'open')
  const command = (await ladder.recv())!
  console.log(`command over the ladder: ${command.topic} ${command.text!}`)

  return { carried, filter, command }
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/link.py#example -->
From [`bindings/python/guides/link.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/link.py):

```python
import asyncio

from pamoja.core import Message, Transport
from pamoja.ladder import Ladder
from pamoja.sync import Store


class QueueLink:
    """A link over two queues, standing in for a radio or cloud SDK.

    Nothing about it names a broker: it needs only the operations the contract asks
    for, and ``recv`` is what makes it a link that delivers rather than an uplink.
    """

    def __init__(self) -> None:
        self.sent: list[Message] = []
        self.filters: list[str] = []
        self.inbox: asyncio.Queue[Message] = asyncio.Queue()

    async def connect(self) -> None:
        pass

    async def send(self, topic: str, payload: bytes) -> None:
        self.sent.append(Message(topic, payload))

    async def subscribe(self, topic: str) -> None:
        self.filters.append(topic)

    async def recv(self) -> Message:
        return await self.inbox.get()

    def deliver(self, message: Message) -> None:
        """The vendor side: a message arriving from the radio."""
        self.inbox.put_nowait(message)


async def main() -> None:
    # The link is a rung like any shipped transport, and the ladder is the link a
    # node is written against.
    link = QueueLink()
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.from_handlers(link))
    await ladder.connect()

    # A reading out through the ladder lands in the link, topic and bytes intact.
    await ladder.send("sensors/1", "21.5")
    carried = link.sent[0]
    print(f"link carried: {carried.topic} {carried.text}")

    # A subscription placed on the ladder reaches the link.
    await ladder.subscribe("commands/#")
    filter = link.filters[0]
    print(f"link subscribed to: {filter}")

    # What the link delivers comes back through the ladder.
    link.deliver(Message("commands/1", "open"))
    command = await ladder.recv()
    print(f"command over the ladder: {command.topic} {command.text}")

    return carried, filter, command


carried, filter, command = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs#parts -->
From [`bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs):

```csharp
/// <summary>
/// A link over two queues, standing in for a radio or cloud SDK. Nothing about it
/// names a broker: it needs only the operations the contract asks for, and
/// <see cref="ReceiveAsync"/> is what makes it a link that delivers rather than an
/// uplink.
/// </summary>
private sealed class QueueLink : IReceivingTransportHandlers
{
    private readonly Channel<TransportMessage?> _inbox =
        Channel.CreateUnbounded<TransportMessage?>();

    public List<TransportMessage> Sent { get; } = new();

    public List<string> Filters { get; } = new();

    public Task ConnectAsync() => Task.CompletedTask;

    public Task SendAsync(string topic, ReadOnlyMemory<byte> payload)
    {
        Sent.Add(new TransportMessage(topic, payload.ToArray()));
        return Task.CompletedTask;
    }

    public Task SubscribeAsync(string topic)
    {
        Filters.Add(topic);
        return Task.CompletedTask;
    }

    public async Task<TransportMessage?> ReceiveAsync() => await _inbox.Reader.ReadAsync();

    /// <summary>
    /// The vendor side: a message arriving from the radio, which the link hands on
    /// in the shape the contract asks for.
    /// </summary>
    /// <param name="topic">The topic it arrived on.</param>
    /// <param name="text">What it carried.</param>
    public void Deliver(string topic, string text) =>
        _inbox.Writer.TryWrite(new TransportMessage(topic, Encoding.UTF8.GetBytes(text)));
}
```
<!-- end -->

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/LinkGuide.cs):

```csharp
// The link is a rung like any shipped transport, and the ladder is the link a
// node is written against.
var link = new QueueLink();
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.FromHandlers(link));
await ladder.ConnectAsync();

// A reading out through the ladder lands in the link, topic and bytes intact.
await ladder.SendAsync("sensors/1", "21.5");
TransportMessage carried = link.Sent[0];
Console.WriteLine(
    $"link carried: {carried.Topic} {carried.Text}");

// A subscription placed on the ladder reaches the link.
await ladder.SubscribeAsync("commands/#");
string filter = link.Filters[0];
Console.WriteLine($"link subscribed to: {filter}");

// What the link delivers comes back through the ladder.
link.Deliver("commands/1", "open");
TransportMessage command = (await ladder.ReceiveAsync())!;
Console.WriteLine(
    $"command over the ladder: {command.Topic} {command.Text}");
```
<!-- end -->

## Reference

<!-- table: reference link -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-link)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-link)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-link)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-link)
<!-- end -->
