using System.Text;

using Pamoja;
using Pamoja.Lora;
using Pamoja.Radios;

using static Guides.Guide;
using static System.FormattableString;

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

        // A simulated SX1262 stands in for the chip on the node's board, driven by the same
        // code that drives a real one, and it reports what that code told it.
        using SimulatedLoraChip chip = SimulatedLoraChip.Sx126x(new Sx126xBoard(Sx126xAmplifier.HighPower));
        using LoraRadio bench = chip.Radio();
        bench.Configure(new LoraRadioConfig(Frequency, link, power.SettingDbm));
        LoraTuning tuned = chip.Tuning();
        Console.WriteLine(Invariant(
            $"tuned     {tuned.FrequencyHz / 1e6:F1} MHz, SF{tuned.Link.SpreadingFactor} at {tuned.Link.BandwidthHz / 1000} kHz, {tuned.OutputDbm} dBm"));

        // The reading goes out, and the airtime comes back for the duty-cycle guard. The
        // sub-band that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine
        // times as long in silence before the next.
        byte[] reading = "level=0.42"u8.ToArray();
        ulong airtime = bench.Transmit(reading);
        Console.WriteLine($"sent      {chip.Sent()[0].Payload.Length} bytes, {airtime} us on air");
        using var guard = new RadioDutyCycle(eu868.DutyCyclePermille(Frequency)!.Value);
        guard.Transmitted(0, link, reading.Length);
        Console.WriteLine($"silence   the next frame starts {guard.WaitMicros(0)} us after this one did");

        // A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
        chip.Hear("ack"u8, -109, -2.5);
        LoraReception heard = bench.Receive(TimeSpan.FromSeconds(1));
        if (heard.Outcome == LoraReceptionOutcome.Frame)
        {
            Console.WriteLine(Invariant(
                $"received  {Encoding.UTF8.GetString(heard.Payload!)} at {heard.RssiDbm:F2} dBm, SNR {heard.SnrDb:F2} dB"));
        }

        // With nothing on the air the reception times out, and a frame whose CRC fails is
        // dropped rather than handed over.
        LoraReceptionOutcome quiet = bench.Receive(TimeSpan.FromSeconds(1)).Outcome;
        chip.HearCorrupt(-121, -12);
        LoraReceptionOutcome broken = bench.Receive(TimeSpan.FromSeconds(1)).Outcome;
        Console.WriteLine($"then      {quiet}, then {broken}");
        // ANCHOR_END: example

        Expect(power.SettingDbm == 14, "the whip and pigtail leave 14 dBm under the ceiling");
        Expect(tuned.FrequencyHz == Frequency, "the chip is tuned to the carrier asked for");
        Expect(chip.Sent()[0].Payload.SequenceEqual(reading), "and sent the reading");
        Expect(guard.WaitMicros(0) == airtime * 100, "a 1% sub-band owes ninety-nine airtimes");
        Expect(quiet == LoraReceptionOutcome.Timeout, "a quiet air times out");
        Expect(broken == LoraReceptionOutcome.Corrupt, "and a bad CRC is dropped");

        // ANCHOR: rfm95w
        // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and
        // the same 16 dBm ceiling leave it the same 14 dBm.
        using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        const uint Channel = 868_100_000;
        LoraLink dr3 = band.LinkSettings(3)!;
        var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
            Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));

        // The same driver calls tune it, through registers this time. Its synthesizer steps
        // in 61 Hz, so the carrier lands on the step nearest the one asked for.
        using SimulatedLoraChip module = SimulatedLoraChip.Sx127x(new Sx127xBoard(Sx127xPaOutput.PaBoost));
        using LoraRadio radio = module.Radio();
        radio.Configure(new LoraRadioConfig(Channel, dr3, rfm95w.OutputDbm));
        LoraTuning carrier = module.Tuning();
        uint off = carrier.FrequencyHz > Channel ? carrier.FrequencyHz - Channel : Channel - carrier.FrequencyHz;
        Console.WriteLine(
            $"rfm95w    {carrier.OutputDbm} dBm on PA_BOOST, carrier {carrier.FrequencyHz} Hz, {off} Hz from {Channel}");

        // The SX1276 gives a packet's strength in whole decibels, and works out the strength
        // of the signal itself from the SNR when it arrived under the noise.
        module.Hear("ack"u8, -109, -2.5);
        LoraReception packet = radio.Receive(TimeSpan.FromSeconds(1));
        if (packet.Outcome == LoraReceptionOutcome.Frame)
        {
            Console.WriteLine(Invariant(
                $"received  RSSI {packet.RssiDbm:F2} dBm, SNR {packet.SnrDb:F2} dB, signal {packet.SignalRssiDbm:F2} dBm"));
        }

        // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
        string Carries(byte dataRate) =>
            Sx126x.Llcc68Supports(band.LinkSettings(dataRate)!) ? "carries" : "cannot carry";
        Console.WriteLine($"llcc68    {Carries(3)} DR3 and {Carries(2)} DR2");
        // ANCHOR_END: rfm95w

        Expect(rfm95w.OutputDbm == 14, "PA_BOOST takes the same 14 dBm under the ceiling");
        Expect(carrier.OutputDbm == 14, "and the chip was set to it");
        Expect(off <= 61, "the carrier is within one synthesizer step");
        Expect(Carries(3) == "carries", "an LLCC68 carries DR3");
        OnALinuxBoard();
    }

    /// <summary>Opens the same radio on a Linux board, over spidev and a GPIO line.</summary>
    private static void OnALinuxBoard()
    {
        // ANCHOR: hardware
        // An RFM95W on a Raspberry Pi: the header's first chip select, with the module's
        // reset pin on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
        var wiring = new LoraRadioWiring("/dev/spidev0.0", "/dev/gpiochip0", 25);
        Console.WriteLine($"radio     an RFM95W on {wiring.Spi}, reset on GPIO{wiring.ResetLine}");

        // The channel and the power the same whip leaves under the same ceiling, now as the
        // number the radio is set to rather than the registers it goes into.
        using LoraChannelPlan band = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        const uint Channel = 868_100_000;
        LoraLink dr3 = band.LinkSettings(3)!;
        var antenna = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        Sx127xTxPower rfm95w = Sx127x.TxPowerUnderCeiling(
            Sx127xPaOutput.PaBoost, antenna, band.MaxEirpDbm(Channel));
        Console.WriteLine($"plan      {Channel} Hz at DR3, {rfm95w.OutputDbm} dBm on PA_BOOST");

        // Opening resets the chip and reads its version back, so a wiring mistake is caught
        // here rather than on the first frame. With no radio wired, this line prints.
        LoraRadio? radio = null;
        try
        {
            radio = LoraRadio.OpenSx127x(wiring, new Sx127xBoard(Sx127xPaOutput.PaBoost));
        }
        catch (PlatformNotSupportedException)
        {
        }
        catch (PamojaException)
        {
        }

        if (radio is null)
        {
            Console.WriteLine("absent    no radio answered, so nothing went out");
            return;
        }

        using (radio)
        {
            radio.Configure(new LoraRadioConfig(Channel, dr3, rfm95w.OutputDbm));
            ulong airtimeUs = radio.Transmit("21.5"u8);
            Console.WriteLine($"sent      a reading in {airtimeUs} us on air");
        }
        // ANCHOR_END: hardware
    }
}
