using Pamoja.Audit;
using Pamoja.Security;

using static Guides.Guide;

namespace Guides;

/// <summary>The audit log guide example; see docs/guides/audit.md.</summary>
public static class AuditGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The controller signs its own log with a provisioned seed. This one is RFC 8032
        // test vector 1, so the key the records are checked against is a published
        // constant rather than a value checked against itself.
        using var keeper = new DeviceIdentity(Convert.FromHexString(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"));
        Expect(
            Convert.ToHexString(keeper.PublicKey).ToLowerInvariant()
                == "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "the key a chain is checked against is the one the vector publishes");

        using var log = new AuditLog(keeper);
        using AuditEntry lit = log.Append("burner=on"u8);
        using AuditEntry stopped = log.Append("burner=off"u8);

        // A record's digest is SHA-256 over its little-endian index, the digest of the
        // record before it, and its payload, so the first record hashes forty zero bytes
        // and then what it carries.
        Expect(lit.Index == 0, "the first record sits at index zero");
        Expect(
            Convert.ToHexString(lit.Digest).ToLowerInvariant()
                == "e50c6a7a944fab6dd13ffdb760ca190e14ea00c168ba7c948745ba0af146c159",
            "the digest is the one the chain construction fixes");
        Expect(
            stopped.Previous.SequenceEqual(lit.Digest),
            "each record carries the hash of the one before it");
        Expect(Audit.VerifyChain(keeper.PublicKey, [lit, stopped]), "an untouched chain verifies");

        // Editing a stored record changes the digest its signature covers.
        byte[] edited = stopped.ToBytes();
        edited[^1] ^= 0xFF;
        using AuditEntry tampered = AuditEntry.FromBytes(edited);
        Expect(
            !Audit.VerifyChain(keeper.PublicKey, [lit, tampered]),
            "and an edited record does not");

        // Dropping the record before it leaves the survivor chained to a link that is no
        // longer there, so a shortened log is caught as readily as an edited one.
        Expect(
            !Audit.VerifyChain(keeper.PublicKey, [stopped]),
            "a log with its first record removed does not verify either");
        // ANCHOR_END: example
    }
}
