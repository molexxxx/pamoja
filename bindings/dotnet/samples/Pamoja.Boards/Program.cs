// Runs one board program by name. Each one drives real pins or a radio, so CI builds them
// and a board runs them; each is spliced into a board page by `cargo xtask docs`.
using Boards.RaspberryPi;

var programs = new Dictionary<string, Func<Task>>(StringComparer.Ordinal)
{
    ["raspberry-pi/sensor"] = () => { Sensor.Run(); return Task.CompletedTask; },
    ["raspberry-pi/probes"] = () => { Probes.Run(); return Task.CompletedTask; },
    ["raspberry-pi/relay"] = () => { Relay.Run(); return Task.CompletedTask; },
    ["raspberry-pi/rig"] = () => { Rig.Run(); return Task.CompletedTask; },
    ["raspberry-pi/radio"] = () => { Radio.Run(); return Task.CompletedTask; },
};

if (args.Length != 1 || !programs.TryGetValue(args[0], out Func<Task>? program))
{
    Console.Error.WriteLine($"name one program: {string.Join(", ", programs.Keys)}");
    return 2;
}

try
{
    await program();
    return 0;
}
catch (Exception error) when (error is PlatformNotSupportedException or Pamoja.PamojaException)
{
    Console.Error.WriteLine(error.Message);
    return 1;
}
