# ChirpStack LoRaWAN interop

Real-network interop for `pamoja-gateway`. The crate's own tests prove the packet forwarder
protocol and the network side against published vectors and against our own types; the test here
proves them against an actual LoRaWAN network server, so interop is real rather than
self-referential.

ChirpStack runs as a stack rather than a single image, so unlike the
[MAVLink SITL interop](../sitl/README.md) next door the containers come up under `docker compose`
while the test runs on the host. The configuration is ChirpStack's own, vendored here and trimmed
to the one region this test operates in.

- `docker-compose.yml` pins the network server, the gateway bridge that speaks the Semtech
  packet forwarder protocol on 1700/udp, the gRPC to REST proxy the provisioning uses, and the
  Postgres, Redis and Mosquitto the server needs.
- `run-chirpstack.sh` brings the stack up, mints an API token with ChirpStack's own
  `create-api-key` command, registers a gateway and an OTAA device, runs the ignored interop test
  ([`examples/tests/chirpstack.rs`](../examples/tests/chirpstack.rs))
  with the endpoints set, then prints the logs and tears the stack down.

## Running

From the repo root, with Docker Desktop running:

```
cargo xtask chirpstack
```

CI runs the same script in the `chirpstack` job. Nothing here is a deployment: the stack lives
for the length of one run, its API secret signs tokens for that run alone, the keys are test
values, and no volumes are kept.

## What the test asserts

The gateway forwards a real OTAA join request and, once the device has joined, an encrypted
uplink, both as `PUSH_DATA` datagrams built by `pamoja_gateway::udp`. ChirpStack publishes what
it made of them on its MQTT integration, and the test asserts against those events: that the join
was accepted for the device we registered, and that the uplink arrived on the port and with the
payload the device sent. The downlink half is asserted from the `PULL_RESP` the server sends back
for the join accept.

It does not assert timing or radio behavior, since there is no radio: the point is that a network
server built by someone else reads what this crate writes, and that this crate reads what it
writes back.
