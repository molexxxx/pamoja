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
        Console.WriteLine($"booting   {fleet.OnBoot().Action}");
        fleet.Confirm();
        Console.WriteLine($"confirmed slot {slot} is now {fleet.Record(slot).State}");

        // The same release signed by a key this device is not anchored to gets nowhere.
        byte[] impostorSeed = new byte[32];
        Array.Fill(impostorSeed, (byte)90);
        using var impostor = new DeviceIdentity(impostorSeed);
        try
        {
            fleet.Stage(Update.SignManifest(manifest with { Sequence = 3 }, impostor), image);
            Console.WriteLine("a forged release was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"forged    refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(manifest.Digest.SequenceEqual(Update.ImageDigest(image)), "the digest is the hash");
        Expect(opened.Digest.SequenceEqual(manifest.Digest), "the envelope carries it");
        Expect(slot == 1, "the release lands in the spare slot");
        Expect(fleet.Record(1).State == SlotState.Confirmed, "and the slot holds it from now on");
    }
}
