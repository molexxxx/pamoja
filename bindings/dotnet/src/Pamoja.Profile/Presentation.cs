using System.Text.Json;
using System.Text.Json.Serialization;

namespace Pamoja.Profile;

/// <summary>
/// The graphic a dashboard draws an element with, named by the instrument rather than
/// the quantity. On the wire each is its lowercase name, as a manifest carries it.
/// </summary>
[JsonConverter(typeof(VizConverter))]
public enum Viz
{
    /// <summary>A rolling sparkline of recent values.</summary>
    Spark,

    /// <summary>A 270-degree arch gauge, for a fraction or percentage.</summary>
    Gauge,

    /// <summary>A half-dial with a needle, for a pressure or flow reading.</summary>
    Dial,

    /// <summary>A horizontal bar with a safe-band tick, for a level or stock.</summary>
    Bar,

    /// <summary>A thermometer, for a temperature.</summary>
    Thermometer,

    /// <summary>A liquid-filled droplet, for humidity or moisture.</summary>
    Droplet,

    /// <summary>A segmented battery cell, for a state of charge or voltage.</summary>
    Battery,

    /// <summary>An anemometer, for wind speed.</summary>
    Wind,

    /// <summary>A sun whose corona grows with the reading, for illuminance.</summary>
    Sun,

    /// <summary>An acoustic waveform, for sound level or an acoustic event.</summary>
    Wave,

    /// <summary>A labeled state chip, lit when the state reads as on.</summary>
    Switch,

    /// <summary>A pipe valve, open along the flow or closed across it.</summary>
    Valve,

    /// <summary>A row of hash-chained blocks, for a tamper-evident record count.</summary>
    Chain,

    /// <summary>A neighbor-mesh topology map, for a mesh node's peers.</summary>
    Mesh,

    /// <summary>A plain numeric counter, for a node or network stat.</summary>
    Count,
}

/// <summary>
/// A piece of text a profile supplies for one of its codes: one string for every
/// locale, or a map from locale tag to text.
/// </summary>
/// <remarks>
/// Exactly one of <see cref="Text"/> and <see cref="PerLocale"/> is set. A string
/// converts implicitly, so <c>messages["state.flushing"] = "Flushing"</c> reads as it
/// would in a manifest.
/// </remarks>
[JsonConverter(typeof(LocalizedTextConverter))]
public sealed record LocalizedText
{
    /// <summary>Gets the text shown in every locale, when one text serves all.</summary>
    public string? Text { get; init; }

    /// <summary>Gets the per-locale text, keyed by locale tag (<c>en</c>, <c>sw</c>, ...).</summary>
    public IReadOnlyDictionary<string, string>? PerLocale { get; init; }

    /// <summary>Wraps one text for every locale.</summary>
    /// <param name="text">The text.</param>
    public static implicit operator LocalizedText(string text) => new() { Text = text };

    /// <summary>Wraps per-locale text.</summary>
    /// <param name="perLocale">The text by locale tag.</param>
    public static implicit operator LocalizedText(Dictionary<string, string> perLocale) =>
        new() { PerLocale = perLocale };
}

/// <summary>A custom sensor or node stat a profile contributes to the dashboard.</summary>
/// <param name="Key">The stable, language-neutral element key, such as <c>water_turbidity</c>.</param>
/// <param name="Unit">The canonical unit name, such as <c>ntu</c>, <c>ph</c>, or <c>count</c>.</param>
/// <param name="Label">A human-readable fallback label, shown when no localized label applies.</param>
/// <param name="Viz">The graphic this element is drawn with.</param>
public sealed record ElementSpec(string Key, string Unit, string Label, Viz Viz)
{
    /// <summary>Gets the per-locale labels, keyed by locale tag.</summary>
    public IReadOnlyDictionary<string, string>? Labels { get; init; }

    /// <summary>Gets the safe band as <c>[low, high]</c> in the element's unit.</summary>
    public float[]? Band { get; init; }

    /// <summary>Gets whether this is a node or network stat rather than a measurement of the world.</summary>
    public bool Stat { get; init; }

    /// <summary>
    /// Gets the link kinds whose groups this element is offered on, such as <c>["mesh"]</c>;
    /// <c>null</c> means every group.
    /// </summary>
    [JsonConverter(typeof(ScopeConverter))]
    public IReadOnlyList<string>? Scope { get; init; }

    /// <summary>Gets whether the element's tile spans two columns.</summary>
    public bool Span { get; init; }

    /// <summary>Gets a starting numeric value for the add-sensor dialog.</summary>
    public float? Value { get; init; }

    /// <summary>Gets a starting discrete state code, such as <c>state.closed</c>, for a non-numeric element.</summary>
    public string? State { get; init; }
}

/// <summary>The theme tokens a profile sets on the dashboard; each is any CSS color.</summary>
public sealed record Theme
{
    /// <summary>Gets the brand and interaction accent.</summary>
    public string? Accent { get; init; }

    /// <summary>Gets the healthy status color, which also tints an in-band gauge.</summary>
    public string? Ok { get; init; }

    /// <summary>Gets the warning status color.</summary>
    public string? Warn { get; init; }

