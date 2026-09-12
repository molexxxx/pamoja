using System.Text;

using Pamoja.Gateway;
using Pamoja.Lora;
using Pamoja.Lorawan;

using static Guides.Guide;

namespace Guides;

/// <summary>The LoRaWAN gateway guide example; see docs/guides/gateway.md.</summary>
public static class GatewayGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
        const string GatewayEui = "b827ebfffe010203";
        var dr5 = new LoraLink(7, 125_000);

        // It heard a packet on 868.1 MHz, and forwards it with the levels it was heard at and
        // the concentrator's own timestamp of the reception.
        var heard = new GatewayRxpk(868_100_000, Encoding.UTF8.GetBytes("TEST_PACKET_1234"))
        {
            Link = dr5,
            RssiDbm = -35,
            SnrDb = 5.1,
            TimestampMicros = 3_512_348_611,
        };
        byte[] datagram = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PushData, 0x1234)
        {
            GatewayEui = GatewayEui,
            Packets = [heard],
        });
        Console.WriteLine($"push      {datagram.Length} bytes, token {0x1234:x4}");

        // The server reads it. Nothing about the packet has to be decoded by hand: the
        // frequency is in hertz, the datarate identifier is the link settings, and the payload
        // is bytes.
        GatewayRxpk received = Gateway.Parse(datagram).Packets[0];
        Console.WriteLine(
            $"heard     {received.FrequencyHz} Hz at SF{received.Link!.SpreadingFactor}, " +
            $"{received.Link.BandwidthHz / 1000} kHz, {received.RssiDbm} dBm, " +
            $"SNR {received.SnrDb} dB, {received.Payload.Length} bytes");

        // Every uplink is acknowledged at once, by token, before anything is processed.
        GatewayPacket ack = Gateway.Acknowledgment(Gateway.Parse(datagram))!;
        Console.WriteLine($"ack       {Gateway.Encode(ack).Length} bytes");

        // Later the server sends one back, at the concentrator timestamp that hits the device's
        // receive window, with the inverted polarity a LoRaWAN device listens for.
        byte[] downlink = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, 0x00AB)
        {
            Transmit = new GatewayTxpk(869_525_000, Encoding.UTF8.GetBytes("downlink"))
            {
                Link = dr5,
                TimestampMicros = 3_513_348_611,
                PowerDbm = 27,
                InvertPolarity = true,
                WithoutCrc = true,
            },
        });
        GatewayTxpk transmit = Gateway.Parse(downlink).Transmit!;
        Console.WriteLine(
            $"downlink  {transmit.FrequencyHz} Hz at {transmit.PowerDbm} dBm, " +
            $"inverted IQ {transmit.InvertPolarity}");

        // The gateway answers with what became of it. A packet already scheduled in that window
        // is refused rather than dropped silently.
        byte[] refused = Gateway.Encode(new GatewayPacket(GatewayPacketKind.TxAck, 0x00AB)
        {
            GatewayEui = GatewayEui,
            TxStatus = GatewayTxStatus.CollisionPacket,
        });
        GatewayTxStatus status = Gateway.Parse(refused).TxStatus!.Value;
        Console.WriteLine(
            $"txack     {Gateway.NameOf(status)}, scheduled {status == GatewayTxStatus.None}");
        // ANCHOR_END: example

        Expect(
            Encoding.UTF8.GetString(received.Payload) == "TEST_PACKET_1234",
            "the payload crosses as bytes");
        Expect(Gateway.Encode(ack).Length == 4, "an acknowledgment is four bytes");
        Expect(status == GatewayTxStatus.CollisionPacket, "the refusal survives the round trip");

        Network();
    }

    /// <summary>Runs the network side of the example.</summary>
    private static void Network()
    {
        // ANCHOR: network
        // One site, on the band it operates in, admitting one device it was told about.
        byte[] devEui = new byte[8];
        Array.Fill(devEui, (byte)0x11);
        byte[] appEui = new byte[8];
        Array.Fill(appEui, (byte)0x22);
        byte[] appKey = new byte[16];
        Array.Fill(appKey, (byte)0x33);

        var dr5 = new LoraLink(7, 125_000);
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        using var site = new GatewayNetwork(plan, 0x00002A, firstDevAddr: 0x26010001);
        site.Register(devEui, appEui, appKey);

        // The gateway forwards a join request it heard. Nothing about the device is known here
        // beyond the key it was provisioned with, which is what verifies the request.
        using var joiner = new LorawanDevice(devEui, appEui, appKey);
        GatewayNetworkEvent joined = site.Uplink(
            new GatewayRxpk(868_100_000, joiner.JoinRequest(0x0102))
            {
                Link = dr5,
                TimestampMicros = 1_000_000,
            });
        Console.WriteLine(
            $"joined    0x{joined.DevAddr:x8} at {joined.Accept!.TimestampMicros} us, " +
            $"inverted IQ {joined.Accept!.InvertPolarity.ToString().ToLowerInvariant()}");

        // The device reads the accept and sends a reading. The site decrypts it and says where
        // an answer goes, which is the uplink window plus the delay the region recommends.
        using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, 0x0102);
        using LorawanSession activated = granted.Session();
        GatewayNetworkEvent carried = site.Uplink(
            new GatewayRxpk(868_100_000, activated.EncodeUplink(0, 2, "21.5"u8))
            {
                Link = dr5,
                TimestampMicros = 9_000_000,
            });
        Console.WriteLine(
            $"uplink    frame {carried.Fcnt}, {carried.Payload!.Length} bytes, " +
            $"answer at {carried.Slot!.TimestampUs} us on {carried.Slot!.FrequencyHz} Hz");

        // The answer goes out in that window, encrypted with the session the join granted.
        GatewayTxpk answer = site.Answer(carried.DevAddr, carried.Slot!, 2, "ok"u8);
        Console.WriteLine(
            $"downlink  {answer.Payload.Length} bytes at {answer.TimestampMicros} us");

        // A gateway hears every network in range, and a frame from one this site never granted
        // is reported rather than refused.
        using LorawanSession elsewhere = new LorawanSession(0x12345678, NetworkKey(0x09), NetworkKey(0x08));
        GatewayNetworkEvent stranger = site.Uplink(
            new GatewayRxpk(868_100_000, elsewhere.EncodeUplink(0, 1, "hello"u8)) { Link = dr5 });
        Console.WriteLine($"foreign   0x{stranger.DevAddr:x8} belongs to another network");
        // ANCHOR_END: network

        Expect(
            Encoding.UTF8.GetString(carried.Payload!) == "21.5",
            "the reading is decrypted");
        Expect(carried.Slot!.TimestampUs == 10_000_000, "the answer goes one second later");
        Expect(
            stranger.Outcome == GatewayNetworkOutcome.Foreign,
            "another network is reported, not refused");
    }

    /// <summary>Builds a session key of one repeated byte.</summary>
    private static byte[] NetworkKey(byte value)
    {
        byte[] key = new byte[16];
        Array.Fill(key, value);
        return key;
    }
}
