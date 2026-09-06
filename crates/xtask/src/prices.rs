//! Refresh the prices on the hardware page. Every product page `docs/hardware.toml` lists
//! under a part is fetched, the price it states is read, and the record is written back
//! with the day it was read; each part's offers are then ordered cheapest first. Vendors
//! publish their prices as Schema.org product data, in JSON-LD or in meta tags, which is
//! what is read, so no vendor needs a rule of its own. A page that states no price that
//! way, or that refuses a scripted reader, keeps its last record and is named in the
//! report. A page that offers several variants states a price for each: the offer whose
//! address carries the variant the record's own address names is taken, else the offer
//! whose SKU the record's product name carries, else the one nearest the last record, so
//! the record keeps tracking the variant a person chose. `curl` does the fetching, as it
//! does for the link check.

use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table};

use crate::hardware::Hardware;

// Vendor sites answer a browser's agent string where they refuse a bare one.
const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0 Safari/537.36 pamoja-prices/1.0 (+https://github.com/molexxxx/pamoja)";

/// Indicative rates for ordering offers across currencies. The prices shown stay in the
/// vendor's own currency; the rates only decide which offer comes first.
const RATES: [(&str, f64); 3] = [("USD", 1.0), ("GBP", 1.30), ("EUR", 1.10)];

/// What reading one product page found.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// The amount, in the page's currency.
    pub amount: f64,
    /// The ISO 4217 currency code.
    pub currency: String,
    /// The offer's SKU, where the page states one.
    pub sku: Option<String>,
    /// The offer's own address, where the page states one; a variant's carries its id.
    pub url: Option<String>,
}

/// How to pick one offer among several on a page.
#[derive(Debug, Default)]
pub struct Pick<'a> {
    /// The record's address; a `variant=` in it names the offer wanted.
    pub url: &'a str,
    /// The record's product name; a SKU in it names the offer wanted.
    pub name: &'a str,
    /// The amount last recorded, the tie-break when nothing names the offer.
    pub near: Option<f64>,
}

