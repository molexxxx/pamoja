using Pamoja.Hal;
using Pamoja.Modbus;

using static Guides.Guide;
using static System.FormattableString;

namespace Guides;

/// <summary>
/// The Modbus RTU guide example: a gateway at a village water pump polls an energy meter and a
/// relay module on one RS485 line; see docs/guides/modbus.md.
/// </summary>
public static class ModbusGuide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
        static string Word(bool on) => on ? "on" : "off";

        // The line: 19200 baud, even parity, one stop bit, the default the Modbus
        // specification sets. Eleven bits a character, and 3.5 of them of silence mark where
        // a frame ends.
        var settings = new SerialSettings(19_200, Parity.Even);
        ulong gap = ModbusClient.FrameGapNanos(settings) / 1_000;
        Console.WriteLine($"line         {settings}, {settings.BitsPerCharacter} bits a character, t3.5 is {gap} us");

        // Each device's manual gives its unit address and where its values live. The meter
        // keeps its measurements in input registers from 0: volts in tenths, amps in
        // hundredths, then a fault word. The relay module has four relays as coils 0 to 3, and
        // the tank's low-level float switch as discrete input 0, on while the water is below it.
        const byte Meter = 17;
        const byte Pump = 18;
        using var meter = new ModbusServer(Meter);
        meter.SetInputRegisters(0, [2301, 418, 0]);
        using var relays = new ModbusServer(Pump);
        relays.SetCoils(0, new bool[4]);
        relays.SetDiscreteInputs(0, [true]);
        using var line = new ModbusLine().Attach(meter).Attach(relays);

        // The devices sit on a simulated line. On a gateway the port is
        // SerialPort.Open("/dev/ttyUSB0", settings), and nothing after this statement changes.
        using SerialPort port = line.Port(settings);
        using var client = new ModbusClient(port);

        // Poll the meter with function 0x04 for three input registers, and scale each one as
        // its manual says.
        ushort[] registers = client.ReadInputRegisters(Meter, 0, 3);
        Console.WriteLine(Invariant(
            $"meter        {registers[0] / 10.0:F1} V, {registers[1] / 100.0:F2} A, faults {registers[2]}"));

        // What that poll cost the line: the request, the reply, and the silence before the
        // request.
        long bytesOut = port.Written;
        long bytesBack = port.Received;
        ulong lineTime = settings.TransferMicros((int)bytesOut) + settings.TransferMicros((int)bytesBack) + gap;
        Console.WriteLine(Invariant(
            $"poll         {bytesOut} bytes out, {bytesBack} back, {lineTime / 1_000.0:F2} ms of line time"));

        // Read the float switch, and start the pump on relay 0 when the tank is low.
        bool low = client.ReadDiscreteInputs(Pump, 0, 1)[0];
        Console.WriteLine($"tank         low-level switch {Word(low)}");
        if (low)
        {
            client.WriteSingleCoil(Pump, 0, true);
        }

        bool[] states = client.ReadCoils(Pump, 0, 4);
        Console.WriteLine($"relays       {string.Join(' ', states.Select(Word))}");

        // A broadcast, to unit 0, reaches every device on the line and none answers: here
        // every relay off at once. The client waits out the turnaround so each device has
        // carried it out before the next request.
        client.WriteMultipleCoils(ModbusClient.Broadcast, 0, new bool[4]);
        Console.WriteLine($"broadcast    every relay off, no reply, {client.Turnaround.TotalMilliseconds} ms turnaround");
        bool[] after = client.ReadCoils(Pump, 0, 4);
        Console.WriteLine($"relays       {string.Join(' ', after.Select(Word))}");

        // The meter keeps its measurements in input registers. Asking for them as holding
        // registers, function 0x03, is the usual mistake with a new device, and the meter
        // refuses it with an exception instead of answering.
        ModbusClientException? refused = null;
        try
        {
            client.ReadHoldingRegisters(Meter, 0, 3);
        }
        catch (ModbusClientException error)
        {
            refused = error;
            Console.WriteLine($"refused      {error.Message}");
        }

        // A unit that is not on the line never answers. The client gives up after its response
        // timeout, one second unless told otherwise, which a simulated line counts instead of
        // sleeping through.
        ulong before = port.WaitedMicros;
        ModbusClientException? silent = null;
        try
        {
            client.ReadInputRegisters(19, 0, 1);
        }
        catch (ModbusClientException error)
        {
            silent = error;
            Console.WriteLine($"silent       {error.Message}, {(port.WaitedMicros - before) / 1_000} ms counted and not slept");
        }
        // ANCHOR_END: example

        Expect(registers.SequenceEqual(new ushort[] { 2301, 418, 0 }), "the meter's three registers");
        Expect(bytesOut == 8 && bytesBack == 11, "an eight-byte request and an eleven-byte reply");
        Expect(low, "the tank is low");
        Expect(states.SequenceEqual(new[] { true, false, false, false }), "the pump relay on");
        Expect(relays.Coil(0) == false, "the broadcast turned it off");
        Expect(refused?.Kind == ModbusClientErrorKind.Exception, "the meter refused the wrong function");
        Expect(silent?.Kind == ModbusClientErrorKind.Timeout && silent.Unit == 19, "unit 19 never answered");
    }
}
