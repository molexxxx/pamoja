// Runs every guide example. Each one is spliced into a page of the documentation
// site by `cargo xtask docs`, so every C# example the site shows is code that ran.
using Guides;

await Quickstart.RunAsync();
SecurityGuide.Run();
CodecGuide.Run();
KitGuide.Run();
SerialGuide.Run();
ModbusGuide.Run();
CanGuide.Run();
GpioGuide.Run();
SensorsGuide.Run();
ActuatorsGuide.Run();
LoraGuide.Run();
LorawanGuide.Run();
MeshGuide.Run();
RoutingGuide.Run();
MavlinkGuide.Run();
AuditGuide.Run();
SessionGuide.Run();
UpdateGuide.Run();
PowerGuide.Run();
TelemetryGuide.Run();
await MqttGuide.RunAsync();
await CoapGuide.RunAsync();
await LoopbackGuide.RunAsync();
await SyncGuide.RunAsync();
await LadderGuide.RunAsync();
await BusGuide.RunAsync();
await TransportGuide.RunAsync();
await SimGuide.RunAsync();
ProfileGuide.Run();
Ros2Guide.Run();
ZenohGuide.Run();

Console.WriteLine("guides ok");
