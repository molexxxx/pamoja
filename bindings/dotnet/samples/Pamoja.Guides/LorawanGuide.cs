using System.Globalization;
using System.Text;

using Pamoja;
using Pamoja.Lora;
using Pamoja.Lorawan;

using static Guides.Guide;

namespace Guides;

/// <summary>The LoRaWAN activation guide example; see docs/guides/lorawan.md.</summary>
public static class LorawanGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // The root key is provisioned into the device at the factory and known to the
        // network server. It is the only secret either side starts with; any 16 bytes
        // stand in here.
        byte[] appKey = new byte[16];
        Array.Fill(appKey, (byte)7);

        // The device asks to join with a nonce it has not used before, which is what stops
        // an old accept being replayed at it.
        const ushort DevNonce = 1;
        using var node = new LorawanDevice(new byte[8], new byte[8], appKey);

        // The network grants the join. It draws its own nonce, names the network the
        // device is joining, and assigns the address it will answer to from then on.
        const uint DevAddr = 0x26012E43;
        var offer = new LorawanGrant(appNonce: 2, netId: 19, devAddr: DevAddr);
        byte[] accept = offer.Accept(appKey, DevNonce);
        Console.WriteLine($"granted   address 0x{DevAddr:X8} in a {accept.Length}-byte accept");

        // The device verifies it against the root key. A join accept carries no device
        // identifier, so only that key decides whether it is for this device.
        using LorawanJoinAccept joined = node.AcceptJoin(accept, DevNonce);
        Console.WriteLine($"joined    the device took address 0x{joined.DevAddr:X8}");

        // Neither side transmits a session key. Both derive the same pair from the root
        // key and the two nonces, so the network reads what the device sends without ever
        // having been told how.
        using LorawanSession network = offer.Session(appKey, DevNonce);
        using LorawanSession activated = joined.Session();
        byte[] uplink = activated.EncodeUplink(1, 1, "level=high"u8);
        LorawanRxData received = network.Decode(uplink, 1);
        Console.WriteLine(
            $"uplink    the network read {System.Text.Encoding.UTF8.GetString(received.Payload)}");

        // A single byte changed in the air fails that check, so no one else can admit the
        // device or put words in its mouth.
        byte[] forged = [.. accept];
        forged[1] ^= 0xFF;
        try
        {
            node.AcceptJoin(forged, DevNonce).Dispose();
            Console.WriteLine("a forged accept was taken, which should never happen");
        }
        catch (PamojaException error)
        {
            Console.WriteLine($"forged    accept refused: {error.Message}");
        }
        // ANCHOR_END: example

        Expect(joined.DevAddr == DevAddr, "the device takes the address it was granted");
        Expect(
            received.Payload.AsSpan().SequenceEqual("level=high"u8),
            "and the network reads the reading it sent");

        Device();
    }

    /// <summary>Runs the end device example.</summary>
    private static void Device()
    {
        // ANCHOR: device
        byte[] rootKey = new byte[16];
        Array.Fill(rootKey, (byte)7);
        byte[] devEui = Convert.FromHexString("70B3D57ED0051234");
        byte[] joinEui = new byte[8];

        // The device owns no radio and no clock. It takes the time in microseconds and says
        // what to put on the air, so the same code runs over an SX1276, an SX1262, or nothing
        // at all. This one's radio puts out 2 to 20 dBm, and its seed would come from the
        // radio's noise.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Us915);
        var settings = new LorawanDeviceSettings(2, 20) { Seed = 1 };
        static string Mhz(uint hz) => (hz / 1e6).ToString("F1", CultureInfo.InvariantCulture);
        using var credentials = new LorawanDevice(devEui, joinEui, rootKey);
        using LorawanEndDevice node = LorawanEndDevice.OverTheAir(plan, credentials, settings);

        // A US915 device joins in passes over the band, one 125 kHz channel from each group
        // of eight. The accept is due on the downlink channel that join channel is answered on.
        LorawanTransmission request = node.Join(1, 0);
        Console.WriteLine(
            $"join      {Mhz(request.FrequencyHz)} MHz at DR{request.DataRate}, {request.OutputDbm} dBm, " +
            $"{request.AirtimeMicros / 1000} ms on air; " +
            $"the accept is due {request.Rx1.DelayMicros / 1_000_000} s later on {Mhz(request.Rx1.FrequencyHz)} MHz");

        // The network answers, and the device takes its address and session from the accept.
        var network = new LorawanGrant(appNonce: 2, netId: 19, devAddr: 0x26012E43);
        if (node.Heard(network.Accept(rootKey, 1), 7) is LorawanHeard.Joined joined)
        {
            Console.WriteLine($"joined    as 0x{joined.DevAddr:X8}");
        }

        // A confirmed reading. While it waits on its windows, the device refuses to send another.
        LorawanTransmission reading = node.Send(2, "21.5"u8, 10_000_000, confirmed: true);
        Console.WriteLine(
            $"uplink    {Mhz(reading.FrequencyHz)} MHz at DR{reading.DataRate}; " +
            $"the answer is due {reading.Rx1.DelayMicros / 1_000_000} s later on {Mhz(reading.Rx1.FrequencyHz)} MHz");
        try
        {
            node.Send(2, "21.6"u8, 10_000_000);
        }
        catch (LorawanDeviceException error) when (error.Kind == LorawanDeviceErrorKind.Busy)
        {
            Console.WriteLine("busy      the reading before still waits on its windows");
        }

        // The network acknowledges it and sends a setting back on the same port.
        using LorawanSession networkSession = network.Session(rootKey, 1);
        byte[] answer = networkSession.EncodeDownlink(0, 2, "set=19.0"u8, new LorawanOptions { Ack = true });
        if (node.Heard(answer, 7) is LorawanHeard.Data data)
        {
            LorawanDelivery delivery = data.Delivery;
            string acknowledged = delivery.Acknowledged ? "true" : "false";
            Console.WriteLine(
                $"downlink  acknowledged: {acknowledged}, port {delivery.Port ?? 0} says {Encoding.UTF8.GetString(delivery.Payload)}");
        }

        // Before sleeping, the device saves what it settled with the network. After the power
        // cut a fresh device resumes it on a clock that starts over, and sends its next reading
        // with no join.
        byte[] saved = node.Save(12_000_000);
        using LorawanEndDevice woken = LorawanEndDevice.OverTheAir(plan, credentials, settings);
        woken.Resume(saved, 0);
        LorawanTransmission next = woken.Send(2, "21.7"u8, 5_000_000);
        Console.WriteLine(
            $"resumed   {saved.Length} saved bytes; the next reading goes out as uplink {woken.FcntUp - 1} without joining again");
        // ANCHOR_END: device

        Expect(next.CarriesPayload, "the resumed device sends its reading");
        Expect(woken.DevAddr == 0x26012E43, "on the address it joined with");
    }
}
