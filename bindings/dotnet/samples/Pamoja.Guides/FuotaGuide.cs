using System.Text;

using Pamoja;
using Pamoja.Lorawan;
using Pamoja.Security;
using Pamoja.Update;

using static Guides.Guide;

namespace Guides;

/// <summary>The firmware update over the air guide example; see docs/guides/fuota.md.</summary>
public static class FuotaGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The publisher signs releases; the devices in the field are anchored to its public
        // half and will take firmware from nobody else, however it reaches them.
        byte[] seed = new byte[32];
        Array.Fill(seed, (byte)7);
        using var publisher = new DeviceIdentity(seed);
        byte[] vendor = Enumerable.Repeat((byte)0x0A, 16).ToArray();
        byte[] flowMeter = Enumerable.Repeat((byte)0x0B, 16).ToArray();

        byte[] image = Encoding.ASCII.GetBytes(
            "firmware for a flow meter, version two, long enough to need fragmenting");
        var manifest = new Manifest(
            Sequence: 2,
            VendorId: vendor,
            ClassId: flowMeter,
            Storage: 1,
            Digest: Update.ImageDigest(image),
            Size: (uint)image.Length);
        byte[] envelope = Update.SignManifest(manifest, publisher);

        // One block carries the release: the signed manifest and the image behind a header
        // that says where each begins. The transport moves bytes and vouches for none of them.
        byte[] block = Update.FrameBlock(envelope, image);
        Console.WriteLine(
            $"release   {block.Length} bytes: a signed manifest and {image.Length} of image");

        // The group every device in the field belongs to. Its key travels wrapped under a key
        // each device derives from its own root key, so the broadcast key is never in the clear.
        byte[] deviceRootKey = Enumerable.Repeat((byte)0x2B, 16).ToArray();
        byte[] groupKey = Enumerable.Repeat((byte)0x77, 16).ToArray();
        const uint groupAddr = 0x2601_0042;
        byte[] keKey = LorawanPackages.McKeKey(LorawanPackages.McRootKey(deviceRootKey));
        byte[] wrapped = LorawanPackages.WrapMcKey(keKey, groupKey);
        byte[] unwrapped = LorawanPackages.McKey(keKey, wrapped);
        Console.WriteLine(
            $"group     0x{groupAddr:X8} keyed by a wrapped key the device unwraps: " +
            $"{unwrapped.SequenceEqual(groupKey)}");
        byte[] payloadKey = LorawanPackages.McAppSKey(groupKey, groupAddr);

        // The server cuts the block into fragments and sends more than there are, so a device
        // that misses some can still finish. Each coded fragment is the exclusive-or of a
        // pseudo-random half of the originals.
        const byte fragSize = 32;
        LorawanFragSession session = LorawanPackages.FragSession(block.Length, fragSize);
        Console.WriteLine(
            $"session   {session.NbFrag} fragments of {fragSize} bytes, " +
            $"{session.Padding} of padding");

        // The device puts it back together in storage it set aside once: the block itself,
        // and room to solve for a handful of losses.
        using var receiver = new LorawanDefragmenter(session.NbFrag, fragSize, 8);

        // The link drops every fourth fragment. The session keeps going until the block is whole.
        int sent = 0;
        int coded = 0;
        for (ushort n = 1; n <= session.NbFrag * 2; n++)
        {
            if (n % 4 == 0)
            {
                continue;
            }

            sent++;
            if (n > session.NbFrag)
            {
                coded++;
            }

            if (receiver.Fragment(n, LorawanPackages.FragFragment(block, fragSize, n)))
            {
                break;
            }
        }

        Console.WriteLine(
            $"received  {sent} fragments, {coded} of them coded, and the block is whole");

        // What the device built is checked against the code the session setup carried, taken
        // over the block a piece at a time so the image is never held twice.
        byte[] blockKey = LorawanPackages.DataBlockIntKey(deviceRootKey);
        byte[] descriptor = Encoding.ASCII.GetBytes(Update.BlockDescriptor);
        using var expected = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
        expected.Update(block);
        byte[] built = receiver.Block().AsSpan(0, block.Length).ToArray();
        using var taken = new LorawanBlockMic(blockKey, 1, 0, descriptor, (uint)block.Length);
        taken.Update(built);
        bool intact = taken.Finish().SequenceEqual(expected.Finish());
        Console.WriteLine($"checked   the block the device built is the one the server sent: {intact}");

        // Only now does the update itself get a say. The header says where the manifest ends;
        // everything after that is the manifest's decision, exactly as for a wired update.
        (byte[] carriedEnvelope, byte[] carriedImage) = Update.SplitBlock(built);
        using var updater = new Updater(vendor, flowMeter, publisher.PublicKey, 2, 4096);
        updater.Provision(0, 1);
        byte slot = updater.Stage(carriedEnvelope, carriedImage);
        Console.WriteLine($"staged    into slot {slot}, leaving the running image alone");

        // A release broadcast to everyone is still refused by anyone it is not for. This one
        // is signed by another key.
        byte[] impostorSeed = new byte[32];
        Array.Fill(impostorSeed, (byte)90);
        using var impostor = new DeviceIdentity(impostorSeed);
        try
        {
            updater.Stage(Update.SignManifest(manifest, impostor), image);
            Console.WriteLine("a forged release was accepted, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"forged    refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(intact, "the block survived the link");
        Expect(slot == 1, "the release lands in the spare slot");
        Expect(carriedImage.SequenceEqual(image), "and it is the image the publisher signed");
        Expect(payloadKey.Length == 16, "the group has a payload key");
    }
}
