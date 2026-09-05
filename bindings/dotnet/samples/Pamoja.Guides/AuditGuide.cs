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
        // The controller signs its own log with a provisioned seed and an auditor holds
        // only the public half, so a log can be checked anywhere without the device.
        byte[] seed = new byte[32];
        Array.Fill(seed, (byte)7);
        using var keeper = new DeviceIdentity(seed);
        byte[] auditor = keeper.PublicKey;

        using var log = new AuditLog(keeper);
        using AuditEntry lit = log.Append("burner=on"u8);
        using AuditEntry stopped = log.Append("burner=off"u8);
        Console.WriteLine($"recorded  {lit.Index} then {stopped.Index}");

        // Each record hashes its own index, the digest of the record before it, and what
        // it carries, so the chain fixes the order as well as the contents.
        Console.WriteLine($"chained   {stopped.Previous.SequenceEqual(lit.Digest)}");
        Console.WriteLine($"verified  {Audit.VerifyChain(auditor, [lit, stopped])}");

        // Editing a stored record changes the digest its signature covers.
        byte[] edited = stopped.ToBytes();
        edited[^1] ^= 0xFF;
        using AuditEntry tampered = AuditEntry.FromBytes(edited);
        Console.WriteLine($"edited    caught: {!Audit.VerifyChain(auditor, [lit, tampered])}");

        // Dropping the first record leaves the survivor chained to a link that is no
        // longer there, so a shortened log is caught as readily as an edited one.
        Console.WriteLine($"shortened caught: {!Audit.VerifyChain(auditor, [stopped])}");
        // ANCHOR_END: example

        Expect(Audit.VerifyChain(auditor, [lit, stopped]), "an untouched chain verifies");
        Expect(!Audit.VerifyChain(auditor, [lit, tampered]), "an edited record does not");
        Expect(!Audit.VerifyChain(auditor, [stopped]), "nor does a shortened log");
    }
}
