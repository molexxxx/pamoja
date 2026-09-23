using System.Text;

using Pamoja;
using Pamoja.Security;
using Pamoja.Update;

using static Guides.Guide;

namespace Guides;

/// <summary>The signed update guide example; see docs/guides/update.md.</summary>
public static class UpdateGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The publisher's key signs releases; devices in the field are anchored to its
        // public half and will take firmware from nobody else.
        byte[] seed = new byte[32];
        Array.Fill(seed, (byte)7);
        using var publisher = new DeviceIdentity(seed);
        byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
        byte[] deviceClass = Enumerable.Repeat((byte)0x0B, 16).ToArray();

        // The release. A manifest says who the image is for, which slot it belongs in, how
        // big it is and what it hashes to; nothing about the image is taken on trust.
        byte[] image = Encoding.ASCII.GetBytes("firmware for a flow meter, version two");
        var manifest = new Manifest(
            Sequence: 2,
            VendorId: vendor,
            ClassId: deviceClass,
            Storage: 1,
            Digest: Update.ImageDigest(image),
            Size: (uint)image.Length);
        byte[] envelope = Update.SignManifest(manifest, publisher);
        Console.WriteLine(
            $"published sequence {manifest.Sequence} in a {envelope.Length}-byte envelope");

        // On the device. It checks the envelope against the key it was anchored to before
        // it accepts a single byte of the image.
        Manifest opened = Update.VerifyEnvelope(envelope, publisher.PublicKey);
        Console.WriteLine($"accepted  a release for slot {opened.Storage}");

        // It left the factory running sequence 1 from slot 0, so the release goes to the
        // spare slot and the image it is running stays where it is.
        using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
        fleet.Provision(0, 1);
        fleet.Begin(envelope);
        for (int at = 0; at < image.Length; at += 16)
        {
            fleet.Write(image.AsSpan(at, Math.Min(16, image.Length - at)));
        }

        Console.WriteLine($"staged    {fleet.CurrentProgress().Written} of {image.Length} bytes");
        byte slot = fleet.Finish();
        Console.WriteLine($"written   to slot {slot}, leaving the running image alone");

        // The first boot into a new image is a trial. It reverts on the next boot unless
        // the device confirms it came up, which is what makes a bad release survivable.
        static string Said(BootDecision decision) => decision.Action switch
        {
            BootAction.Trying => $"slot {decision.Slot} on trial",
            BootAction.Confirmed => $"slot {decision.Slot}, already confirmed",
            _ => $"slot {decision.Slot} never confirmed, so the device runs slot {decision.Fallback} again",
        };
        BootDecision decision = fleet.OnBoot();
        Console.WriteLine($"booting   {Said(decision)}");
        fleet.Confirm();
        Console.WriteLine($"confirmed slot {slot} is now {fleet.Record(slot).State}");

        // The same release offered again would take the device nowhere new, so it is
        // refused as a rollback, and so would any older one.
        try
        {
            fleet.Stage(envelope, image);
            Console.WriteLine("an old release was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"old       refused: {error.Message}");
        }

        // The next release goes to the slot the device is not running, slot 0 now. An
        // image damaged on the way still arrives in full, but it does not hash to what was
        // signed.
        byte[] upgrade = Encoding.ASCII.GetBytes("firmware for a flow meter, version three");
        Manifest third = manifest with
        {
            Sequence = 3,
            Storage = 0,
            Digest = Update.ImageDigest(upgrade),
            Size = (uint)upgrade.Length,
        };
        byte[] release = Update.SignManifest(third, publisher);
        byte[] damaged = (byte[])upgrade.Clone();
        damaged[0] ^= 0xFF;
        try
        {
            fleet.Stage(release, damaged);
            Console.WriteLine("a damaged image was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"corrupt   refused: {error.Message}");
        }

        // The same release signed by a key this device is not anchored to gets nowhere.
        byte[] impostorSeed = new byte[32];
        Array.Fill(impostorSeed, (byte)90);
        using var impostor = new DeviceIdentity(impostorSeed);
        try
        {
            fleet.Stage(Update.SignManifest(third, impostor), upgrade);
            Console.WriteLine("a forged release was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"forged    refused: {error.Message}");
        }

        // The genuine release stages and boots on trial, but never confirms: the next boot
        // fails it and goes back to the image that worked.
        fleet.Stage(release, upgrade);
        BootDecision trial = fleet.OnBoot();
        Console.WriteLine($"booting   {Said(trial)}, running sequence {third.Sequence}");
        BootDecision after = fleet.OnBoot();
        Console.WriteLine($"reverted  {Said(after)}");

        // A release that failed cannot be offered again, or a captured image could be
        // replayed; the fix goes out as sequence 4.
        try
        {
            fleet.Stage(release, upgrade);
            Console.WriteLine("a failed release was accepted again, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"again     refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(manifest.Digest.SequenceEqual(Update.ImageDigest(image)), "the digest is the hash");
        Expect(opened.Digest.SequenceEqual(manifest.Digest), "the envelope carries it");
        Expect(slot == 1, "the release lands in the spare slot");
        Expect(decision.Action == BootAction.Trying, "its first boot is a trial");
        Expect(fleet.Record(1).State == SlotState.Confirmed, "and the slot holds it from now on");
        Expect(trial == new BootDecision(BootAction.Trying, 0, 0), "the next release tries slot 0");
        Expect(after == new BootDecision(BootAction.Reverted, 0, 1), "and reverts to slot 1");
        Expect(fleet.Record(0).State == SlotState.Failed, "leaving slot 0 failed");
        Expect(fleet.InstalledSequence == 3, "and its sequence spent");
    }
}
