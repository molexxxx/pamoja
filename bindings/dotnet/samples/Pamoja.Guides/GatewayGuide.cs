using System.Text;

using Pamoja.Gateway;
using Pamoja.Lora;

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
    }
}
