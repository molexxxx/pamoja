// ANCHOR: example
using Pamoja.Hal;
using Pamoja.Modbus;

namespace Boards.RaspberryPi;

/// <summary>
/// A Modbus line scan: asks every unit address on an RS485 line, through a USB adapter, which
/// devices are there. Wire the adapter's A and B to every device's A and B, join the grounds,
/// and set the line's format below to the one the devices use.
/// </summary>
public static class ModbusScan
{
    // A USB RS485 adapter. The kernel names the first one it finds ttyUSB0, or ttyACM0 for an
    // adapter that presents itself as a modem.
    private const string Port = "/dev/ttyUSB0";

    /// <summary>Asks units 1 to 247 in turn and reports every one that answers.</summary>
    public static void Run()
    {
        // The format every device on the line uses, from their manuals: 19200 8E1 is the
        // default the specification sets, and many meters ship at 9600 8N1 instead.
        var settings = new SerialSettings(19_200, Parity.Even);
        using SerialPort port = SerialPort.Open(Port, settings);

        // A device that is there answers within a few milliseconds, so a short response timeout
        // keeps the scan of all 247 addresses under half a minute.
        using var client = new ModbusClient(port) { ResponseTimeout = TimeSpan.FromMilliseconds(100) };

        int found = 0;
        for (int unit = 1; unit <= 247; unit++)
        {
            // Holding register 0 is a question any device can answer, with its value or with
            // an exception, and either proves the device is there.
            try
            {
                ushort value = client.ReadHoldingRegisters((byte)unit, 0, 1)[0];
                Console.WriteLine($"unit {unit,3}  holding register 0 is {value}");
                found++;
            }
            catch (ModbusClientException error)
            {
                if (error.Kind == ModbusClientErrorKind.Exception)
                {
                    Console.WriteLine($"unit {unit,3}  there, and refused register 0 with exception 0x{(byte)error.ExceptionCode!:x2}");
                    found++;
                }
                else if (!(error.Kind == ModbusClientErrorKind.Timeout && error.Received == 0))
                {
                    Console.WriteLine($"unit {unit,3}  {error.Message}: check the format and the wiring");
                }
            }
        }

        Console.WriteLine($"{found} units answered on {Port} at {settings}");
    }
}
// ANCHOR_END: example
