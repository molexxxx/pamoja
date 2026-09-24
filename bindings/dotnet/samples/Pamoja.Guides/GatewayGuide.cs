using System.Net;
using System.Net.Sockets;
using System.Text;

using Pamoja;
using Pamoja.Core;
using Pamoja.Gateway;
using Pamoja.Lora;
using Pamoja.Lorawan;

using static System.FormattableString;
using static Guides.Guide;

namespace Guides;

/// <summary>The LoRaWAN gateway guide example; see docs/guides/gateway.md.</summary>
public static class GatewayGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        TradeDatagrams();
        JoinASite();
        HoldAStationSession();
        ServeBesideTheGateway();
    }

    /// <summary>
    /// A gateway holds its downlink path open, forwards a reading with its own counts, and is
    /// asked to transmit twice, once in time and once too late.
    /// </summary>
    private static void TradeDatagrams()
    {
        // ANCHOR: example
        // A gateway on a Raspberry Pi, whose identifier is written from its network interface.
        const string GatewayEui = "b827ebfffe010203";

        // Every few seconds it sends a PULL_DATA, which holds a path open through whatever
        // translates its address, so the server has somewhere to send a downlink. The server
        // answers each one, and a gateway that stops hearing answers knows the path is gone.
        byte[] pull = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullData, 0x7A01)
        {
            GatewayEui = GatewayEui,
        });
        GatewayPacket held = Gateway.Acknowledgment(Gateway.Parse(pull))!;
        Console.WriteLine(
            $"pull      {pull.Length} bytes out and {Gateway.Encode(held).Length} back hold the downlink path open");

        // A node sends a reading, and the gateway hears it on 868.1 MHz at SF9, near the edge
        // of its range. It forwards the frame as it arrived, with the levels, the
        // concentrator's own timestamp, and its counts since the last report. It holds no key
        // and reads none of it.
        using var node = new LorawanSession(0x26010001, Filled(0x44), Filled(0x55));
        byte[] frame = node.EncodeUplink(7, 2, "21.5"u8);
        var heard = new GatewayRxpk(868_100_000, frame)
        {
            Link = new LoraLink(9, 125_000),
            RssiDbm = -97,
            SnrDb = -3.2,
            TimestampMicros = 3_512_348_611,
        };
        var counts = new GatewayStat { Received = 2, ReceivedOk = 1, Forwarded = 1, AcknowledgedPercent = 100 };
        byte[] datagram = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PushData, 0x1234)
        {
            GatewayEui = GatewayEui,
            Packets = [heard],
            Status = counts,
        });
        Console.WriteLine(
            $"push      a reading and the gateway's counts, {datagram.Length} bytes, token {0x1234:x4}");

        // The server reads it. The frequency is in hertz, the datarate identifier is the link
        // settings, and the payload is bytes, so nothing is decoded by hand.
        GatewayPacket forwarded = Gateway.Parse(datagram);
        GatewayRxpk received = forwarded.Packets[0];
        Console.WriteLine(Invariant(
            $"heard     {received.FrequencyHz} Hz at SF{received.Link!.SpreadingFactor}, {received.Link.BandwidthHz / 1000} kHz, ") +
            Invariant($"{received.RssiDbm:F0} dBm, SNR {received.SnrDb:F1} dB, ") +
            $"CRC {received.Crc.ToString().ToLowerInvariant()}, {received.Payload.Length} bytes");
        GatewayStat report = forwarded.Status!;
        Console.WriteLine(Invariant(
            $"counts    {report.Received} received, {report.ReceivedOk} with a good CRC, {report.Forwarded} forwarded, {report.AcknowledgedPercent:F1}% acknowledged"));

        // It is acknowledged at once, by token, before anything in it is read.
        GatewayPacket ack = Gateway.Acknowledgment(Gateway.Parse(datagram))!;
        Console.WriteLine($"ack       token {ack.Token:x4} acknowledged in {Gateway.Encode(ack).Length} bytes");

        // An answer goes back in a PULL_RESP, timed in the concentrator's own microseconds for
        // the device's first receive window, a second after the uplink ended, with the
        // inverted polarity a LoRaWAN device listens for.
        byte[] answer = node.EncodeDownlink(0, 2, "ok"u8);
        byte[] pullResp = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, 0x00AB)
        {
            Transmit = new GatewayTxpk(868_100_000, answer)
            {
                Link = new LoraLink(9, 125_000),
                TimestampMicros = 3_513_348_611,
                PowerDbm = 14,
                InvertPolarity = true,
            },
        });
        GatewayTxpk transmit = Gateway.Parse(pullResp).Transmit!;
        string iq = transmit.InvertPolarity ? "IQ inverted" : "IQ upright";
        Console.WriteLine(
            $"downlink  at {transmit.TimestampMicros} us on {transmit.FrequencyHz} Hz, {transmit.PowerDbm} dBm, {iq}");

        // The gateway answers each PULL_RESP with a TX_ACK saying what became of it:
        // scheduled, or refused with a reason, such as a window that had already passed.
        foreach (GatewayTxStatus said in new[] { GatewayTxStatus.None, GatewayTxStatus.TooLate })
        {
            byte[] reported = Gateway.Encode(new GatewayPacket(GatewayPacketKind.TxAck, 0x00AB)
            {
                GatewayEui = GatewayEui,
                TxStatus = said,
            });
            GatewayTxStatus status = Gateway.Parse(reported).TxStatus!.Value;
            string meaning = status == GatewayTxStatus.None
                ? "it goes out in the device's window"
                : "it was not sent";
            Console.WriteLine($"txack     {Gateway.NameOf(status)}: {meaning}");
        }
        // ANCHOR_END: example

        Expect(Gateway.Encode(held).Length == 4, "a PULL_ACK is four bytes");
        Expect(received.Payload.AsSpan().SequenceEqual(frame), "the frame crosses as bytes");
        Expect(ack.Token == 0x1234, "the acknowledgment carries the token");
        Expect(transmit.TimestampMicros == 3_513_348_611, "the downlink is timed for the window");
    }

    /// <summary>
    /// A device joining a site, the reading it sends once it has, a copy of that reading played
    /// back, and a frame from another network, as the network side of one gateway sees them.
    /// </summary>
    private static void JoinASite()
    {
        // ANCHOR: network
        // One site, on the band it operates in, admitting one device it was told about: its EUI
        // from its label, the application it joins, and the root key it was provisioned with.
        byte[] devEui = Convert.FromHexString("70b3d57ed0001234");
        byte[] joinEui = Convert.FromHexString("70b3d57ed0000000");
        byte[] appKey = Filled(0x2B);
        using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        using var site = new GatewayNetwork(eu868, 0x000013, firstDevAddr: 0x26010001);
        site.Register(devEui, joinEui, appKey);

        // The gateway forwards a join request it heard. Nothing about the device is known here
        // beyond the key, which is what verifies the request, and the accept is timed for the
        // join window, five seconds after the request.
        LoraLink dr5 = eu868.LinkSettings(5)!;
        using var joiner = new LorawanDevice(devEui, joinEui, appKey);
        const uint HeardAt = 1_000_000;
        GatewayNetworkEvent joined = site.Uplink(
            new GatewayRxpk(868_100_000, joiner.JoinRequest(0x0102)) { Link = dr5, TimestampMicros = HeardAt });
        uint acceptedAt = joined.Accept!.TimestampMicros!.Value;
        Console.WriteLine(
            $"joined    0x{joined.DevAddr:x8}, accepted at {acceptedAt} us, {(acceptedAt - HeardAt) / 1_000_000} s after the request");

        // The device reads the accept and sends a reading. The site decrypts it and says where
        // an answer goes: the uplink's own channel, a second after it ended.
        using LorawanJoinAccept granted = joiner.AcceptJoin(joined.Accept!.Payload, 0x0102);
        using LorawanSession device = granted.Session();
        var carried = new GatewayRxpk(868_100_000, device.EncodeUplink(0, 2, "21.5"u8))
        {
            Link = dr5,
            TimestampMicros = 9_000_000,
        };
        GatewayNetworkEvent reading = site.Uplink(carried);
        Console.WriteLine(
            $"uplink    frame {reading.Fcnt} on port {reading.Fport} says {Encoding.UTF8.GetString(reading.Payload!)}, " +
            $"answer at {reading.Slot!.TimestampUs} us on {reading.Slot!.FrequencyHz} Hz");

        // The answer goes out in that window, encrypted with the session the join granted.
        GatewayTxpk reply = site.Answer(reading.DevAddr, reading.Slot!, 2, "ok"u8);
        Console.WriteLine($"answer    {reply.Payload.Length} bytes at {reply.TimestampMicros} us");

        // The same frame again, as a replay would send it, is refused: its counter was seen.
        try
        {
            site.Uplink(carried);
            throw new InvalidOperationException("a counter is taken once");
        }
        catch (PamojaException refused)
        {
            Console.WriteLine($"replay    {refused.Message}");
        }

        // A gateway hears every network in range, and a frame from one this site never
        // granted is reported as another network's rather than refused.
        using var elsewhere = new LorawanSession(0x12345678, Filled(0x09), Filled(0x08));
        GatewayNetworkEvent stranger = site.Uplink(
            new GatewayRxpk(868_300_000, elsewhere.EncodeUplink(0, 1, "hello"u8)) { Link = dr5 });
        Console.WriteLine($"foreign   0x{stranger.DevAddr:x8} belongs to another network");
        // ANCHOR_END: network

        Expect(Encoding.UTF8.GetString(reading.Payload!) == "21.5", "the reading is decrypted");
        Expect(reading.Slot!.TimestampUs == 10_000_000, "the answer goes one second later");
        Expect(stranger.Outcome == GatewayNetworkOutcome.Foreign, "another network is reported, not refused");
    }

    /// <summary>
    /// A Basics Station session, played from both sides: the station finds its server, says
    /// what it is, and reports a frame; the server answers it in a receive window; and the
    /// station reports that the answer went out.
    /// </summary>
    private static void HoldAStationSession()
    {
        // ANCHOR: station
        // The station asks its configured address where its network server is, naming itself.
        // The server reads who asked, and sends it to the websocket its session runs on.
        byte[] station = Convert.FromHexString("b827ebfffe010203");
        string asking = GatewayStation.Discovery(station);
        Console.WriteLine($"ask       {GatewayStation.DiscoveryPath} {asking}");
        byte[] asked = GatewayStation.DiscoveryParse(asking);
        string answer = GatewayStation.RouterAccepted(
            asked, Convert.FromHexString("0000000000000001"), "ws://lns.example.invalid:3001/router");
        Console.WriteLine($"open      {GatewayStation.RouterParse(answer).Uri}");

        // Once the websocket is open the station speaks first, saying what it is.
        string hello = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Version)
        {
            Station = "pamoja",
            Firmware = PamojaCore.Version,
            Package = "pamoja-gateway",
            Model = "linux",
            Protocol = GatewayStation.ProtocolVersion,
            Features = "gps",
        });
        GatewayStationMessage said = GatewayStation.Parse(hello);
        Console.WriteLine($"version   {said.Station} {said.Firmware} on {said.Model}, protocol {said.Protocol}");

        // The radio hears a node's reading 3512.348611 seconds into the station's first run. A
        // station holds no key, so it splits the frame into the fields the protocol names and
        // lets the server judge them, with its own clock for the moment it arrived.
        using var node = new LorawanSession(0x26010001, Filled(0x44), Filled(0x55));
        long heardAt = GatewayStation.Xtime(0, 1, 3_512_348_611);
        string updf = GatewayStation.Encode(GatewayStation.Heard(
            node.EncodeUplink(7, 2, "21.5"u8),
            5,
            868_100_000,
            new GatewayStationLevels(0, heardAt) { Rssi = -97.0, Snr = -3.2 }));
        GatewayStationMessage uplink = GatewayStation.Parse(updf);
        Console.WriteLine(
            $"updf      0x{uplink.DevAddr:x8} counter {uplink.Fcnt} on port {uplink.Fport}, " +
            $"DR{uplink.DataRate}, {uplink.Payload.Length} bytes still encrypted");

        // The server answers in the receive windows the region gives: the first at the
        // uplink's own rate and channel, the second where the plan fixes it. It hands the
        // station's clock back untouched, so the station can time the answer from the moment
        // it heard the uplink.
        using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        byte rx1Rate = eu868.Rx1DataRate(uplink.DataRate, 0)!.Value;
        LoraRx2 rx2 = eu868.Rx2();
        string dnmsg = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Downlink)
        {
            DevEui = Convert.FromHexString("70b3d57ed0001234"),
            Class = 0,
            Diid = 1,
            Payload = node.EncodeDownlink(0, 2, "ok"u8),
            RxDelay = 1,
            Rx1 = new GatewayStationWindow(rx1Rate, uplink.FrequencyHz),
            Rx2 = new GatewayStationWindow(rx2.DataRate, rx2.FrequencyHz),
            Priority = 0,
            Xtime = uplink.Levels!.Xtime,
            Rctx = uplink.Levels!.Rctx,
        });
        GatewayStationMessage told = GatewayStation.Parse(dnmsg);
        byte delay = told.RxDelay ?? 1;
        Console.WriteLine(
            $"dnmsg     RX1 DR{told.Rx1!.DataRate} on {told.Rx1!.FrequencyHz} Hz or " +
            $"RX2 DR{told.Rx2!.DataRate} on {told.Rx2!.FrequencyHz} Hz, {delay} s after the uplink");

        // The station opens the first window a second after the uplink on its own clock, puts
        // the answer on the air, and reports it by the identifier the server gave it.
        GatewayStationXtime uplinkAt = GatewayStation.XtimeParts(told.Xtime!.Value);
        long sentAt = GatewayStation.Xtime(uplinkAt.Unit, uplinkAt.Session, uplinkAt.Micros + (delay * 1_000_000L));
        string dntxed = GatewayStation.Encode(new GatewayStationMessage(GatewayStationKind.Transmitted)
        {
            Diid = told.Diid,
            DevEui = told.DevEui,
            Rctx = told.Rctx ?? 0,
            Xtime = sentAt,
            Txtime = GatewayStation.XtimeParts(sentAt).Micros / 1e6,
        });
        GatewayStationMessage reported = GatewayStation.Parse(dntxed);
        GatewayStationXtime went = GatewayStation.XtimeParts(reported.Xtime!.Value);
        Console.WriteLine($"dntxed    downlink {reported.Diid} went out at {went.Micros} us of run {went.Session}");
        // ANCHOR_END: station

        Expect(asked.AsSpan().SequenceEqual(station), "the server reads who asked");
        Expect(uplink.DevAddr == 0x26010001, "the address it came from");
        Expect(uplink.Fcnt == 7, "the counter it carried");
        Expect(Encoding.UTF8.GetString(uplink.Payload) != "21.5", "the payload stays encrypted");
        Expect(went.Micros == 3_513_348_611, "the answer goes a second after the uplink");
    }

    /// <summary>
    /// A network server on the Raspberry Pi the gateway daemon runs on, answering what the
    /// daemon forwards over UDP.
    /// </summary>
    private static void ServeBesideTheGateway()
    {
        // ANCHOR: hardware
        // The daemon forwards to 127.0.0.1 when it runs on the same Pi. To serve gateways
        // elsewhere, listen on 0.0.0.0 behind a firewall that admits only them: the protocol
        // has no authentication of its own.
        using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        using var site = new GatewayNetwork(eu868, 0x000013);
        site.Register(
            Convert.FromHexString("70b3d57ed0001234"),
            Convert.FromHexString("70b3d57ed0000000"),
            Filled(0x2B));

        using var socket = new UdpClient(AddressFamily.InterNetwork);
        try
        {
            socket.Client.Bind(new IPEndPoint(IPAddress.Loopback, Gateway.DefaultPort));
        }
        catch (SocketException)
        {
            Console.WriteLine($"absent    another program holds port {Gateway.DefaultPort}");
            return;
        }

        // A pamoja gateway holds its path open every five seconds, so six seconds of silence
        // means none is running. After that the server keeps answering until a minute passes
        // with nothing heard.
        socket.Client.ReceiveTimeout = 6_000;
        IPEndPoint? downlinks = null;
        ushort token = 0;
        while (true)
        {
            var from = new IPEndPoint(IPAddress.Any, 0);
            byte[] datagram;
            try
            {
                datagram = socket.Receive(ref from);
            }
            catch (SocketException quiet) when (quiet.SocketErrorCode == SocketError.TimedOut)
            {
                break;
            }

            socket.Client.ReceiveTimeout = 60_000;

            GatewayPacket packet;
            try
            {
                packet = Gateway.Parse(datagram);
            }
            catch (PamojaException why)
            {
                Console.WriteLine($"ignored   {why.Message}");
                continue;
            }

            if (Gateway.Acknowledgment(packet) is { } ack)
            {
                socket.Send(Gateway.Encode(ack), from);
            }

            switch (packet.Kind)
            {
                case GatewayPacketKind.PullData:
                    if (downlinks is null)
                    {
                        Console.WriteLine($"gateway   {packet.GatewayEui} holds its downlink path open");
                    }

                    downlinks = from;
                    break;
                case GatewayPacketKind.PushData:
                    foreach (GatewayRxpk arrived in packet.Packets)
                    {
                        if (arrived.Crc != GatewayCrc.Ok)
                        {
                            continue;
                        }

                        GatewayTxpk transmit;
                        try
                        {
                            GatewayNetworkEvent happened = site.Uplink(arrived);
                            if (happened.Outcome == GatewayNetworkOutcome.Joined)
                            {
                                Console.WriteLine($"joined    0x{happened.DevAddr:x8}");
                                transmit = happened.Accept!;
                            }
                            else if (happened.Outcome == GatewayNetworkOutcome.Data)
                            {
                                Console.WriteLine(
                                    $"reading   0x{happened.DevAddr:x8} says {Encoding.UTF8.GetString(happened.Payload!)}");
                                transmit = site.Answer(happened.DevAddr, happened.Slot!, 2, "ok"u8);
                            }
                            else
                            {
                                continue;
                            }
                        }
                        catch (PamojaException why)
                        {
                            Console.WriteLine($"refused   {why.Message}");
                            continue;
                        }

                        if (downlinks is not null)
                        {
                            token++;
                            byte[] resp = Gateway.Encode(new GatewayPacket(GatewayPacketKind.PullResp, token)
                            {
                                Transmit = transmit,
                            });
                            socket.Send(resp, downlinks);
                        }
                    }

                    break;
                case GatewayPacketKind.TxAck:
                    Console.WriteLine($"txack     {Gateway.NameOf(packet.TxStatus!.Value)}");
                    break;
            }
        }

        if (downlinks is null)
        {
            Console.WriteLine("absent    no gateway reported in, so nothing was answered");
        }
        // ANCHOR_END: hardware
    }

    /// <summary>Builds a key of one repeated byte.</summary>
    private static byte[] Filled(byte value)
    {
        byte[] key = new byte[16];
        Array.Fill(key, value);
        return key;
    }
}
