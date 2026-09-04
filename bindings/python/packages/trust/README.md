# pamoja-trust

Proving what a node did, saying it in confidence, fixing it in the field, and deciding how often it can afford to do any of that.

One install for the 5 capabilities of this domain. Each is also its own
distribution, and `pamoja` is the whole framework in one.

```sh
pip install pamoja-trust
```

```python
from pamoja.trust import audit
```

| Capability | Module | What it covers |
| --- | --- | --- |
| [Audit log](https://pamoja.molex.cloud/docs/guides/audit.html) | `pamoja.audit` | A tamper-evident, hash-chained log; altering, reordering, or dropping a record breaks verification |
| [Secured session](https://pamoja.molex.cloud/docs/guides/session.html) | `pamoja.session` | X25519 key agreement, HKDF, and ChaCha20-Poly1305 with an anti-replay window, with no TLS stack |
| [Signed updates](https://pamoja.molex.cloud/docs/guides/update.html) | `pamoja.update` | Signed firmware manifests, streaming image verification, and A/B slots that fall back on their own |
| [Power](https://pamoja.molex.cloud/docs/guides/power.html) | `pamoja.power` | Duty cycling and an energy-aware governor that stretches work as the battery drains |
| [Telemetry](https://pamoja.molex.cloud/docs/guides/telemetry.html) | `pamoja.telemetry` | Observability that ships only what is worth the bytes as link cost rises, while counting everything |

The guides, with a worked Python example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
