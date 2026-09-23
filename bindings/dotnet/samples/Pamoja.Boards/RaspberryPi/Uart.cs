// ANCHOR: example
using System.Buffers.Binary;
using System.Diagnostics;
using System.Globalization;

using Pamoja.Hal;
using Pamoja.Serial;

namespace Boards.RaspberryPi;

/// <summary>
/// A UART self-test: the Pi's own serial port with TX jumpered to RX, sending COBS frames and
/// reading each one straight back. Turn the serial port hardware on and the serial console off
/// (raspi-config, Interface Options, Serial Port), reboot, and put one jumper between GPIO14 and
/// GPIO15.
/// </summary>
public static class Uart
{
    // The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
    private const string Port = "/dev/serial0";

    /// <summary>Sends five frames and reports each one's return.</summary>
    public static void Run()
    {
        using SerialPort port = SerialPort.Open(Port, new SerialSettings(115_200));
        using var decoder = new CobsDecoder();

        for (ushort sequence = 1; sequence <= 5; sequence++)
        {
            byte[] payload = new byte[2 + "ping"u8.Length];
            BinaryPrimitives.WriteUInt16BigEndian(payload, sequence);
            "ping"u8.CopyTo(payload.AsSpan(2));
            byte[] frame = Serial.CobsEncode(payload);
            var started = Stopwatch.StartNew();
            port.Write(frame);

            // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
            // until the line has been quiet for 100 ms.
            byte[]? echoed = null;
            while (echoed is null)
            {
                byte[] arrived = port.Read(64, TimeSpan.FromMilliseconds(100));
                if (arrived.Length == 0)
                {
                    break;
                }

                echoed = decoder.Feed(arrived).FirstOrDefault();
            }

            if (echoed is null)
            {
                Console.WriteLine($"frame {sequence}  nothing came back: check the jumper, and that the console is off");
            }
            else if (echoed.SequenceEqual(payload))
            {
                Console.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"frame {sequence}  {frame.Length} bytes back in {started.Elapsed.TotalMilliseconds:F2} ms"));
            }
            else
            {
                Console.WriteLine($"frame {sequence}  came back changed: check the speed and the wiring");
            }

            Thread.Sleep(TimeSpan.FromMilliseconds(500));
        }
    }
}
// ANCHOR_END: example
