using Pamoja.Lora;
using Pamoja.Radios;

using static Guides.Guide;

namespace Guides;

/// <summary>The LoRa radio guide example; see docs/guides/radios.md.</summary>
public static class RadiosGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz,
        // through a 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP
        // there, and the antenna and pigtail decide how hard the amplifier may drive
        // under that cap.
        using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        const uint Frequency = 868_100_000;
        LoraLink link = eu868.LinkSettings(3)!;
        var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        sbyte ceiling = eu868.MaxEirpDbm(Frequency);
        Sx126xTxPower power = Sx126x.TxPowerUnderCeiling(Sx126xAmplifier.HighPower, whip, ceiling);
        Console.WriteLine($"power     {power.SettingDbm} dBm under a {ceiling} dBm EIRP ceiling");

        // The commands in the order section 14.2 of the datasheet gives, each sent in its
        // own SPI transaction once BUSY is low. The chip gives up on the frame a second
        // after its airtime.
        ulong airtime = link.AirtimeMicros(10);
        Sx126xIrq events = Sx126xIrq.TxDone | Sx126xIrq.Timeout;
        (string Name, byte[] Bytes)[] commands =
        [
            ("standby", Sx126x.SetStandby()),
            ("packet type", Sx126x.SetPacketTypeLora()),
            ("frequency", Sx126x.SetRfFrequency(Frequency)),
            ("pa config", Sx126x.SetPaConfig(power)),
            ("tx params", Sx126x.SetTxParams(power, 40)),
            ("modulation", Sx126x.SetLoraModulationParams(link)),
            ("packet", Sx126x.SetLoraPacketParams(link, 10, false)),
            ("irq", Sx126x.SetDioIrqParams(events, events)),
            ("tx", Sx126x.SetTx(airtime + 1_000_000)),
        ];
        foreach ((string name, byte[] bytes) in commands)
        {
            Console.WriteLine($"{name,-12}{string.Join(" ", bytes.Select(b => b.ToString("x2")))}");
        }

        // Once the frame has left, GetIrqStatus answers with TxDone, and the status byte
        // shows the chip back in standby.
        Sx126xIrq irq = Sx126x.Irq([0x00, 0x01]);
        bool sent = irq.HasFlag(Sx126xIrq.TxDone);
        bool timedOut = irq.HasFlag(Sx126xIrq.Timeout);
        Console.WriteLine($"sent      tx done {sent}, timed out {timedOut}");
        Sx126xStatus status = Sx126x.Status(0x2C);
        Console.WriteLine($"status    {status.ChipMode}, {status.CommandStatus}");

        // A frame that arrives later comes with the signal levels it was heard at.
        Sx126xPacketStatus heard = Sx126x.PacketStatus([0xDB, 0xF6, 0xE0]);
        Console.WriteLine($"received  RSSI {heard.RssiDbm} dBm, SNR {heard.SnrDb} dB");

        // The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime
        // buys ninety-nine times as long in silence before the next.
        using var guard = new RadioDutyCycle(eu868.DutyCyclePermille(Frequency)!.Value);
        ulong held = guard.Transmitted(0, link, 10);
        Console.WriteLine($"airtime   {held} us, next frame after {guard.WaitMicros(0)} us");
        // ANCHOR_END: example

        // The bytes each command carries are pinned once, in the crate tests and the
        // generated conformance vectors, so a guide asserts behavior instead.
        Expect(power.SettingDbm == 14, "the whip and pigtail leave 14 dBm under the ceiling");
        Expect(commands.Length == 9, "a transmission takes nine commands");
        Expect(sent && !timedOut, "the frame left before the chip gave up on it");
        Expect(held == airtime, "the guard records the frame's own airtime");
        Expect(!guard.Ready(0), "a 1% sub-band owes silence after a frame");
        Expect(guard.Ready(held * 100), "and allows the next once it has passed");

        // ANCHOR: rfm95w
        // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and
        // the same 16 dBm ceiling leave it the same 14 dBm, set through three registers.
        using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        const uint Channel = 868_100_000;
        LoraLink dr3 = band.LinkSettings(3)!;
        var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
            Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));
        Console.WriteLine(
            $"rfm95w    {rfm95w.OutputDbm} dBm on PA_BOOST: RegPaConfig {rfm95w.PaConfig:x2}, " +
            $"RegPaDac {rfm95w.PaDac:x2}, RegOcp {rfm95w.Ocp:x2}");

        // The carrier and the modem go into registers while the chip stands by, and TX mode
        // sends the frame the FIFO holds.
        Sx127xModem modem = Sx127x.Modem(dr3, Channel);
        Console.WriteLine($"carrier   RegFrf {Sx127x.FrequencyWord(Channel):x6}");
        Console.WriteLine(
            $"modem     RegModemConfig {modem.ModemConfig1:x2} {modem.ModemConfig2:x2} " +
            $"{modem.ModemConfig3:x2}");
        Console.WriteLine($"tx mode   RegOpMode {Sx127x.LoraOpMode(Sx127xMode.Tx):x2}");

        // A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers
        // give its levels on the high frequency port.
        var flags = (Sx127xIrq)0x50;
        bool received = flags.HasFlag(Sx127xIrq.RxDone);
        bool corrupt = flags.HasFlag(Sx127xIrq.PayloadCrcError);
        Console.WriteLine($"irq       rx done {received}, crc error {corrupt}");
        Sx127xPacketStatus packet = Sx127x.PacketStatus([0xF6, 0x30], Channel);
        Console.WriteLine(
            $"received  RSSI {packet.RssiDbm} dBm, SNR {packet.SnrDb} dB, signal {packet.SignalRssiDbm} dBm");

        // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
        bool Fits(byte dataRate) => Sx126x.Llcc68Supports(band.LinkSettings(dataRate)!);
        Console.WriteLine($"llcc68    DR3 {Fits(3)}, DR2 {Fits(2)}");
        // ANCHOR_END: rfm95w

        Expect(rfm95w.OutputDbm == 14, "PA_BOOST takes the same 14 dBm under the ceiling");
        Expect(rfm95w.PaConfig == 0xFC, "on PA_BOOST with OutputPower 12");
        Expect(modem.ModemConfig2 == 0x94, "SF9 with a CRC");
        Expect(received && !corrupt, "the packet arrived whole");
        Expect(Fits(3) && !Fits(2), "an LLCC68 carries DR3 but not DR2");
    }
}
