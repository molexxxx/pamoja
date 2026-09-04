namespace Pamoja.Native.Interop;

/// <summary>
/// Which direction an I2C transfer runs, mirroring <c>PamojaI2cDirection</c> in
/// <c>pamoja.h</c>.
/// </summary>
public enum PamojaI2cDirection
{
    /// <summary>The controller writes to the device. Read/write bit 0.</summary>
    Write = 0,

    /// <summary>The controller reads from the device. Read/write bit 1.</summary>
    Read = 1,
}
