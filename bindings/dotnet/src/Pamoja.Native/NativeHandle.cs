using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// A <see cref="SafeHandle"/> over a native pointer whose release function is
/// supplied at construction, so the handle is always released exactly once even
/// across finalization races.
/// </summary>
/// <remarks>
/// The helper-math capabilities expose a family of handles that differ only in
/// which <c>*_free</c> function releases them, so one handle type carrying that
/// function serves them all.
///
/// A handle behind an object with asynchronous members is created serialized: its
/// calls run one at a time, so a send issued while a receive is waiting runs once
/// the receive returns, rather than reaching the native object alongside it. A call
/// that waits its turn through <see cref="UseAsync{TResult}"/> holds no thread while
/// it waits.
/// </remarks>
public sealed class NativeHandle : SafeHandle
{
    private readonly Action<IntPtr> _release;
    private readonly SemaphoreSlim? _gate;

    /// <summary>Wraps a non-null native pointer with the function that frees it.</summary>
    /// <param name="handle">The pointer returned by a native constructor.</param>
    /// <param name="release">The matching native release function.</param>
    /// <param name="serialized">Run the calls on this handle one at a time.</param>
    public NativeHandle(IntPtr handle, Action<IntPtr> release, bool serialized = false)
        : base(IntPtr.Zero, ownsHandle: true)
    {
        _release = release;
        _gate = serialized ? new SemaphoreSlim(1, 1) : null;
        SetHandle(handle);
    }

    /// <inheritdoc/>
    public override bool IsInvalid => handle == IntPtr.Zero;

    /// <summary>Creates a wrapper, throwing when the native constructor returned null.</summary>
    /// <param name="handle">The pointer returned by a native constructor.</param>
    /// <param name="release">The matching native release function.</param>
    /// <param name="what">What was being created, for the exception message.</param>
    /// <param name="serialized">Run the calls on this handle one at a time.</param>
    /// <returns>The wrapped handle.</returns>
    /// <exception cref="PamojaException">The native constructor returned null.</exception>
    public static NativeHandle Create(
        IntPtr handle, Action<IntPtr> release, string what, bool serialized = false)
    {
        if (handle == IntPtr.Zero)
        {
            throw new PamojaException(Status.LastError() ?? $"failed to create the {what}");
        }

        return new NativeHandle(handle, release, serialized);
    }

    /// <summary>Runs a native call that returns a value, holding the handle open.</summary>
    /// <typeparam name="TResult">The value the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public TResult Use<TResult>(Func<IntPtr, TResult> call)
    {
        _gate?.Wait();
        try
        {
            return Invoke(call);
        }
        finally
        {
            _gate?.Release();
        }
    }

    /// <summary>Runs a native call that returns nothing, holding the handle open.</summary>
    /// <param name="call">The native call to make.</param>
    public void Use(Action<IntPtr> call) => Use(handle =>
    {
        call(handle);
        return 0;
    });

    /// <summary>
    /// Runs a native call that answers "maybe", holding the handle open, and maps
    /// its bool-plus-out-parameter shape onto a nullable value.
    /// </summary>
    /// <typeparam name="TValue">The value the native call may produce.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>The value the call produced, or <c>null</c> when it produced none.</returns>
    public TValue? UseTry<TValue>(NativeTry<TValue> call)
        where TValue : struct =>
        Use<TValue?>(handle => call(handle, out TValue value) ? value : null);

    /// <summary>Runs a native call on the thread pool, holding the handle open.</summary>
    /// <remarks>
    /// On a serialized handle the call first waits, without holding a thread, for
    /// any call already running on it.
    /// </remarks>
    /// <typeparam name="TResult">The value the native call returns.</typeparam>
    /// <param name="call">The native call to make.</param>
    /// <returns>Whatever the native call returned.</returns>
    public async Task<TResult> UseAsync<TResult>(Func<IntPtr, TResult> call)
    {
        if (_gate is null)
        {
            return await Task.Run(() => Invoke(call)).ConfigureAwait(false);
        }

        await _gate.WaitAsync().ConfigureAwait(false);
        try
        {
            return await Task.Run(() => Invoke(call)).ConfigureAwait(false);
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>Runs a native call that returns nothing on the thread pool, holding the handle open.</summary>
    /// <param name="call">The native call to make.</param>
    /// <returns>A task that completes when the call has returned.</returns>
    public Task UseAsync(Action<IntPtr> call) => UseAsync(handle =>
    {
        call(handle);
        return 0;
    });

    /// <summary>
    /// Lends the pointer to asynchronous work, such as a call on another handle that
    /// takes this one as an argument, holding the handle open until the work ends.
    /// </summary>
    /// <typeparam name="TResult">The value the work produces.</typeparam>
    /// <param name="work">The work to run with the pointer.</param>
    /// <returns>Whatever the work produced.</returns>
    public async Task<TResult> LendAsync<TResult>(Func<IntPtr, Task<TResult>> work)
    {
        if (_gate is not null)
        {
            await _gate.WaitAsync().ConfigureAwait(false);
        }

        bool added = false;
        try
        {
            DangerousAddRef(ref added);
            return await work(DangerousGetHandle()).ConfigureAwait(false);
        }
        finally
        {
            if (added)
            {
                DangerousRelease();
            }

            _gate?.Release();
        }
    }

    /// <summary>Hands the pointer to a native call that takes ownership of it.</summary>
    /// <remarks>
    /// This handle never releases the pointer afterwards. A call still running on a
    /// serialized handle is using the native object, so taking it then throws rather
    /// than giving away memory that call is still reading.
    /// </remarks>
    /// <param name="busy">The message for a handle with a call running.</param>
    /// <returns>The pointer, which the caller now owns.</returns>
    /// <exception cref="PamojaException">A call on this handle is still running.</exception>
    public IntPtr Take(string busy)
    {
        if (_gate is not null && !_gate.Wait(0))
        {
            throw new PamojaException(busy);
        }

        bool added = false;
        try
        {
            DangerousAddRef(ref added);
            IntPtr pointer = DangerousGetHandle();
            SetHandleAsInvalid();
            return pointer;
        }
        finally
        {
            if (added)
            {
                DangerousRelease();
            }

            _gate?.Release();
        }
    }

    /// <inheritdoc/>
    protected override bool ReleaseHandle()
    {
        _release(handle);
        return true;
    }

    /// <summary>Runs a native call with the handle held open, outside the gate.</summary>
    private TResult Invoke<TResult>(Func<IntPtr, TResult> call)
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
}

/// <summary>A native call that reports whether it produced a value, and writes it.</summary>
/// <typeparam name="TValue">The value the call may produce.</typeparam>
/// <param name="handle">The native handle to act on.</param>
/// <param name="value">The value produced, when the call returns <c>true</c>.</param>
/// <returns><c>true</c> when a value was produced.</returns>
public delegate bool NativeTry<TValue>(IntPtr handle, out TValue value);
