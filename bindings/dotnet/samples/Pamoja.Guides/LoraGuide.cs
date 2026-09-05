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
        // EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the
        // setting that reaches furthest and holds the channel longest.
        using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        LoraLink link = plan.LinkSettings(0)!;
        Console.WriteLine($"{plan.Name} DR0 is SF{link.SpreadingFactor} at 125 kHz");

        // The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an
        // explicit header and CRC on, carrying a ten-byte reading.
        ulong airtime = link.AirtimeMicros(10);
        Console.WriteLine($"airtime   {airtime / 1e6:F2} s for ten bytes");

        // 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
        // transmission buys ninety-nine times its own length in silence.
        const uint Channel = 868_100_000;
        uint permille = plan.DutyCyclePermille(Channel)!.Value;
        Console.WriteLine(
            $"channel   {permille} per mille duty cycle, {plan.MaxEirpDbm(Channel)} dBm");

        ulong offTime = link.MinOffTimeMicros(10, permille)!.Value;
        Console.WriteLine($"silence   {offTime / 1e6:F1} s owed after each reading");

        // The airtime plus that silence is what one reading really costs, which is the
        // budget a deployment plans against.
        Console.WriteLine($"budget    {link.MessagesPerHour(10, permille)} readings an hour");

        // A frequency in no sub-band the plan describes has no duty cycle to budget
        // against. That is a limit published elsewhere, not permission to transmit.
        uint? outside = plan.DutyCyclePermille(700_000_000);
        Console.WriteLine($"700 MHz  is outside this plan, so it budgets nothing: {outside is null}");
        // ANCHOR_END: example

        Expect(plan.Name == "EU863-870", "the plan names its band");
        Expect(link.SpreadingFactor == 12, "DR0 is the slowest rate the band defines");
        Expect(airtime == 991_232, "the published time on air of a ten-byte frame");
        Expect(permille == 10, "the 868.1 MHz sub-band is limited to 1%");
        Expect(plan.MaxEirpDbm(Channel) == 16, "and to 16 dBm");
        Expect(offTime == airtime * 99, "so each frame owes ninety-nine times its length");
        Expect(link.MessagesPerHour(10, permille) == 36, "the message budget at DR0");
        Expect(outside is null, "a frequency outside the plan budgets nothing");
    }
}
