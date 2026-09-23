// ANCHOR: example
using Pamoja.Gpio;
using Pamoja.Kit;

namespace Boards.RaspberryPi;

/// <summary>
/// A relay board on GPIO17 that follows a limit switch on GPIO27, debounced. Wire the relay
/// board's IN to GPIO17 and its VCC and GND to the header's 5V and ground, and the switch
/// between GPIO27 and ground with <c>gpio=27=ip,pu</c> in config.txt.
/// </summary>
public static class Relay
{
    // The GPIO chip the header's lines live on. Every current model exposes them here, and
    // the line numbers below are the BCM numbers the documentation and the kernel both use.
    private const string Chip = "/dev/gpiochip0";
    private const uint RelayLine = 17;
    private const uint SwitchLine = 27;

    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // Taking the line as an output also says what to drive the moment it is taken.
        // Until then every GPIO is an input, so a relay board sees whatever its own pull
        // gives it; driving the resting level immediately is what keeps a vent from opening
        // at boot. Most relay boards energize on a low input, which is what `ActiveLow`
        // says once so that nothing below this line has to think about the inversion again.
        using GpioLine relayLine = GpioLine.OpenOutput(Chip, RelayLine, PinLevel.High);
        var relay = Switch.ActiveLow(relayLine);

        // The switch is wired to pull the line down when it closes, so it is active low too.
        using GpioLine switchLine = GpioLine.OpenInput(Chip, SwitchLine);
        var limit = Contact.ActiveLow(switchLine);

        // A mechanical contact bounces for a few milliseconds as it closes. Sampling every
        // 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
        // before it counts, which is longer than the bounce and shorter than a person.
        using var settled = new Debounce(3, false);
        bool wasClosed = false;

        Console.WriteLine($"watching GPIO{SwitchLine}, driving GPIO{RelayLine}; Ctrl-C to stop");
        while (true)
        {
            bool closed = settled.Update(limit.IsAsserted());
            if (closed != wasClosed)
            {
                Console.WriteLine($"the limit switch {(closed ? "closed" : "opened")}");

                // The relay follows the switch. A real vent would run its motor until the
                // limit closes and then stop; this is the same two calls either way.
                relay.Set(!closed);
                wasClosed = closed;
            }

            Thread.Sleep(20);
        }
    }
}
// ANCHOR_END: example
