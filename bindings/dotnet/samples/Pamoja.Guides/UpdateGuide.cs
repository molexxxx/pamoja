using System.Text;

using Pamoja;
using Pamoja.Security;
using Pamoja.Update;

using static Guides.Guide;

namespace Guides;

/// <summary>The signed updates guide example; see docs/guides/update.md.</summary>
public static class UpdateGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
        byte[] deviceClass = Enumerable.Repeat((byte)0x0B, 16).ToArray();
        using var publisher = new DeviceIdentity(Enumerable.Repeat((byte)0x31, 32).ToArray());

        // The image stands in for firmware. It is the 56-byte message FIPS 180-4 hashes in
        // its second worked example, so the digest the manifest commits to is a published
        // constant rather than a value checked against itself.
        byte[] image = Encoding.ASCII.GetBytes(
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        byte[] digest = Convert.FromHexString(
            "248D6A61D20638B8E5C026930C3E6039A33CE45964FF2167F6ECEDD419DB06C1");

        // A release says who it is for, which slot it belongs in, and what it hashes to. The
        // publisher signs that statement; nothing else about the image is taken on trust.
        var manifest = new Manifest(
            Sequence: 2,
            VendorId: vendor,
            ClassId: deviceClass,
            Storage: 1,
            Digest: digest,
            Size: (uint)image.Length);
        byte[] envelope = Update.SignManifest(manifest, publisher);
        Expect(
            Update.VerifyEnvelope(envelope, publisher.PublicKey).Digest.SequenceEqual(digest),
            "the release verifies against the key that signed it");

        // The device left the factory running sequence 1 from slot 0, so the release goes to
        // the spare slot and the image it runs today stays where it is.
        using var fleet = new Updater(vendor, deviceClass, publisher.PublicKey, 2, 4096);
        fleet.Provision(0, 1);
        Expect(fleet.Begin(envelope) == 1, "the release names the spare slot");
        for (int at = 0; at < image.Length; at += 16)
        {
            fleet.Write(image.AsSpan(at, Math.Min(16, image.Length - at)));
        }
        Expect(fleet.CurrentProgress().Written == image.Length, "every byte arrived");
        Expect(fleet.Finish() == 1, "and the image matched what was promised");

        // The first boot into a new image is a trial. It reverts to slot 0 on the next boot
        // unless it confirms itself.
        Expect(fleet.OnBoot().Action == BootAction.Trying, "a new image is on trial");
        Expect(fleet.Confirm() == 1, "and confirms once it has run");
        Expect(fleet.Record(1).State == SlotState.Confirmed, "so the slot holds it from now on");

        // The same release, signed by a key this device is not anchored to, gets nowhere.
        using var impostor = new DeviceIdentity(Enumerable.Repeat((byte)0x32, 32).ToArray());
        bool refused = false;
        try
        {
            fleet.Stage(Update.SignManifest(manifest with { Sequence = 3 }, impostor), image);
        }
        catch (PamojaException)
        {
            refused = true;
        }
        Expect(refused, "a release signed by an untrusted key is refused");
        // ANCHOR_END: example
    }
}
