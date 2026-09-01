using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// The P/Invoke declarations for the identity, codec, and helper-math capabilities
/// of the pamoja C ABI, mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the MQTT declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Both halves must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Returns a pointer to a byte buffer's contents.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_buffer_data(IntPtr buffer);

    /// <summary>Returns the length in bytes of a byte buffer.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_buffer_len(IntPtr buffer);

    /// <summary>Releases a byte buffer handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_buffer_free(IntPtr buffer);

    /// <summary>Creates a device identity from a 32-byte seed, or returns null.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_device_identity_new(ReadOnlySpan<byte> seed, nuint seedLen);

    /// <summary>Writes the 32-byte public key matching a device identity.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_device_identity_public_key(
        IntPtr identity,
        Span<byte> outPublicKey);

    /// <summary>Signs a payload, writing the 64-byte signature.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_device_identity_sign(
        IntPtr identity,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        Span<byte> outSignature);

    /// <summary>Releases a device identity handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_device_identity_free(IntPtr identity);

    /// <summary>Writes the 16-character hex fingerprint of a public key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_public_identity_fingerprint(
        ReadOnlySpan<byte> publicKey,
        Span<byte> outFingerprint);

    /// <summary>Verifies that a signature covers a payload for a public key.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_public_identity_verify(
        ReadOnlySpan<byte> publicKey,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        ReadOnlySpan<byte> signature);

    /// <summary>Converts a JSON document into its CBOR encoding.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_json_to_cbor(
        ReadOnlySpan<byte> json,
        nuint jsonLen,
        out IntPtr outBuffer);

    /// <summary>Converts a CBOR document into its JSON encoding.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_cbor_to_json(
        ReadOnlySpan<byte> cbor,
        nuint cborLen,
        out IntPtr outBuffer);

    /// <summary>Delta-encodes a series of integer samples.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_encode_deltas(
        ReadOnlySpan<long> samples,
        nuint count,
        out IntPtr outBuffer);

    /// <summary>Decodes a delta-encoded buffer back into integer samples.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_decode_deltas(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outSamples);

    /// <summary>Quantizes and delta-encodes a batch of float readings.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_quantizer_encode(
        float scale,
        ReadOnlySpan<float> readings,
        nuint count,
        out IntPtr outBuffer);

    /// <summary>Decodes a quantized batch back into float readings.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_codec_quantizer_decode(
        float scale,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        out IntPtr outReadings);

    /// <summary>Returns a pointer to a decoded integer sample series.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_samples_data(IntPtr samples);

    /// <summary>Returns the number of integer samples in a decoded series.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_samples_len(IntPtr samples);

    /// <summary>Releases a decoded sample series. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_samples_free(IntPtr samples);

    /// <summary>Returns a pointer to a decoded float reading series.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_readings_data(IntPtr readings);

    /// <summary>Returns the number of float readings in a decoded series.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_readings_len(IntPtr readings);

    /// <summary>Releases a decoded reading series. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_readings_free(IntPtr readings);

    /// <summary>Creates an exponential smoother.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_smoother_new(float weight);

    /// <summary>Folds a sample into a smoother and returns the smoothed value.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_smoother_update(IntPtr smoother, float sample);

    /// <summary>Reads a smoother's current value, if it has one.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_smoother_value(IntPtr smoother, out float outValue);

    /// <summary>Clears a smoother back to its initial state.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_smoother_reset(IntPtr smoother);

    /// <summary>Releases a smoother handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_smoother_free(IntPtr smoother);

    /// <summary>Creates a PID controller with no output limits.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_pid_new(float kp, float ki, float kd);

    /// <summary>Creates a PID controller clamped to the given output range.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_pid_new_with_limits(
        float kp,
        float ki,
        float kd,
        float min,
        float max);

    /// <summary>Advances a PID controller by one step.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_pid_update(
        IntPtr pid,
        float setpoint,
        float measurement,
        float dt);

    /// <summary>Clears a PID controller's integral and last error.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_pid_reset(IntPtr pid);

    /// <summary>Releases a PID handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_pid_free(IntPtr pid);

    /// <summary>Creates a cooling thermostat.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_thermostat_cooling(float setpoint, float hysteresis);

    /// <summary>Creates a heating thermostat.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_thermostat_heating(float setpoint, float hysteresis);

    /// <summary>Feeds a reading to a thermostat and returns the load state.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_thermostat_update(IntPtr thermostat, float reading);

    /// <summary>Reports a thermostat's current load state.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_thermostat_is_on(IntPtr thermostat);

    /// <summary>Releases a thermostat handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_thermostat_free(IntPtr thermostat);

    /// <summary>Creates a depletion estimator.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_depletion_new(float threshold);

    /// <summary>Records a level and estimates the samples until the threshold.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_depletion_update(
        IntPtr depletion,
        float level,
        out uint outSamples);

    /// <summary>Releases a depletion handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_depletion_free(IntPtr depletion);

    /// <summary>Creates a one-dimensional Kalman filter.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_kalman_new(
        float processNoise,
        float measurementNoise,
        float initial);

    /// <summary>Folds a reading into a Kalman filter and returns the estimate.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_kalman_update(IntPtr kalman, float reading);

    /// <summary>Reads a Kalman filter's current estimate.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_kalman_estimate(IntPtr kalman);

    /// <summary>Releases a Kalman handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_kalman_free(IntPtr kalman);

    /// <summary>Creates a boolean debouncer.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_debounce_new(
        ushort samples,
        [MarshalAs(UnmanagedType.U1)] bool initial);

    /// <summary>Feeds a raw reading to a debouncer and returns the settled state.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_debounce_update(
        IntPtr debounce,
        [MarshalAs(UnmanagedType.U1)] bool raw);

    /// <summary>Reports a debouncer's settled state.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_debounce_state(IntPtr debounce);

    /// <summary>Releases a debouncer handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_debounce_free(IntPtr debounce);

    /// <summary>Creates a rate limiter.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ramp_new(float start, float maxStep);

    /// <summary>Moves a ramp one step toward a target.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ramp_update(IntPtr ramp, float target);

    /// <summary>Reads a ramp's current value.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_ramp_value(IntPtr ramp);

    /// <summary>Forces a ramp to a value without rate limiting.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ramp_set(IntPtr ramp, float value);

    /// <summary>Releases a ramp handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_ramp_free(IntPtr ramp);

    /// <summary>Creates a detector for rises of at least the given size.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_surge_rising(float limit);

    /// <summary>Creates a detector for falls of at least the given size.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_surge_falling(float limit);

    /// <summary>Feeds a value to a surge detector, reporting a qualifying step.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_surge_update(IntPtr surge, float value, out float outDelta);

    /// <summary>Releases a surge handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_surge_free(IntPtr surge);

    /// <summary>Creates a linear calibration.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_calibration_linear(float scale, float offset);

    /// <summary>Creates a calibration fitted through two reference points.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_calibration_two_point(
        float rawLow,
        float valueLow,
        float rawHigh,
        float valueHigh);

    /// <summary>Converts a raw reading into calibrated units.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_calibration_apply(IntPtr calibration, float raw);

    /// <summary>Releases a calibration handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_calibration_free(IntPtr calibration);

    /// <summary>Creates a circular geofence.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_geofence_new(PamojaCoordinate center, double radiusM);

    /// <summary>Feeds a fix to a geofence and reports where it sits.</summary>
    [LibraryImport(Library)]
    public static partial PamojaBoundary pamoja_geofence_update(
        IntPtr geofence,
        PamojaCoordinate point);

    /// <summary>Reports whether a fix lies inside a geofence.</summary>
    [LibraryImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_geofence_contains(IntPtr geofence, PamojaCoordinate point);

    /// <summary>Releases a geofence handle. Passing null is a no-op.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_geofence_free(IntPtr geofence);

    /// <summary>Returns the great-circle distance between two coordinates, in metres.</summary>
    [LibraryImport(Library)]
    public static partial double pamoja_coordinate_distance_to(
        PamojaCoordinate from,
        PamojaCoordinate to);

    /// <summary>Returns the initial bearing between two coordinates, in degrees.</summary>
    [LibraryImport(Library)]
    public static partial double pamoja_coordinate_bearing_to(
        PamojaCoordinate from,
        PamojaCoordinate to);

    /// <summary>Suppresses movement within a band around a centre value.</summary>
    [LibraryImport(Library)]
    public static partial float pamoja_kit_deadband(float value, float center, float width);
}
