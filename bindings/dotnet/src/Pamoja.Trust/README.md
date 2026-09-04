# Pamoja.Trust

Proving what a node did, saying it in confidence, fixing it in the field, and deciding how often it can afford to do any of that.

One reference for the 5 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.Trust
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html) | `Pamoja.Audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| [Secured session](https://pamoja.molex.cloud/docs/guides/session.html) | `Pamoja.Session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html) | `Pamoja.Update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| [Power](https://pamoja.molex.cloud/docs/guides/power.html) | `Pamoja.Power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | `Pamoja.Telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