    /// <summary>Gets the alarm status color.</summary>
    public string? Alarm { get; init; }

    /// <summary>Gets the unfilled track color behind gauges and bars.</summary>
    public string? Track { get; init; }
}

/// <summary>
/// How a profile presents itself on the dashboard: its custom elements, an optional
/// theme, and the words for any state or event code it introduces.
/// </summary>
/// <param name="Elements">The custom sensors and node stats this profile contributes.</param>
public sealed record Presentation(IReadOnlyList<ElementSpec> Elements)
{
    /// <summary>Gets an optional theme that tints the dashboard.</summary>
    public Theme? Theme { get; init; }

    /// <summary>
    /// Gets the text for the codes this profile introduces, keyed by the page's message
    /// key (<c>state.flushing</c>, <c>event.filter_clog</c>).
    /// </summary>
    public IReadOnlyDictionary<string, LocalizedText>? Messages { get; init; }

    /// <summary>The serializer settings that match the manifest's JSON one for one.</summary>
    internal static readonly JsonSerializerOptions Wire = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        WriteIndented = true,
    };

    /// <summary>Reads a presentation from the JSON object a manifest carries under <c>presentation</c>.</summary>
    /// <param name="json">The JSON object.</param>
    /// <returns>The presentation.</returns>
    /// <exception cref="JsonException">The text is not a presentation.</exception>
    public static Presentation FromJson(string json) =>
        JsonSerializer.Deserialize<Presentation>(json, Wire)
        ?? throw new JsonException("the text is not a presentation");

    /// <summary>Writes this presentation as the JSON object a manifest carries.</summary>
    /// <returns>The JSON text.</returns>
    public string ToJson() => JsonSerializer.Serialize(this, Wire);
}

/// <summary>Reads and writes a <see cref="Viz"/> as its lowercase manifest name.</summary>
internal sealed class VizConverter : JsonConverter<Viz>
{
    /// <inheritdoc/>
    public override Viz Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        string name = reader.GetString() ?? throw new JsonException("a graphic is named by a string");
        foreach (Viz viz in Enum.GetValues<Viz>())
        {
            if (string.Equals(viz.ToString(), name, StringComparison.OrdinalIgnoreCase))
            {
                return viz;
            }
        }

        throw new JsonException($"`{name}` is not a graphic the dashboard draws");
    }

    /// <inheritdoc/>
    public override void Write(Utf8JsonWriter writer, Viz value, JsonSerializerOptions options) =>
        writer.WriteStringValue(value.ToString().ToLowerInvariant());
}

/// <summary>
/// Reads and writes an element's scope: the string <c>always</c> becomes <c>null</c>,
/// and an object naming <c>links</c> becomes the list.
/// </summary>
internal sealed class ScopeConverter : JsonConverter<IReadOnlyList<string>?>
{
    /// <inheritdoc/>
    public override IReadOnlyList<string>? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.String)
        {
            string word = reader.GetString() ?? string.Empty;
            return word == "always"
                ? null
                : throw new JsonException($"`{word}` is not a scope; use `always` or an object naming `links`");
        }

        if (reader.TokenType != JsonTokenType.StartObject)
        {
            throw new JsonException("a scope is `always` or an object naming `links`");
        }

        List<string>? links = null;
        while (reader.Read() && reader.TokenType != JsonTokenType.EndObject)
        {
            string property = reader.GetString() ?? string.Empty;
            reader.Read();
            if (property == "links")
            {
                links = JsonSerializer.Deserialize<List<string>>(ref reader, options);
            }
            else
            {
                reader.Skip();
            }
        }

        return links ?? throw new JsonException("a scoped element names its `links`");
    }

    /// <inheritdoc/>
    public override void Write(Utf8JsonWriter writer, IReadOnlyList<string>? value, JsonSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteStringValue("always");
            return;
        }

        writer.WriteStartObject();
        writer.WritePropertyName("links");
        JsonSerializer.Serialize(writer, value, options);
        writer.WriteEndObject();
    }
}

/// <summary>Reads and writes a <see cref="LocalizedText"/> as a string or a locale map.</summary>
internal sealed class LocalizedTextConverter : JsonConverter<LocalizedText>
{
    /// <inheritdoc/>
    public override LocalizedText Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.String)
        {
            return new LocalizedText { Text = reader.GetString() };
        }

        if (reader.TokenType != JsonTokenType.StartObject)
        {
            throw new JsonException("a message is a string or an object keyed by locale");
        }

        Dictionary<string, string> perLocale = JsonSerializer.Deserialize<Dictionary<string, string>>(ref reader, options)
            ?? throw new JsonException("a message is a string or an object keyed by locale");
        return new LocalizedText { PerLocale = perLocale };
    }

    /// <inheritdoc/>
    public override void Write(Utf8JsonWriter writer, LocalizedText value, JsonSerializerOptions options)
    {
        if (value.PerLocale is not null)
        {
            JsonSerializer.Serialize(writer, value.PerLocale, options);
        }
        else
        {
            writer.WriteStringValue(value.Text ?? string.Empty);
        }
    }
}
