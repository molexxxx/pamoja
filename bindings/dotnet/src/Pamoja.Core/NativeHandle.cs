using System.Runtime.InteropServices;

namespace Pamoja.Core;

/// <summary>
/// A <see cref="SafeHandle"/> over a native pointer whose release function is
/// supplied at construction, so the handle is always released exactly once even
/// across finalization races.
/// </summary>
/// <remarks>
/// The helper-math capabilities expose a family of handles that differ only in
/// which <c>*_free</c> function releases them, so one handle type carrying that
/// function serves them all.
/// </remarks>
internal sealed class NativeHandle : SafeHandle
{
    private readonly Action<IntPtr> _release;

    /// <summary>Wraps a non-null native pointer with the function that frees it.</summary>
    /// <param name="handle">The pointer returned by a native constructor.</param>
    /// <param name="release">The matching native release function.</param>
    public NativeHandle(IntPtr handle, Action<IntPtr> release)
        : base(IntPtr.Zero, ownsHandle: true)
    {
        _release = release;
        SetHandle(handle);
    }

    /// <inheritdoc/>
    public override bool IsInvalid => handle == IntPtr.Zero;

    /// <summary>Creates a wrapper, throwing when the native constructor returned null.</summary>
    /// <param name="handle">The pointer returned by a native constructor.</param>
    /// <param name="release">The matching native release function.</param>
    /// <param name="what">What was being created, for the exception message.</param>
    /// <returns>The wrapped handle.</returns>
    /// <exception cref="PamojaException">The native constructor returned null.</exception>
    public static NativeHandle Create(IntPtr handle, Action<IntPtr> release, string what)
    {
        if (handle == IntPtr.Zero)
        {
            throw new PamojaException(PamojaCore.LastError() ?? $"failed to create the {what}");
        }

        return new NativeHandle(handle, release);
    }

    /// <summary>Runs a native call that returns a value, holding the handle open.</summary>
    /// <typeparam name="TResult">The value the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call)
    {
        bool added = false;
        try
        {
            DangerousAddRef(ref added);
            return call(DangerousGetHandle());
        }
        finally
        {
            if (added)
            {
                DangerousRelease();
            }
        }
    }

    /// <summary>Runs a native call that returns nothing, holding the handle open.</summary>
    /// <param name="call">The native call to make.</param>
    public void Use(Action<IntPtr> call)
    {
        bool added = false;
        try
        {
            DangerousAddRef(ref added);
            call(DangerousGetHandle());
        }
        finally
        {
            if (added)
            {
                DangerousRelease();
            }
        }
    }

    /// <summary>
    /// Runs a native call that answers "maybe", holding the handle open, and maps
    /// its bool-plus-out-parameter shape onto a nullable value.
    /// </summary>
    /// <typeparam name="TValue">The value the native call may produce.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>The value the call produced, or <c>null</c> when it produced none.</returns>
    public TValue? UseTry<TValue>(NativeTry<TValue> call)
        where TValue : struct
    {
        bool added = false;
        try
        {
            DangerousAddRef(ref added);
            return call(DangerousGetHandle(), out TValue value) ? value : null;
        }
        finally
        {
            if (added)
            {
                DangerousRelease();
            }
        }
    }

    /// <inheritdoc/>
    protected override bool ReleaseHandle()
    {
        _release(handle);
        return true;
    }
}

/// <summary>A native call that reports whether it produced a value, and writes it.</summary>
/// <typeparam name="TValue">The value the call may produce.</typeparam>
/// <param name="handle">The native handle to act on.</param>
/// <param name="value">The value produced, when the call returns <c>true</c>.</param>
/// <returns><c>true</c> when a value was produced.</returns>
internal delegate bool NativeTry<TValue>(IntPtr handle, out TValue value);
