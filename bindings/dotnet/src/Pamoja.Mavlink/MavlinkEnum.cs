using Pamoja.Native.Interop;

namespace Pamoja.Mavlink;

/// <summary>One named value of a dialect enumeration.</summary>
/// <param name="Name">The name the dialect gives the value, such as <c>MAV_STATE_STANDBY</c>.</param>
/// <param name="Value">The value on the wire.</param>
public readonly record struct MavlinkEnumEntry(string Name, ulong Value);

/// <summary>The MAVLink dialect's named values.</summary>
/// <remarks>
/// An enum field rides on the wire as a plain integer, so setting <c>type</c> on a heartbeat
/// or reading <c>result</c> off an acknowledgment works in numbers. These calls turn the names
/// the dialect writes, such as <c>MAV_TYPE_QUADROTOR</c>, into those numbers and back, for
/// every enumeration a field of a typed message names.
/// </remarks>
public static class MavlinkEnum
{
    /// <summary>Returns the value a dialect entry name stands for.</summary>
    /// <remarks>
    /// MAVLink writes each value's name with its enumeration in front, and the names are
    /// unique across the dialect, so the name alone is enough.
    /// </remarks>
    /// <param name="entry">The entry's name, such as <c>MAV_CMD_COMPONENT_ARM_DISARM</c>.</param>
    /// <returns>The value, ready to set on a field.</returns>
    /// <exception cref="PamojaException">No entry of any dialect enumeration has that name.</exception>
    public static ulong Value(string entry)
    {
        Status.ThrowIfError(NativeMethods.pamoja_mavlink_enum_value(entry, out ulong value));
        return value;
    }

    /// <summary>Names the entry of an enumeration that stands for a value.</summary>
    /// <param name="enumeration">The enumeration's name, such as <c>MAV_STATE</c>.</param>
    /// <param name="value">The value a field carried.</param>
    /// <returns>The entry's name, or <c>null</c> if no entry names the value.</returns>
    /// <exception cref="PamojaException">The enumeration is unknown.</exception>
    public static string? Entry(string enumeration, ulong value)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_enum_entry(enumeration, value, out IntPtr name));
        return OwnedString.ReadOrNull(name);
    }

    /// <summary>Names the entries a value is made of.</summary>
    /// <param name="enumeration">The enumeration's name, such as <c>MAV_MODE_FLAG</c>.</param>
    /// <param name="value">The value a field carried.</param>
    /// <returns>
    /// For a bitmask, each entry whose bits are set in the value, in dialect order; otherwise
    /// the one entry that names it. Empty when none applies.
    /// </returns>
    /// <exception cref="PamojaException">The enumeration is unknown.</exception>
    public static IReadOnlyList<string> Names(string enumeration, ulong value)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_enum_names(enumeration, value, out IntPtr names));
        string joined = OwnedString.Read(names);
        return joined.Length == 0 ? [] : joined.Split('|');
    }

    /// <summary>Returns every entry of an enumeration, in dialect order.</summary>
    /// <param name="enumeration">The enumeration's name.</param>
    /// <returns>Each entry's name and value.</returns>
    /// <exception cref="PamojaException">The enumeration is unknown.</exception>
    public static IReadOnlyList<MavlinkEnumEntry> Entries(string enumeration)
    {
        Status.ThrowIfError(NativeMethods.pamoja_mavlink_enum_len(enumeration, out nuint count));
        List<MavlinkEnumEntry> entries = new((int)count);
        for (nuint index = 0; index < count; index += 1)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_mavlink_enum_entry_at(
                    enumeration,
                    index,
                    out IntPtr name,
                    out ulong value));
            entries.Add(new MavlinkEnumEntry(OwnedString.Read(name), value));
        }
        return entries;
    }

    /// <summary>Reports whether an enumeration's values combine as bits.</summary>
    /// <param name="enumeration">The enumeration's name.</param>
    /// <returns><c>true</c> for a bitmask such as <c>MAV_MODE_FLAG</c>.</returns>
    /// <exception cref="PamojaException">The enumeration is unknown.</exception>
    public static bool IsBitmask(string enumeration)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_enum_is_bitmask(enumeration, out bool bitmask));
        return bitmask;
    }

    /// <summary>Returns the names of every enumeration this build names values for.</summary>
    /// <returns>
    /// Each enumeration a field of a typed message names, in the order the dialect defines
    /// them.
    /// </returns>
    public static IReadOnlyList<string> Known()
    {
        int count = (int)NativeMethods.pamoja_mavlink_enum_count();
        List<string> names = new(count);
        for (int index = 0; index < count; index += 1)
        {
            Status.ThrowIfError(NativeMethods.pamoja_mavlink_enum_at((nuint)index, out IntPtr name));
            names.Add(OwnedString.Read(name));
        }
        return names;
    }
}
