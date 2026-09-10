// Runs every guide example. Each one is spliced into a page of the documentation
// site by `cargo xtask docs`, so every C# example the site shows is code that ran.
using Guides;

// An argument names one guide to run; without one every guide runs, which is what CI does.
var guides = new Dictionary<string, Func<Task>>(StringComparer.Ordinal)
{
    ["quickstart"] = async () => await Quickstart.RunAsync(),
    ["security"] = () => { SecurityGuide.Run(); return Task.CompletedTask; },
    ["codec"] = () => { CodecGuide.Run(); return Task.CompletedTask; },
    ["kit"] = () => { KitGuide.Run(); return Task.CompletedTask; },
    ["serial"] = () => { SerialGuide.Run(); return Task.CompletedTask; },
    ["modbus"] = () => { ModbusGuide.Run(); return Task.CompletedTask; },
    ["can"] = () => { CanGuide.Run(); return Task.CompletedTask; },
    ["gpio"] = () => { GpioGuide.Run(); return Task.CompletedTask; },
    ["hal"] = async () => await HalGuide.RunAsync(),
    ["sensors"] = () => { SensorsGuide.Run(); return Task.CompletedTask; },
    ["actuators"] = () => { ActuatorsGuide.Run(); return Task.CompletedTask; },
    ["device"] = async () => await DeviceGuide.RunAsync(),
    ["lora"] = () => { LoraGuide.Run(); return Task.CompletedTask; },
    ["lorawan"] = () => { LorawanGuide.Run(); return Task.CompletedTask; },
    ["mesh"] = () => { MeshGuide.Run(); return Task.CompletedTask; },
    ["routing"] = () => { RoutingGuide.Run(); return Task.CompletedTask; },
    ["mavlink"] = () => { MavlinkGuide.Run(); return Task.CompletedTask; },
    ["audit"] = () => { AuditGuide.Run(); return Task.CompletedTask; },
    ["session"] = () => { SessionGuide.Run(); return Task.CompletedTask; },
    ["update"] = () => { UpdateGuide.Run(); return Task.CompletedTask; },
    ["power"] = () => { PowerGuide.Run(); return Task.CompletedTask; },
    ["telemetry"] = () => { TelemetryGuide.Run(); return Task.CompletedTask; },
    ["mqtt"] = async () => await MqttGuide.RunAsync(),
    ["coap"] = async () => await CoapGuide.RunAsync(),
    ["loopback"] = async () => await LoopbackGuide.RunAsync(),
    ["sync"] = async () => await SyncGuide.RunAsync(),
    ["ladder"] = async () => await LadderGuide.RunAsync(),
    ["bus"] = async () => await BusGuide.RunAsync(),
    ["transport"] = async () => await TransportGuide.RunAsync(),
    ["link"] = async () => await LinkGuide.RunAsync(),
    ["sim"] = async () => await SimGuide.RunAsync(),
    ["profile"] = () => { ProfileGuide.Run(); return Task.CompletedTask; },
    ["rules"] = async () => await RulesGuide.RunAsync(),
    ["ros2"] = () => { Ros2Guide.Run(); return Task.CompletedTask; },
    ["zenoh"] = () => { ZenohGuide.Run(); return Task.CompletedTask; },
};

var only = args.Length > 0 ? args[0] : null;
if (only is not null && !guides.ContainsKey(only))
{
    Console.Error.WriteLine($"no guide named {only}; try one of: {string.Join(", ", guides.Keys)}");
    return 1;
}

foreach (var (name, run) in guides)
{
    if (only is not null && name != only)
    {
        continue;
    }

    await run();
}

Console.WriteLine("guides ok");
return 0;
