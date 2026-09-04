using Pamoja.Native.Interop;

namespace Pamoja.Lora;

/// <summary>A channel plan under construction.</summary>
/// <remarks>
/// Tables are indexed by position, so entries are added in data-rate order and a
/// number the plan does not use is added as a reserved data rate. What a region
/// would share between directions is filled in by <see cref="Build"/>: an empty
/// downlink data-rate table mirrors the uplink one, an empty downlink payload
/// table mirrors its uplink counterpart, and an empty back-off chain steps down
/// one data rate at a time.
/// <para>
/// A plan built here is the same kind of thing as a published region. A
/// deployment holding licensed spectrum, or one working somewhere no published
/// plan describes, gets every answer a named band gives.
/// </para>
/// </remarks>
public sealed class LoraPlanBuilder : IDisposable
{
    private IntPtr _builder;

    /// <summary>Starts an empty plan.</summary>
    /// <param name="name">What to call the plan, such as the band it covers.</param>
    /// <exception cref="PamojaException">The native core rejected the name.</exception>
    /// <remarks>
    /// The plan begins with no data rates, channels, or sub-bands, a two-decibel
    /// power ladder, and no dwell-time limit.
    /// </remarks>
    public LoraPlanBuilder(string name)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_new(name, out IntPtr builder));
        _builder = builder;
    }

    /// <summary>Adds the next data rate in a direction.</summary>
    /// <param name="rate">The data rate to add.</param>
    /// <param name="direction">Which table to extend.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder DataRate(
        LoraDataRate rate,
        LoraDirection direction = LoraDirection.Uplink)
    {
        PamojaLoraDataRate native = rate.ToNative();
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_data_rate(
                Live(),
                (uint)direction,
                in native));
        return this;
    }

    /// <summary>Adds the next entry of one payload table.</summary>
    /// <param name="payload">
    /// The limits, or <c>null</c> for a data rate that carries nothing.
    /// </param>
    /// <param name="table">Which table to extend.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder MaxPayload(
        LoraMaxPayload? payload,
        LoraPayloadTable table = LoraPayloadTable.UplinkDirect)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_max_payload(
                Live(),
                (uint)table,
                (byte)(payload.HasValue ? 1 : 0),
                payload?.MacPayload ?? 0,
                payload?.Application ?? 0));
        return this;
    }

    /// <summary>Adds a run of evenly spaced channels.</summary>
    /// <param name="block">The channels to add.</param>
    /// <param name="which">The join set or the default set.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder ChannelBlock(
        LoraChannelBlock block,
        LoraChannelSet which = LoraChannelSet.Default)
    {
        PamojaLoraChannelBlock native = new()
        {
            StartHz = block.StartHz,
            StepHz = block.StepHz,
            Count = block.Count,
            MinDataRate = block.MinDataRate,
            MaxDataRate = block.MaxDataRate,
        };
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_channel_block(
                Live(),
                (uint)which,
                in native));
        return this;
    }

    /// <summary>Adds a sub-band and the transmit limits inside it.</summary>
    /// <param name="band">The sub-band to add.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    /// <remarks>
    /// A deployment on licensed spectrum gives its sub-band a duty cycle of
    /// <c>1000</c>, which reports as unrestricted.
    /// </remarks>
    public LoraPlanBuilder SubBand(LoraSubBand band)
    {
        PamojaLoraSubBand native = new()
        {
            StartHz = band.StartHz,
            EndHz = band.EndHz,
            DutyCyclePermille = band.DutyCyclePermille,
            MaxEirpDbm = band.MaxEirpDbm,
        };
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_sub_band(Live(), in native));
        return this;
    }

    /// <summary>Adds the RX1 downlink data rates for the next uplink data rate.</summary>
    /// <param name="offsets">The downlink data rate at each offset, in order.</param>
    /// <param name="dwellLimited">Whether this extends the dwell-limited mapping.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    /// <remarks>
    /// Every row must be as wide as the plan's highest RX1 offset allows.
    /// </remarks>
    public LoraPlanBuilder Rx1Row(ReadOnlySpan<byte> offsets, bool dwellLimited = false)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_rx1_row(
                Live(),
                (byte)(dwellLimited ? 1 : 0),
                offsets,
                (nuint)offsets.Length));
        return this;
    }

    /// <summary>Adds the next entry of the adaptive back-off chain.</summary>
    /// <param name="lower">
    /// The data rate to fall back to, or <c>null</c> at the slowest, which has
    /// nothing below it.
    /// </param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder Backoff(byte? lower)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_push_backoff(
                Live(),
                (byte)(lower.HasValue ? 1 : 0),
                lower ?? 0));
        return this;
    }

    /// <summary>Sets the transmit-power ladder.</summary>
    /// <param name="defaultMaxEirpDbm">The ceiling where no sub-band says otherwise.</param>
    /// <param name="stepDb">The step between power settings, in decibels.</param>
    /// <param name="maxIndex">The highest power index the plan defines.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder Power(sbyte defaultMaxEirpDbm, byte stepDb = 2, byte maxIndex = 7)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_set_power(
                Live(),
                defaultMaxEirpDbm,
                stepDb,
                maxIndex));
        return this;
    }

    /// <summary>Sets the receive windows.</summary>
    /// <param name="rx2FrequencyHz">The frequency the second window listens on.</param>
    /// <param name="rx2DataRate">The data rate the second window listens at.</param>
    /// <param name="maxRx1Offset">
    /// The highest RX1 offset the plan allows, which fixes how wide every RX1 row
    /// must be.
    /// </param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder Rx(uint rx2FrequencyHz, byte rx2DataRate = 0, byte maxRx1Offset = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_set_rx(
                Live(),
                rx2FrequencyHz,
                rx2DataRate,
                maxRx1Offset));
        return this;
    }

    /// <summary>Sets the Class B beacon and whether the plan limits dwell time.</summary>
    /// <param name="beacon">The beacon settings.</param>
    /// <param name="hasDwellTimeLimit">
    /// Whether the plan caps how long one transmission may hold a channel.
    /// </param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    public LoraPlanBuilder Beacon(LoraBeacon beacon, bool hasDwellTimeLimit = false)
    {
        PamojaLoraBeacon native = new()
        {
            FrequencyHz = beacon.FrequencyHz,
            PingSlotFrequencyHz = beacon.PingSlotFrequencyHz,
            DataRate = beacon.DataRate,
        };
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_set_beacon(
                Live(),
                in native,
                (byte)(hasDwellTimeLimit ? 1 : 0)));
        return this;
    }

    /// <summary>Finishes the plan.</summary>
    /// <returns>The plan, which answers what a published region does.</returns>
    /// <exception cref="PamojaException">
    /// The builder has already been built, or the plan would answer a question
    /// wrongly: an RX1 row narrower than the plan's offsets allow, a table whose
    /// length disagrees with the data rates it indexes, or a second receive
    /// window listening at a data rate the plan does not define.
    /// </exception>
    /// <remarks>
    /// The builder is spent once this returns, whether the plan was accepted or
    /// refused.
    /// </remarks>
    public LoraChannelPlan Build()
    {
        IntPtr builder = Live();
        _builder = IntPtr.Zero;
        Status.ThrowIfError(
            NativeMethods.pamoja_lora_plan_builder_build(builder, out IntPtr plan));
        return new LoraChannelPlan(plan);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_builder != IntPtr.Zero)
        {
            NativeMethods.pamoja_lora_plan_builder_free(_builder);
            _builder = IntPtr.Zero;
        }
    }

    /// <summary>Returns the builder pointer, refusing one already spent.</summary>
    /// <returns>The live builder pointer.</returns>
    /// <exception cref="PamojaException">The builder has already been built.</exception>
    private IntPtr Live() => _builder == IntPtr.Zero
        ? throw new PamojaException("this builder has already been built")
        : _builder;
}
