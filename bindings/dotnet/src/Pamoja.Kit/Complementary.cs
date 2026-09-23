using Pamoja.Native.Interop;

namespace Pamoja.Kit;

/// <summary>Fuses a drifting rate, such as a gyroscope's, with a noisy absolute reading, such as an accelerometer's tilt, into one steady estimate.</summary>
/// <remarks>
/// Each update integrates the rate onto the estimate, then nudges that toward the absolute
/// reading by <c>1 - alpha</c>, so the rate rules the short term and the absolute reading the
/// long term. An update with a rate, reading, or time step that is not a finite number is
/// ignored.
/// </remarks>
public sealed class Complementary : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a filter.</summary>
    /// <param name="alpha">
    /// The weight on the integrated rate, held to 0 to 1: near 1 trusts the rate and corrects
    /// slowly. One that is not a number is taken as 0.
    /// </param>
    /// <param name="initial">The starting estimate.</param>
    public Complementary(float alpha, float initial)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_complementary_new(alpha, initial),
            NativeMethods.pamoja_complementary_free,
            "complementary filter",
            serialized: true);
    }

    /// <summary>Gets the current estimate.</summary>
    public float Estimate => _handle.Use(NativeMethods.pamoja_complementary_estimate);

    /// <summary>Fuses a rate and an absolute reading over a time step.</summary>
    /// <param name="rate">The rate of change, such as degrees per second from a gyroscope.</param>
    /// <param name="absolute">The absolute reading, such as a tilt from an accelerometer.</param>
    /// <param name="dt">The time since the last update.</param>
    /// <returns>The fused estimate.</returns>
    public float Update(float rate, float absolute, float dt) =>
        _handle.Use(handle => NativeMethods.pamoja_complementary_update(handle, rate, absolute, dt));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Roll and pitch, in degrees.</summary>
/// <param name="Roll">Rotation about the forward axis, in degrees, from -180 to 180.</param>
/// <param name="Pitch">Rotation about the right axis, in degrees, from -90 to 90.</param>
public readonly record struct Tilt(double Roll, double Pitch);

/// <summary>Converts readings between the units a sensor reports and the ones a person reads.</summary>
public static class Units
{
    /// <summary>Converts degrees Celsius to degrees Fahrenheit.</summary>
    /// <param name="celsius">A temperature in degrees Celsius.</param>
    /// <returns>The temperature in degrees Fahrenheit.</returns>
    public static float CelsiusToFahrenheit(float celsius) => NativeMethods.pamoja_units_celsius_to_fahrenheit(celsius);

    /// <summary>Converts degrees Fahrenheit to degrees Celsius.</summary>
    /// <param name="fahrenheit">A temperature in degrees Fahrenheit.</param>
    /// <returns>The temperature in degrees Celsius.</returns>
    public static float FahrenheitToCelsius(float fahrenheit) => NativeMethods.pamoja_units_fahrenheit_to_celsius(fahrenheit);

    /// <summary>Converts degrees Celsius to kelvin.</summary>
    /// <param name="celsius">A temperature in degrees Celsius.</param>
    /// <returns>The temperature in kelvin.</returns>
    public static float CelsiusToKelvin(float celsius) => NativeMethods.pamoja_units_celsius_to_kelvin(celsius);

    /// <summary>Converts kelvin to degrees Celsius.</summary>
    /// <param name="kelvin">A temperature in kelvin.</param>
    /// <returns>The temperature in degrees Celsius.</returns>
    public static float KelvinToCelsius(float kelvin) => NativeMethods.pamoja_units_kelvin_to_celsius(kelvin);

    /// <summary>Converts pascals to hectopascals.</summary>
    /// <param name="pascals">A pressure in pascals.</param>
    /// <returns>The pressure in hectopascals.</returns>
    public static float PascalsToHectopascals(float pascals) => NativeMethods.pamoja_units_pascals_to_hectopascals(pascals);

    /// <summary>Converts hectopascals to pascals.</summary>
    /// <param name="hectopascals">A pressure in hectopascals.</param>
    /// <returns>The pressure in pascals.</returns>
    public static float HectopascalsToPascals(float hectopascals) => NativeMethods.pamoja_units_hectopascals_to_pascals(hectopascals);

    /// <summary>Converts pascals to kilopascals.</summary>
    /// <param name="pascals">A pressure in pascals.</param>
    /// <returns>The pressure in kilopascals.</returns>
    public static float PascalsToKilopascals(float pascals) => NativeMethods.pamoja_units_pascals_to_kilopascals(pascals);

    /// <summary>Converts kilopascals to pascals.</summary>
    /// <param name="kilopascals">A pressure in kilopascals.</param>
    /// <returns>The pressure in pascals.</returns>
    public static float KilopascalsToPascals(float kilopascals) => NativeMethods.pamoja_units_kilopascals_to_pascals(kilopascals);

    /// <summary>Converts pascals to pounds per square inch.</summary>
    /// <param name="pascals">A pressure in pascals.</param>
    /// <returns>The pressure in pounds per square inch.</returns>
    public static float PascalsToPsi(float pascals) => NativeMethods.pamoja_units_pascals_to_psi(pascals);

    /// <summary>Converts pounds per square inch to pascals.</summary>
    /// <param name="psi">A pressure in pounds per square inch.</param>
    /// <returns>The pressure in pascals.</returns>
    public static float PsiToPascals(float psi) => NativeMethods.pamoja_units_psi_to_pascals(psi);

    /// <summary>Converts a ratio from 0 to 1 to a percentage.</summary>
    /// <param name="ratio">The ratio.</param>
    /// <returns>The percentage.</returns>
    public static float RatioToPercent(float ratio) => NativeMethods.pamoja_units_ratio_to_percent(ratio);

    /// <summary>Converts a percentage to a ratio from 0 to 1.</summary>
    /// <param name="percent">The percentage.</param>
    /// <returns>The ratio.</returns>
    public static float PercentToRatio(float percent) => NativeMethods.pamoja_units_percent_to_ratio(percent);
}