/// Refresh every price and write the file back.
///
/// # Arguments
///
/// * `root` - the repository root, holding `docs/hardware.toml`.
/// * `report` - where to write the Markdown report, besides printing it.
///
/// # Returns
///
/// Success unless the file cannot be read, parsed, checked, or written. A page that
/// cannot be read is reported, not failed on: a few vendors refuse every scripted client.
pub fn run(root: &Path, report: Option<&Path>) -> ExitCode {
    match refresh(root) {
        Ok(text) => {
            print!("{text}");
            if let Some(path) = report {
                if let Err(err) = fs::write(path, &text) {
                    eprintln!("xtask prices: writing {}: {err}", path.display());
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("xtask prices: {message}");
            ExitCode::FAILURE
        }
    }
}

// One row of the report: which offer, what it said, what it says now.
struct Change {
    part: String,
    vendor: String,
    was: String,
    now: String,
}

fn refresh(root: &Path) -> Result<String, String> {
    let path = root.join("docs/hardware.toml");
    let text =
        fs::read_to_string(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|err| format!("hardware.toml is not valid TOML: {err}"))?;
    let today = today();
    let mut changed = Vec::new();
    let mut unchanged = 0usize;
    let mut unread = Vec::new();

    let entries = doc
        .get_mut("entry")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or("hardware.toml has no [[entry]] tables")?;
    for entry in entries.iter_mut() {
        let part = entry
            .get("key")
            .and_then(Item::as_str)
            .unwrap_or_default()
            .to_owned();
        let Some(buys) = entry.get_mut("buy").and_then(Item::as_array_of_tables_mut) else {
            continue;
        };
        for buy in buys.iter_mut() {
            let vendor = buy
                .get("vendor")
                .and_then(Item::as_str)
                .unwrap_or_default()
                .to_owned();
            let url = buy
                .get("url")
                .and_then(Item::as_str)
                .unwrap_or_default()
                .to_owned();
            let was = buy
                .get("price")
                .and_then(Item::as_str)
                .unwrap_or_default()
                .to_owned();
            let name = buy
                .get("name")
                .and_then(Item::as_str)
                .unwrap_or_default()
                .to_owned();
            let pick = Pick {
                url: &url,
                name: &name,
                near: usd(&was).map(|_| amount_of(&was)),
            };
            match fetch(&url).and_then(|html| {
                reading(&html, &pick)
                    .ok_or_else(|| "the page states no price as product data".to_owned())
            }) {
                Ok(read) => {
                    let now = money(&read);
                    if now != was {
                        changed.push(Change {
                            part: part.clone(),
                            vendor: vendor.clone(),
                            was: was.clone(),
                            now: now.clone(),
                        });
                    } else {
                        unchanged += 1;
                    }
                    buy["price"] = toml_edit::value(now);
                    buy["checked"] = toml_edit::value(today.clone());
                    if buy.get("verified").is_some() {
                        buy["verified"] = toml_edit::value(true);
                    }
                }
                Err(reason) => unread.push(format!("{part}: {vendor} ({url}): {reason}")),
            }
        }
        order(buys);
    }

    let updated = doc.to_string();
    Hardware::parse(&updated)?.check(root)?;
    fs::write(&path, &updated).map_err(|err| format!("writing {}: {err}", path.display()))?;
    Ok(report_text(&today, &changed, unchanged, &unread))
}

// Cheapest first, by the indicative rates; an offer whose price cannot be read as a
// number keeps its place after the rest.
fn order(buys: &mut toml_edit::ArrayOfTables) {
    let mut tables: Vec<Table> = buys.iter().cloned().collect();
    tables.sort_by(|a, b| {
        let key = |t: &Table| {
            t.get("price")
                .and_then(Item::as_str)
                .and_then(usd)
                .unwrap_or(f64::MAX)
        };
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    buys.clear();
    for table in tables {
        buys.push(table);
    }
}

// A price string such as "US$14.95" or "£11.50" as an amount in dollars, for ordering.
fn usd(price: &str) -> Option<f64> {
    let (currency, digits) = if let Some(rest) = price.strip_prefix("US$") {
        ("USD", rest)
    } else if let Some(rest) = price.strip_prefix('£') {
        ("GBP", rest)
    } else {
        ("EUR", price.strip_prefix('€')?)
    };
    let number: String = digits
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let amount: f64 = number.parse().ok()?;
    let rate = RATES
        .iter()
        .find(|(code, _)| *code == currency)
        .map(|(_, rate)| *rate)?;
    Some(amount * rate)
}

// The price as the page states it, in the style the file uses.
fn money(read: &Reading) -> String {
    match read.currency.as_str() {
        "USD" => format!("US${:.2}", read.amount),
        "GBP" => format!("£{:.2}", read.amount),
        "EUR" => format!("€{:.2}", read.amount),
        other => format!("{other} {:.2}", read.amount),
    }
}

/// The price a product page states, from its Schema.org product data: the offers in
/// JSON-LD first, the price meta tags second. Among several offers, the one the record's
/// address or product name names, else the one nearest the last record, else the lowest.
///
/// # Arguments
///
/// * `html` - the page.
/// * `pick` - what names the offer wanted.
///
/// # Returns
///
/// The amount and currency, or `None` when the page states no price that way.
pub fn reading(html: &str, pick: &Pick) -> Option<Reading> {
    let mut found = Vec::new();
    for block in json_ld(html) {
        if let Ok(value) = serde_json::from_str::<Value>(&block) {
            offers(&value, &mut found);
        }
    }
    if found.is_empty() {
        if let Some(read) = meta_price(html) {
            found.push(read);
        }
    }
    let variant = pick
        .url
        .split_once("variant=")
        .map(|(_, rest)| rest.split(['&', '#']).next().unwrap_or(rest).to_owned());
    if let Some(variant) = variant {
        let wanted = format!("variant={variant}");
        if let Some(offer) = found
            .iter()
            .find(|r| r.url.as_deref().is_some_and(|u| u.contains(&wanted)))
        {
            return Some(offer.clone());
        }
    }
    if let Some(offer) = found.iter().find(|r| {
        r.sku
            .as_deref()
            .is_some_and(|sku| !sku.is_empty() && pick.name.contains(sku))
    }) {
        return Some(offer.clone());
    }
    let key = |r: &Reading| match pick.near {
        Some(last) => (r.amount - last).abs(),
        None => r.amount,
    };
    found.into_iter().min_by(|a, b| {
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

// The number in a price string such as "US$14.95", in its own currency.
fn amount_of(price: &str) -> f64 {
    price
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .parse()
        .unwrap_or(0.0)
}

// The bodies of every `<script type="application/ld+json">` on the page.
fn json_ld(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("<script") {
        let start = from + at;
        let Some(open_end) = lower[start..].find('>') else {
            break;
        };
        let tag = &lower[start..start + open_end];
        let body_start = start + open_end + 1;
        let Some(close) = lower[body_start..].find("</script>") else {
            break;
        };
        if tag.contains("ld+json") {
            out.push(html[body_start..body_start + close].to_owned());
        }
        from = body_start + close;
    }
    out
}

// Every offer with a price under a Product, wherever it sits in the document.
fn offers(value: &Value, out: &mut Vec<Reading>) {
    match value {
        Value::Object(map) => {
            if is_product(map.get("@type")) {
                if let Some(offer) = map.get("offers") {
                    collect(offer, out);
                }
            }
            for child in map.values() {
                offers(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                offers(item, out);
            }
        }
        _ => {}
    }
}

fn is_product(kind: Option<&Value>) -> bool {
    match kind {
        Some(Value::String(s)) => s == "Product",
        Some(Value::Array(items)) => items.iter().any(|v| v.as_str() == Some("Product")),
        _ => false,
    }
}

fn collect(offer: &Value, out: &mut Vec<Reading>) {
    match offer {
        Value::Array(items) => {
            for item in items {
                collect(item, out);
            }
        }
        Value::Object(map) => {
            let amount = map
                .get("price")
                .or_else(|| map.get("lowPrice"))
                .and_then(number);
            let currency = map
                .get("priceCurrency")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let (Some(amount), Some(currency)) = (amount, currency) {
                out.push(Reading {
                    amount,
                    currency,
                    sku: map.get("sku").and_then(Value::as_str).map(str::to_owned),
                    url: map.get("url").and_then(Value::as_str).map(str::to_owned),
                });
            } else if let Some(nested) = map.get("offers") {
                collect(nested, out);
            }
        }
        _ => {}
    }
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', "").parse().ok(),
        _ => None,
    }
}

// The `product:price:amount`, `og:price:amount`, or `itemprop="price"` meta tags.
fn meta_price(html: &str) -> Option<Reading> {
    let mut amount = None;
    let mut currency = None;
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("<meta") {
        let start = from + at;
        let Some(end) = lower[start..].find('>') else {
            break;
        };
        let tag = &html[start..start + end];
        let name = attribute(tag, "property")
            .or_else(|| attribute(tag, "itemprop"))
            .or_else(|| attribute(tag, "name"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let content = attribute(tag, "content").unwrap_or_default();
        match name.as_str() {
            "product:price:amount" | "og:price:amount" | "price" if amount.is_none() => {
                amount = content.replace(',', "").trim().parse::<f64>().ok();
            }
            "product:price:currency" | "og:price:currency" | "pricecurrency"
                if currency.is_none() =>
            {
                currency = Some(content.trim().to_uppercase());
            }
            _ => {}
        }
        from = start + end;
    }
    Some(Reading {
        amount: amount?,
        currency: currency?,
        sku: None,
        url: None,
    })
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&format!("{name}="))? + name.len() + 1;
    let rest = &tag[at..];
    let quote = rest.chars().next()?;
    if quote == '"' || quote == '\'' {
        let inner = &rest[1..];
        let end = inner.find(quote)?;
        Some(&inner[..end])
    } else {
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(rest.len());
        Some(&rest[..end])
    }
}

// The page as curl sees it, following redirects; a status other than 200 is a reason.
fn fetch(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--location",
            "--max-time",
            "45",
            "--user-agent",
            AGENT,
            "--write-out",
            "\n%{http_code}",
            url,
        ])
        .output()
        .map_err(|err| format!("running curl: {err}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.trim()));
    match code.trim() {
        "200" => Ok(body.to_owned()),
        "000" => Err("no answer".to_owned()),
        other => Err(format!("answered {other}")),
    }
}

// Today in UTC as YYYY-MM-DD, from the epoch without a calendar dependency.
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil(i64::try_from(secs / 86_400).unwrap_or(0))
}

fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn report_text(today: &str, changed: &[Change], unchanged: usize, unread: &[String]) -> String {
    let mut out = format!("Prices read on {today}.\n\n");
    if changed.is_empty() {
        out.push_str("No price changed.\n");
    } else {
        out.push_str("| Part | Vendor | Was | Now |\n| --- | --- | --- | --- |\n");
        for change in changed {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                change.part, change.vendor, change.was, change.now
            ));
        }
    }
    out.push_str(&format!(
        "\n{unchanged} offer(s) unchanged, re-dated to today.\n"
    ));
    if !unread.is_empty() {
        out.push_str("\nKept the last record, since the page could not be read:\n\n");
        for line in unread {
            out.push_str(&format!("- {line}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lowest_json_ld_offer_is_read() {
        let html = r#"<html><head><script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"Breakout","offers":[{"@type":"Offer","price":"19.95","priceCurrency":"USD"},{"@type":"Offer","price":14.95,"priceCurrency":"USD"}]}</script></head></html>"#;
        let read = reading(html, &Pick::default()).unwrap();
        assert!((read.amount - 14.95).abs() < f64::EPSILON && read.currency == "USD");
    }

    #[test]
    fn a_product_inside_a_graph_and_a_nested_offer_are_found() {
        let html = r#"<script type="application/ld+json">{"@graph":[{"@type":"WebPage"},{"@type":["Product","Thing"],"offers":{"@type":"AggregateOffer","lowPrice":"11.50","priceCurrency":"GBP"}}]}</script>"#;
        assert_eq!(reading(html, &Pick::default()).unwrap().currency, "GBP");
        assert!((reading(html, &Pick::default()).unwrap().amount - 11.5).abs() < f64::EPSILON);
    }

    #[test]
    fn meta_tags_are_the_fallback_and_a_page_without_a_price_reads_as_none() {
        let html = r#"<meta property="og:title" content="x"><meta property="product:price:amount" content="8.90"><meta property="product:price:currency" content="usd">"#;
        assert_eq!(money(&reading(html, &Pick::default()).unwrap()), "US$8.90");
        assert_eq!(
            reading("<html><body>nothing</body></html>", &Pick::default()),
            None
        );
        let broken = r#"<script type="application/ld+json">{not json</script><meta itemprop="price" content="4,999.00"><meta itemprop="priceCurrency" content="EUR">"#;
        assert_eq!(
            money(&reading(broken, &Pick::default()).unwrap()),
            "€4999.00"
        );
    }

    #[test]
    fn a_page_with_variants_yields_the_one_the_record_names() {
        let html = r#"<script type="application/ld+json">{"@type":"Product","offers":[{"price":"43.20","priceCurrency":"GBP","sku":"SC2162","url":"https://shop/pi-5?variant=1"},{"price":"105.60","priceCurrency":"GBP","sku":"SC1111","url":"https://shop/pi-5?variant=2"},{"price":"168.00","priceCurrency":"GBP","sku":"SC1112","url":"https://shop/pi-5?variant=3"}]}</script>"#;
        let by_url = Pick {
            url: "https://shop/pi-5?variant=2",
            name: "Raspberry Pi 5",
            near: Some(43.2),
        };
        assert!((reading(html, &by_url).unwrap().amount - 105.6).abs() < f64::EPSILON);
        let by_sku = Pick {
            url: "https://shop/pi-5",
            name: "Raspberry Pi 5, 8 GB (SC1112)",
            near: None,
        };
        assert!((reading(html, &by_sku).unwrap().amount - 168.0).abs() < f64::EPSILON);
        let by_nearness = Pick {
            url: "https://shop/pi-5",
            name: "Raspberry Pi 5",
            near: Some(100.0),
        };
        assert!((reading(html, &by_nearness).unwrap().amount - 105.6).abs() < f64::EPSILON);
        assert!((reading(html, &Pick::default()).unwrap().amount - 43.2).abs() < f64::EPSILON);
        assert!((amount_of("£88.00") - 88.0).abs() < f64::EPSILON);
    }

    #[test]
    fn offers_order_cheapest_first_across_currencies() {
        let text = "[[entry]]\nkey = \"x\"\n[[entry.buy]]\nvendor = \"A\"\nurl = \"https://a\"\nprice = \"US$16.95\"\nchecked = \"2026-09-06\"\n[[entry.buy]]\nvendor = \"B\"\nurl = \"https://b\"\nprice = \"£11.50\"\nchecked = \"2026-09-06\"\n[[entry.buy]]\nvendor = \"C\"\nurl = \"https://c\"\nprice = \"US$14.95\"\nchecked = \"2026-09-06\"\n";
        let mut doc: DocumentMut = text.parse().unwrap();
        let entries = doc["entry"].as_array_of_tables_mut().unwrap();
        let buys = entries.iter_mut().next().unwrap()["buy"]
            .as_array_of_tables_mut()
            .unwrap();
        order(buys);
        let vendors: Vec<&str> = buys.iter().map(|t| t["vendor"].as_str().unwrap()).collect();
        assert_eq!(vendors, ["C", "B", "A"]);
        assert_eq!(usd("US$14.95"), Some(14.95));
        assert!(usd("£10.00").unwrap() > 12.9 && usd("free").is_none());
    }

    #[test]
    fn the_date_is_civil_utc() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(20_702), "2026-09-06");
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn the_report_names_what_changed_and_what_could_not_be_read() {
        let text = report_text(
            "2026-09-13",
            &[Change {
                part: "bme280".to_owned(),
                vendor: "Adafruit".to_owned(),
                was: "US$14.95".to_owned(),
                now: "US$15.95".to_owned(),
            }],
            3,
            &["uln2003: Digi-Key (https://x): answered 403".to_owned()],
        );
        assert!(text.contains("| bme280 | Adafruit | US$14.95 | US$15.95 |"));
        assert!(text.contains("3 offer(s) unchanged"));
        assert!(text.contains("- uln2003: Digi-Key (https://x): answered 403"));
    }
}
