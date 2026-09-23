// ANCHOR: example
using System.Diagnostics;
using System.Text;

using Pamoja.Lora;
using Pamoja.Radios;

namespace Boards.RaspberryPi;

/// <summary>
/// A LoRa radio on the header: an RFM95W breakout on SPI0, beaconing a reading and printing
/// every frame it hears in between. Wire the breakout's VIN to a 3V3 pin, GND to ground,
/// SCK to GPIO11, MISO to GPIO9, MOSI to GPIO10, CS to GPIO8 (CE0), and RST to GPIO25, and
/// screw on an antenna for the band before powering it. Two boards running it hear each
/// other.
/// </summary>
public static class Radio
{
    // The header's first SPI chip select, the GPIO chip its lines are on, and the line the
    // breakout's reset pin is wired to.
    private const string Spi = "/dev/spidev0.0";
    private const string Chip = "/dev/gpiochip0";
    private const uint ResetLine = 25;

    // The channel this node uses, the data rate it sends at, and how long it listens
    // between beacons.
    private const uint FrequencyHz = 868_100_000;
    private const byte DataRate = 3;
    private static readonly TimeSpan Listen = TimeSpan.FromSeconds(10);

    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The regional plan decides the channel's power ceiling and its duty cycle, so no
        // limit below is a number anyone has to remember.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        LoraLink link = plan.LinkSettings(DataRate)!;
        sbyte ceilingDbm = plan.MaxEirpDbm(FrequencyHz);
        uint permille = plan.DutyCyclePermille(FrequencyHz)!.Value;

        // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against
        // the ceiling and the pigtail's loss counts for it, so the amplifier takes what is
        // left.
        var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };
        sbyte outputDbm = (sbyte)Math.Floor(whip.MaxTransmitPowerDbm(ceilingDbm));

        // Opening resets the chip and reads its version back, so a wiring mistake is caught
        // here rather than on the first frame.
        using LoraRadio radio = LoraRadio.OpenSx127x(
            new LoraRadioWiring(Spi, Chip, ResetLine), new Sx127xBoard(Sx127xPaOutput.PaBoost));
        radio.Configure(new LoraRadioConfig(FrequencyHz, link, outputDbm));
        Console.WriteLine(
            $"beacon on {FrequencyHz} Hz at DR{DataRate}, {outputDbm} dBm under a {ceilingDbm} dBm ceiling");

        // The duty cycle is the radio's other budget: each frame buys silence in proportion
        // to its airtime, and the guard says when the next one may go out.
        using var duty = new RadioDutyCycle(permille);
        var clock = Stopwatch.StartNew();
        int reading = 0;

        while (true)
        {
            // Listening returns as soon as a frame arrives, and a frame comes with the
            // levels it was heard at: how strong it was, and how far above the noise.
            LoraReception heard = radio.Receive(Listen);
            if (heard.Outcome == LoraReceptionOutcome.Frame)
            {
                Console.WriteLine(
                    $"heard  {Encoding.UTF8.GetString(heard.Payload!)} at {heard.RssiDbm:F0} dBm, SNR {heard.SnrDb:F1} dB");
            }
            else if (heard.Outcome == LoraReceptionOutcome.Corrupt)
            {
                Console.WriteLine("heard  a frame whose CRC failed");
            }

            ulong nowUs = (ulong)(clock.Elapsed.Ticks / (TimeSpan.TicksPerMillisecond / 1000));
            if (duty.Ready(nowUs))
            {
                string frame = $"pi reading {reading}";
                ulong airtimeUs = radio.Transmit(Encoding.UTF8.GetBytes(frame));
                duty.Transmitted(nowUs, link, frame.Length);
                Console.WriteLine($"sent   {frame} in {airtimeUs} us on air");
                reading++;
            }
        }
    }
}
// ANCHOR_END: example
