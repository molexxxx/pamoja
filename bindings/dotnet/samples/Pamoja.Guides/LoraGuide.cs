using Pamoja.Lora;

using static Guides.Guide;

namespace Guides;

/// <summary>The LoRa airtime guide example; see docs/guides/lora.md.</summary>
public static class LoraGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        // EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz,
        // the setting that reaches furthest and holds the channel longest.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        Expect(plan.Name == "EU863-870", "the plan names its band");
        LoraLink link = plan.LinkSettings(0)!;
        Expect(link.SpreadingFactor == 12, "DR0 is the slowest rate the band defines");

        // The published time on air for SF12 at 125 kHz, coding rate 4/5, an
        // eight-symbol preamble, an explicit header and CRC on, carrying ten bytes.
        ulong airtime = link.AirtimeMicros(10);
        Expect(airtime == 991_232, "the published time on air of a ten-byte frame");

        // 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
        // transmission buys ninety-nine times its own length in silence.
        uint permille = plan.DutyCyclePermille(868_100_000)!.Value;
        Expect(permille == 10, "the 868.1 MHz sub-band is limited to 1%");
        Expect(plan.MaxEirpDbm(868_100_000) == 16, "and to 16 dBm");
        Expect(
            link.MinOffTimeMicros(10, permille) == airtime * 99,
            "the silence a 1% duty cycle forces after one frame");

        // The airtime plus that silence is what one reading really costs, which is the
        // budget a deployment plans against: at SF12, thirty-six readings an hour.
        Expect(link.MessagesPerHour(10, permille) == 36, "the message budget at DR0");

        // A frequency in no sub-band the plan describes has no duty cycle to budget
        // against. That is a limit published elsewhere, not permission to transmit.
        Expect(plan.DutyCyclePermille(700_000_000) is null, "700 MHz is outside the band");
        // ANCHOR_END: example
    }
}
