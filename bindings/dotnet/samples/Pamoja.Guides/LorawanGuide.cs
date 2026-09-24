using System.Globalization;
using System.Text;

using Pamoja;
using Pamoja.Gateway;
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

        // The network acknowledges it in the first window and sends a setting back on the same
        // port. Naming the window holds the frame to the length that window's data rate carries.
        using LorawanSession networkSession = network.Session(rootKey, 1);
        byte[] answer = networkSession.EncodeDownlink(0, 2, "set=19.0"u8, new LorawanOptions { Ack = true });
        if (node.Heard(answer, 7, LorawanReceiveWindow.Rx1) is LorawanHeard.Data data)
        {
            LorawanDelivery delivery = data.Delivery;
            string readingWas = delivery.Acknowledged ? "acknowledged" : "not acknowledged";
            Console.WriteLine(
                $"downlink  the reading was {readingWas}, and port {delivery.Port ?? 0} says {Encoding.UTF8.GetString(delivery.Payload)}");
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

        RelayedDevice();
    }

    /// <summary>Runs the relay example.</summary>
    private static void RelayedDevice()
    {
        // ANCHOR: relay
        // One site: a gateway on a hill, a relay on a rooftop in range of it, and a sensor
        // in a cellar the gateway cannot hear at all.
        byte[] rootKey = new byte[16];
        Array.Fill(rootKey, (byte)7);
        byte[] joinEui = new byte[8];
        byte[] relayEui = Convert.FromHexString("70B3D57ED0050001");
        byte[] sensorEui = Convert.FromHexString("70B3D57ED0050002");
        using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        var radio = new LorawanDeviceSettings(2, 14)
        {
            LowestHz = 863_000_000,
            HighestHz = 870_000_000,
            Seed = 1,
        };
        using var site = new GatewayNetwork(band, 0x00002A, firstDevAddr: 0x26010001);
        site.Register(relayEui, joinEui, rootKey);
        site.Register(sensorEui, joinEui, rootKey);
        static string Mhz(uint hz) => (hz / 1e6).ToString("F1", CultureInfo.InvariantCulture);
        static GatewayRxpk HeardAt(LorawanTransmission transmission, uint atMicros) =>
            new(transmission.FrequencyHz, transmission.Frame)
            {
                Link = transmission.Link,
                TimestampMicros = atMicros,
            };

        // The relay is an end device that also listens for others, so it joins the ordinary way.
        using var relayCredentials = new LorawanDevice(relayEui, joinEui, rootKey);
        using LorawanRelayNode rooftop = LorawanRelayNode.OverTheAir(
            band, relayCredentials, radio, LorawanXtalAccuracy.Ppm20, LorawanCadToRx.Symbols4);
        LorawanTransmission relayJoin = rooftop.Join(1, 1_000_000);
        GatewayNetworkEvent relayAccept = site.Uplink(HeardAt(relayJoin, 1_000_000));
        rooftop.HeardIn(LorawanReceiveWindow.Rx1, relayAccept.Accept!.Payload, 7);
        Console.WriteLine($"relay     joined as 0x{rooftop.DevAddr!.Value:X8}");

        // The sensor joins too. Its own uplinks never reach the gateway, but its join does,
        // because the cellar door is open while it is installed.
        using var sensorCredentials = new LorawanDevice(sensorEui, joinEui, rootKey);
        using LorawanEndDevice cellar = LorawanEndDevice.OverTheAir(band, sensorCredentials, radio);
        LorawanTransmission sensorJoin = cellar.Join(2, 20_000_000);
        GatewayNetworkEvent sensorAccept = site.Uplink(HeardAt(sensorJoin, 20_000_000));
        cellar.Heard(sensorAccept.Accept!.Payload, 7, LorawanReceiveWindow.Rx1);
        uint sensorAddr = cellar.DevAddr!.Value;

        // The network hands the relay the key that lets it verify the sensor's wake-up frames,
        // in a command riding on the relay's own downlink.
        LorawanTransmission relayEmpty = rooftop.SendEmpty(40_000_000);
        GatewayNetworkEvent relayCarried = site.Uplink(HeardAt(relayEmpty, 40_000_000));
        LorawanMacCommand trust = site.TrustCommand(sensorAddr, 0);
        GatewayTxpk configure = site.Command(rooftop.DevAddr!.Value, relayCarried.Slot!, [trust]);
        rooftop.HeardIn(LorawanReceiveWindow.Rx1, configure.Payload, 7);
        Console.WriteLine($"trusted   the relay now forwards for 0x{sensorAddr:X8}");

        // It scans once a second on the region's wake-on-radio channel.
        rooftop.Start(LorawanCadPeriodicity.Ms1000, 0);
        LorawanScan scan = rooftop.NextScan(60_000_000)!;
        Console.WriteLine(
            $"scan      {Mhz(scan.Carrier.FrequencyHz)} MHz at DR{scan.Carrier.DataRate} every second");

        // The sensor turns relay mode on. Its uplink now goes out behind a frame whose preamble
        // spans a whole scan period, because it does not yet know when the relay listens.
        cellar.UseRelay(true);
        LorawanTransmission reading = cellar.Send(2, "21.5"u8, 61_000_000);
        LorawanRelayExchange exchange = reading.Relay!;
        Console.WriteLine(
            $"wake      {exchange.WakeUp.Frame.Length} bytes with a " +
            $"{exchange.WakeUp.Link.PreambleSymbols}-symbol preamble, " +
            $"{(exchange.UplinkStartMicros - exchange.WakeUp.StartMicros) / 1000} ms before the uplink");

        // The relay hears it, knows the device, and answers with when it scanned, so every
        // frame after this one carries only the preamble the two clocks could have drifted apart.
        var woke = (LorawanWake.Uplink)rooftop.HeardWor(
            scan, exchange.WakeUp.Frame, -90, 4, scan.StartMicros + 500_000);
        LorawanRelayStatus said = cellar.HeardWorAck(woke.Acknowledgment!.Frame);
        Console.WriteLine(
            $"ack       the relay scans every {PeriodMillis(said.CadPeriodicity)} ms and " +
            $"forwards at DR{said.RelayDataRate}");

        // The uplink follows, and the relay wraps it in one of its own on port 226.
        ulong dueUs = rooftop.HeardUplink(reading.Frame, -88, 6, woke.Listen!.StartMicros + 100_000);
        LorawanTransmission forwarded = rooftop.Forward(dueUs);
        GatewayNetworkEvent relayed = site.Uplink(HeardAt(forwarded, (uint)dueUs));
        Console.WriteLine(
            $"forwarded {Encoding.UTF8.GetString(relayed.Payload!)} from 0x{relayed.DevAddr:X8}, " +
            $"heard by 0x{relayed.Relay!.Relay:X8} at {relayed.Relay!.RssiDbm} dBm");

        // The answer goes back the same way: the network answers the sensor, the relay unwraps
        // it and sends it on, and the sensor hears it in the window it keeps for a relay.
        GatewayTxpk answer = site.Answer(sensorAddr, relayed.Slot!, 2, "set=19.0"u8);
        var passed = (LorawanRelayHeard.Downlink)rooftop.HeardIn(
            LorawanReceiveWindow.Rx1, answer.Payload, 7);
        var delivered = (LorawanHeard.Data)cellar.Heard(
            passed.Forwarded.Frame, 7, LorawanReceiveWindow.Rxr);
        Console.WriteLine(
            $"downlink  port {delivered.Delivery.Port} says " +
            $"{Encoding.UTF8.GetString(delivered.Delivery.Payload)}, " +
            $"{exchange.Rxr.DelayMicros / 1_000_000} s after the uplink");
        // ANCHOR_END: relay

        Expect(relayed.Relay!.Relay == rooftop.DevAddr!.Value, "the relay carried the reading");
    }

    /// <summary>How many milliseconds a relay's scan period lasts.</summary>
    /// <param name="periodicity">The period, as an acknowledgment names it.</param>
    /// <returns>The period in milliseconds.</returns>
    private static int PeriodMillis(LorawanCadPeriodicity periodicity) => periodicity switch
    {
        LorawanCadPeriodicity.Ms500 => 500,
        LorawanCadPeriodicity.Ms250 => 250,
        LorawanCadPeriodicity.Ms100 => 100,
        LorawanCadPeriodicity.Ms50 => 50,
        LorawanCadPeriodicity.Ms20 => 20,
        _ => 1000,
    };
}
