namespace Pamoja.Native.Interop;

/// <summary>
/// The limit comparison an INA226 alert pin responds to. Mirrors
/// &lt;c&gt;PamojaIna226AlertFunction&lt;/c&gt; in &lt;c&gt;pamoja.h&lt;/c&gt;.
/// </summary>
public enum PamojaIna226AlertFunction
{
    /// <summary>Shunt voltage above the alert limit.</summary>
    ShuntOverLimit = 0,

    /// <summary>Shunt voltage below the alert limit.</summary>
    ShuntUnderLimit = 1,

    /// <summary>Bus voltage above the alert limit.</summary>
    BusOverLimit = 2,

    /// <summary>Bus voltage below the alert limit.</summary>
    BusUnderLimit = 3,

    /// <summary>Power above the alert limit.</summary>
    PowerOverLimit = 4,
}
