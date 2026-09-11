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

        // ANCHOR: range
        using LoraChannelPlan eu868 = LoraChannelPlan.ForRegion(LoraRegion.Eu868);
        LoraLink dr0 = eu868.LinkSettings(0)!;
        const uint Frequency = 868_100_000;

        // A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with
        // a 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
        var whip = new LoraLinkBudget { TransmitAntennaGainDbi = 2.15, TransmitCableLossDb = 0.5 };

        // The plan caps what leaves the antenna, so the antenna and cable decide how hard
        // the radio may drive. A radio takes whole decibels, so the setting rounds down.
        sbyte ceiling = eu868.MaxEirpDbm(Frequency);
        double most = whip.MaxTransmitPowerDbm(ceiling);
        LoraLinkBudget node = whip with
        {
            TransmitPowerDbm = Math.Floor(most),
            ReceiveAntennaGainDbi = 6,
            ReceiveCableLossDb = 1.5,
            NoiseFigureDb = LoraLinkBudget.GatewayNoiseFigureDb,
        };
        Console.WriteLine($"radio     {most:F2} dBm allowed, set to {node.TransmitPowerDbm} dBm");
        Console.WriteLine($"eirp      {node.EirpDbm:F2} dBm under a {ceiling} dBm ceiling");

        // The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most
        // path loss the link survives.
        double sensitivity = node.SensitivityDbm(dr0);
        double survives = node.MaxPathLossDb(dr0);
        Console.WriteLine(
            $"gateway   hears down to {sensitivity:F2} dBm, so {survives:F2} dB of path loss");

        // Free space at three distances, and what each path leaves to spare.
        var margins = new List<double>();
        foreach (uint distanceMeters in new uint[] { 2_000, 5_000, 15_000 })
        {
            double loss = LoraLinkBudget.FreeSpaceLossDb(distanceMeters, Frequency);
            double margin = node.MarginDb(dr0, loss);
            Console.WriteLine(
                $"{distanceMeters / 1_000,2} km     {loss:F2} dB lost, {margin:F2} dB to spare");
            margins.Add(margin);
        }

        // Free space assumes nothing is in the way. Terrain inside the first Fresnel zone
        // adds diffraction loss, which starts once the clearance falls below 60% of its
        // radius.
        uint radius = LoraLinkBudget.FresnelRadiusMillimeters(2_500, 2_500, Frequency);
        uint clear = radius * 6 / 10;
        Console.WriteLine(
            $"fresnel   {radius / 1000.0:F1} m at the middle of 5 km, keep {clear / 1000.0:F1} m clear");

        // In the United States, 47 CFR 15.247 caps conducted power instead, and takes off
        // every decibel an antenna has over 6 dBi.
        double limit = LoraLinkBudget.FccMaxConductedDbm(9, hoppingChannels: 64)!.Value;
        Console.WriteLine($"fcc       a 9 dBi Yagi on 64 hopping channels may carry {limit:F2} dBm");
        // ANCHOR_END: range

        Expect(most == 14.35, "the plan leaves 14.35 dBm for the radio behind the whip");
        Expect(node.TransmitPowerDbm == 14, "a radio setting rounds down to whole decibels");
        Expect(node.EirpDbm == 15.65, "which radiates 15.65 dBm");
        Expect(sensitivity == -140.03, "a gateway hears SF12 down to -140.03 dBm");
        Expect(survives == 160.18, "so the link survives 160.18 dB of path loss");
        Expect(
            margins.SequenceEqual(new[] { 62.94, 54.98, 45.44 }),
            "the margins free space leaves at 2, 5, and 15 km");
        Expect(radius == 20_777, "the Fresnel radius halfway along 5 km");
        Expect(limit == 27, "a 9 dBi antenna takes 3 dB off the 1 W limit");
    }
}
