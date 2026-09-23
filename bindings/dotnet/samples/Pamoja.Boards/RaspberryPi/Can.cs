// ANCHOR: example
using System.Diagnostics;
using System.Globalization;

using Pamoja.Can;

namespace Boards.RaspberryPi;

/// <summary>
/// A CAN bus monitor: listens on the Pi's CAN interface, through an MCP2515, and names each J1939
/// message it hears. Load the controller's overlay and bring the interface up at the bus's bit
/// rate first.
/// </summary>
public static class CanMonitor
{
    // The interface the MCP2515 overlay makes.
    private const string Interface = "can0";

    // Engine speed's parameter group, where the speed sits in it, and its scale.
    private const uint EngineController1 = 61_444;
    private const int EngineSpeedAt = 3;
    private const double RpmPerBit = 0.125;

    /// <summary>Listens for ten seconds and names each frame.</summary>
    public static void Run()
    {
        // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
        using CanBus bus = CanBus.Open(Interface);
        var started = Stopwatch.StartNew();
        while (started.Elapsed < TimeSpan.FromSeconds(10))
        {
            CanFrame? frame = bus.Receive(TimeSpan.FromSeconds(1));
            if (frame is null)
            {
                Console.WriteLine("quiet for a second: check the bit rate, the wiring, and the termination");
                continue;
            }

            string at = started.Elapsed.TotalSeconds.ToString("F3", CultureInfo.InvariantCulture).PadLeft(7);
            J1939Message? message = Can.DecodeJ1939(frame.Id, frame.Extended);
            if (message is { Pgn: EngineController1 })
            {
                double rpm = (Signals.From(frame.Data).U16(EngineSpeedAt) ?? 0) * RpmPerBit;
                Console.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"{at} s  pgn {message.Pgn} from {message.Source}: {rpm:F1} rpm"));
            }
            else if (message is not null)
            {
                Console.WriteLine($"{at} s  pgn {message.Pgn} from {message.Source}, {frame.Length} bytes");
            }
            else
            {
                Console.WriteLine($"{at} s  0x{frame.Id:X3}, {frame.Length} bytes, an 11-bit identifier");
            }
        }

        Console.WriteLine($"{bus.Received} frames in ten seconds on {Interface}");
    }
}
// ANCHOR_END: example
