//! Refresh the prices on the hardware page. Every product page `docs/hardware.toml` lists
//! under a part is fetched, the price it states is read, and the record is written back
//! with the day it was read; each part's offers are then ordered cheapest first. Vendors
//! publish their prices as Schema.org product data, in JSON-LD or in meta tags, which is
//! what is read, so no vendor needs a rule of its own. A page that states no price that
//! way, or that refuses a scripted reader, keeps its last record and is named in the
//! report with the day it was last read. A store that answers that the page does not
//! exist no longer lists the part, and the offer is taken off the page. A page that offers
//! several variants states a price for each: the offer whose address carries the variant
//! the record's own address names is taken, else the offer whose SKU the record's product
//! name carries, else the one nearest the last record, so the record keeps tracking the
//! variant a person chose.
//!
//! Digi-Key refuses scripted readers at its store pages and answers through its Product
//! Information API instead. Where a production app's credentials are set, in
//! `DIGIKEY_CLIENT_ID` and `DIGIKEY_CLIENT_SECRET`, with `DIGIKEY_ACCOUNT_ID` where the
//! account asks for it, a Digi-Key offer is read from the API: its single-unit price, and
//! whether Digi-Key still sells the part. Without them a Digi-Key offer is fetched like
//! any other and kept when refused. `curl` does the fetching, as it does for the link
//! check, and every credential reaches it on standard input rather than its command line.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

// Why an offer was not read, and whether the store has stopped listing it.
#[derive(Debug, PartialEq)]
struct Unread {
    reason: String,
    gone: bool,
}

impl Unread {
    fn kept(reason: impl Into<String>) -> Self {
        Unread {
            reason: reason.into(),
            gone: false,
        }
    }

    fn gone(reason: impl Into<String>) -> Self {
        Unread {
            reason: reason.into(),
            gone: true,
        }
    }
}

// What the refresh found across every offer, for the report.
#[derive(Default)]
struct Found {
    changed: Vec<Change>,
    unchanged: usize,
    removed: Vec<String>,
    notes: Vec<String>,
    unread: Vec<String>,
}

fn refresh(root: &Path) -> Result<String, String> {
    let path = root.join("docs/hardware.toml");
    let text =
        fs::read_to_string(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|err| format!("hardware.toml is not valid TOML: {err}"))?;
    let today = today();
    let mut found = Found::default();
    let mut digikey = DigiKey::from_env();

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
        let mut gone = Vec::new();
        for (at, buy) in buys.iter_mut().enumerate() {
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
            let checked = buy
                .get("checked")
                .and_then(Item::as_str)
                .unwrap_or("an unknown day")
                .to_owned();
            let pick = Pick {
                url: &url,
                name: &name,
                near: usd(&was).map(|_| amount_of(&was)),
            };
            let outcome = match digikey.as_mut().filter(|_| is_digikey(&url)) {
                Some(api) => api.read(&url),
                None => page(&url, &pick).map(|read| (read, None)),
            };
            match outcome {
                Ok((read, note)) => {
                    let now = money(&read);
                    if now != was {
                        found.changed.push(Change {
                            part: part.clone(),
                            vendor: vendor.clone(),
                            was: was.clone(),
                            now: now.clone(),
                        });
                    } else {
                        found.unchanged += 1;
                    }
                    if let Some(note) = note {
                        found.notes.push(format!("{part}: {vendor}: {note}"));
                    }
                    buy["price"] = toml_edit::value(now);
                    buy["checked"] = toml_edit::value(today.clone());
                    if buy.get("verified").is_some() {
                        buy["verified"] = toml_edit::value(true);
                    }
                }
                Err(Unread { reason, gone: true }) => {
                    gone.push(at);
                    found
                        .removed
                        .push(format!("{part}: {vendor} ({url}): {reason}"));
                }
                Err(Unread { reason, .. }) => found.unread.push(format!(
                    "{part}: {vendor} ({url}), last read {checked}: {reason}"
                )),
            }
        }
        settle(entry, &gone, &today);
    }

    let updated = doc.to_string();
    Hardware::parse(&updated)?.check(root)?;
    fs::write(&path, &updated).map_err(|err| format!("writing {}: {err}", path.display()))?;
    Ok(report_text(&today, &found))
}

// Takes the offers at `gone` off a part and orders the rest cheapest first. A part left
// with no offer says so from the day it lost the last one.
fn settle(entry: &mut Table, gone: &[usize], today: &str) {
    let Some(buys) = entry.get_mut("buy").and_then(Item::as_array_of_tables_mut) else {
        return;
    };
    for &at in gone.iter().rev() {
        buys.remove(at);
    }
    if buys.is_empty() {
        entry.remove("buy");
        entry["buy_checked"] = toml_edit::value(today);
    } else {
        order(buys);
    }
}

// The price a store page states, or why it could not be read.
fn page(url: &str, pick: &Pick) -> Result<Reading, Unread> {
    let html = fetch(url)?;
    reading(&html, pick).ok_or_else(|| Unread::kept("the page states no price as product data"))
}

// Whether an address is on Digi-Key's store.
fn is_digikey(url: &str) -> bool {
    url.split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .is_some_and(|host| host == "digikey.com" || host.ends_with(".digikey.com"))
}

// Where Digi-Key hands out an access token for a production app's credentials.
const DIGIKEY_TOKEN: &str = "https://api.digikey.com/v1/oauth2/token";

// The product details operation of Product Information V4, before the product number.
const DIGIKEY_SEARCH: &str = "https://api.digikey.com/products/v4/search/";

// A token lasts ten minutes; one older than this is replaced before it is used.
const TOKEN_LIFE: Duration = Duration::from_secs(8 * 60);

/// Digi-Key's Product Information V4 API, read with a production app's credentials
/// under the client credentials grant.
struct DigiKey {
    client_id: String,
    client_secret: String,
    account_id: Option<String>,
    token: Option<(String, Instant)>,
}

impl DigiKey {
    /// The app named by the environment, or `None` when its credentials are not set.
    ///
    /// # Returns
    ///
    /// The client, before any token is asked for.
    fn from_env() -> Option<DigiKey> {
        let set = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Some(DigiKey {
            client_id: set("DIGIKEY_CLIENT_ID")?,
            client_secret: set("DIGIKEY_CLIENT_SECRET")?,
            account_id: set("DIGIKEY_ACCOUNT_ID"),
            token: None,
        })
    }

    /// A current access token, asked for again once the last one is near its end.
    ///
    /// # Errors
    ///
    /// Kept, naming the status, when Digi-Key refuses the credentials.
    fn token(&mut self) -> Result<String, Unread> {
        if let Some((token, at)) = &self.token {
            if at.elapsed() < TOKEN_LIFE {
                return Ok(token.clone());
            }
        }
        let form = format!(
            "client_id={}&client_secret={}&grant_type=client_credentials",
            encode(&self.client_id),
            encode(&self.client_secret)
        );
        let (body, code) =
            curl(&["--data", "@-", DIGIKEY_TOKEN], Some(&form)).map_err(Unread::kept)?;
        if code != "200" {
            return Err(Unread::kept(format!(
                "Digi-Key refused the app's credentials, answering {code}"
            )));
        }
        let token = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| value.get("access_token")?.as_str().map(str::to_owned))
            .ok_or_else(|| Unread::kept("Digi-Key's token answer carried no access token"))?;
        self.token = Some((token.clone(), Instant::now()));
        Ok(token)
    }

    /// Reads one Digi-Key offer through the API.
    ///
    /// # Arguments
    ///
    /// * `url` - the offer's store page, which names the part and the listing.
    ///
    /// # Returns
    ///
    /// The single-unit price, and a note where Digi-Key marks the part anything but active.
    ///
    /// # Errors
    ///
    /// Gone when Digi-Key no longer lists or sells the part; kept for anything else.
    fn read(&mut self, url: &str) -> Result<(Reading, Option<String>), Unread> {
        let (part, listing) = digikey_part(url)
            .ok_or_else(|| Unread::kept("the address names no Digi-Key product"))?;
        let token = self.token()?;
        let mut headers = format!(
            "Authorization: Bearer {token}\nX-DIGIKEY-Client-Id: {}\nX-DIGIKEY-Locale-Site: US\nX-DIGIKEY-Locale-Language: en\nX-DIGIKEY-Locale-Currency: USD\nAccept: application/json\n",
            self.client_id
        );
        if let Some(account) = &self.account_id {
            headers.push_str(&format!("X-DIGIKEY-Account-Id: {account}\n"));
        }
        let address = format!("{DIGIKEY_SEARCH}{}/productdetails", encode(&part));
        let (body, code) =
            curl(&["--header", "@-", &address], Some(&headers)).map_err(Unread::kept)?;
        match code.as_str() {
            "200" => details(&body, listing),
            "404" => Err(Unread::gone("Digi-Key no longer lists the part")),
            other => Err(Unread::kept(format!("Digi-Key's API answered {other}"))),
        }
    }
}

// The manufacturer part number and Digi-Key's listing number from a store address of the
// shape `/en/products/detail/<maker>/<part>/<listing>`.
fn digikey_part(url: &str) -> Option<(String, &str)> {
    let path = url.split(['?', '#']).next()?;
    let rest = path.split_once("/products/detail/")?.1;
    let mut segments = rest.trim_end_matches('/').split('/');
    let _maker = segments.next()?;
    let part = segments.next().filter(|s| !s.is_empty())?;
    let listing = segments.next().filter(|s| !s.is_empty())?;
    Some((decode(part), listing))
}

// What Digi-Key's product details say about the listing the record names.
fn details(body: &str, listing: &str) -> Result<(Reading, Option<String>), Unread> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| Unread::kept("Digi-Key's answer is not JSON"))?;
    let product = value
        .get("Product")
        .ok_or_else(|| Unread::kept("Digi-Key's answer names no product"))?;
    let address = product
        .get("ProductUrl")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !address
        .trim_end_matches('/')
        .ends_with(&format!("/{listing}"))
    {
        return Err(Unread::kept(format!(
            "Digi-Key matched a different listing, {address}"
        )));
    }
    let status = product
        .pointer("/ProductStatus/Status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let flag = |name: &str| product.get(name).and_then(Value::as_bool).unwrap_or(false);
    let stock = product
        .get("QuantityAvailable")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let lower = status.to_ascii_lowercase();
    if flag("Discontinued")
        || lower.starts_with("discontinued")
        || (lower == "obsolete" && stock == 0)
    {
        let marked = if status.is_empty() {
            "discontinued"
        } else {
            status
        };
        return Err(Unread::gone(format!("Digi-Key marks it {marked}")));
    }
    let amount = product
        .get("UnitPrice")
        .and_then(number)
        .filter(|amount| *amount > 0.0)
        .ok_or_else(|| Unread::kept("Digi-Key states no single-unit price"))?;
    let currency = value
        .pointer("/SearchLocaleUsed/Currency")
        .and_then(Value::as_str)
        .unwrap_or("USD")
        .to_ascii_uppercase();
    let note = if flag("EndOfLife") {
        Some("the maker has ended it; Digi-Key sells the stock that remains".to_owned())
    } else if !status.is_empty() && lower != "active" {
        Some(format!("Digi-Key marks it {status}"))
    } else {
        None
    };
    Ok((
        Reading {
            amount,
            currency,
            sku: None,
            url: None,
        },
        note,
    ))
}

// Percent-encodes everything but the unreserved characters, for a path segment or a form.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

// Undoes percent-encoding; a malformed escape is left as written.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| text.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// Cheapest first, by the indicative rates; an offer whose price cannot be read as a
// number keeps its place after the rest.
fn order(buys: &mut toml_edit::ArrayOfTables) {
    // A table remembers where in the document it was written, and that position, not the
    // order of the array, is what decides where it is rendered. Sorting the array alone
    // leaves the file exactly as it was, so the slots are collected here and handed back
    // out in the sorted order.
    let mut slots: Vec<Option<isize>> = buys.iter().map(Table::position).collect();
    slots.sort_unstable();
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
    for (mut table, slot) in tables.into_iter().zip(slots) {
        table.set_position(slot);
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

// The page as curl sees it, following redirects. A page the store says does not exist is
// gone; any other status but 200 is a reason to keep the last record.
fn fetch(url: &str) -> Result<String, Unread> {
    let (body, code) =
        curl(&["--location", "--user-agent", AGENT, url], None).map_err(Unread::kept)?;
    match code.as_str() {
        "200" => Ok(body),
        "404" | "410" => Err(Unread::gone(format!(
            "answered {code}, so the store no longer lists it"
        ))),
        "000" => Err(Unread::kept("no answer")),
        other => Err(Unread::kept(format!("answered {other}"))),
    }
}

// curl with the given arguments and, where given, `input` on its standard input, which is
// how a credential reaches it without appearing on a command line. The body and the
// status code.
fn curl(args: &[&str], input: Option<&str>) -> Result<(String, String), String> {
    let mut child = Command::new("curl")
        .args([
            "--silent",
            "--max-time",
            "45",
            "--write-out",
            "\n%{http_code}",
        ])
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| format!("running curl: {err}"))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin
            .write_all(text.as_bytes())
            .map_err(|err| format!("writing to curl: {err}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("running curl: {err}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.trim()));
    Ok((body.to_owned(), code.trim().to_owned()))
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

fn report_text(today: &str, found: &Found) -> String {
    let mut out = format!("Prices read on {today}.\n\n");
    if found.changed.is_empty() {
        out.push_str("No price changed.\n");
    } else {
        out.push_str("| Part | Vendor | Was | Now |\n| --- | --- | --- | --- |\n");
        for change in &found.changed {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                change.part, change.vendor, change.was, change.now
            ));
        }
    }
    out.push_str(&format!(
        "\n{} offer(s) unchanged, re-dated to today.\n",
        found.unchanged
    ));
    let sections = [
        (
            "Taken off the page, since the store no longer lists the part:",
            &found.removed,
        ),
        ("Read, with something to know about the part:", &found.notes),
        (
            "Kept the last record, since the page could not be read:",
            &found.unread,
        ),
    ];
    for (heading, lines) in sections {
        if !lines.is_empty() {
            out.push_str(&format!("\n{heading}\n\n"));
            for line in lines {
                out.push_str(&format!("- {line}\n"));
            }
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

        // The written file is what a reader sees, and a table is rendered where its
        // recorded position says, so the order has to hold after the document is
        // printed and not only in the array.
        let rendered = doc.to_string();
        let at = |vendor: &str| rendered.find(vendor).expect("the vendor is written");
        assert!(
            at("\"C\"") < at("\"B\"") && at("\"B\"") < at("\"A\""),
            "cheapest first survives rendering:\n{rendered}"
        );
    }

    #[test]
    fn the_date_is_civil_utc() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(20_702), "2026-09-06");
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn the_report_names_what_changed_what_was_removed_and_what_could_not_be_read() {
        let found = Found {
            changed: vec![Change {
                part: "bme280".to_owned(),
                vendor: "Adafruit".to_owned(),
                was: "US$14.95".to_owned(),
                now: "US$15.95".to_owned(),
            }],
            unchanged: 3,
            removed: vec!["ina226: A store (https://y): answered 404".to_owned()],
            notes: vec!["hdc1080: Digi-Key: Digi-Key marks it Last Time Buy".to_owned()],
            unread: vec![
                "uln2003: Digi-Key (https://x), last read 2026-09-06: answered 403".to_owned(),
            ],
        };
        let text = report_text("2026-09-13", &found);
        assert!(text.contains("| bme280 | Adafruit | US$14.95 | US$15.95 |"));
        assert!(text.contains("3 offer(s) unchanged"));
        assert!(text.contains("no longer lists the part:\n\n- ina226: A store"));
        assert!(text.contains("- hdc1080: Digi-Key: Digi-Key marks it Last Time Buy"));
        assert!(text.contains("- uln2003: Digi-Key (https://x), last read 2026-09-06"));
        assert!(!report_text("2026-09-13", &Found::default()).contains("Taken off"));
    }

    #[test]
    fn an_offer_that_is_gone_leaves_the_file_and_a_part_left_bare_says_so() {
        let text = "[[entry]]\nkey = \"x\"\n[[entry.buy]]\nvendor = \"A\"\nurl = \"https://a\"\nprice = \"US$16.95\"\nchecked = \"2026-09-06\"\n[[entry.buy]]\nvendor = \"B\"\nurl = \"https://b\"\nprice = \"US$9.00\"\nchecked = \"2026-09-06\"\n[[entry]]\nkey = \"y\"\n[[entry.buy]]\nvendor = \"C\"\nurl = \"https://c\"\nprice = \"US$1.00\"\nchecked = \"2026-09-06\"\n";
        let mut doc: DocumentMut = text.parse().unwrap();
        {
            let entries = doc["entry"].as_array_of_tables_mut().unwrap();
            let mut tables = entries.iter_mut();
            settle(tables.next().unwrap(), &[1], "2026-09-23");
            settle(tables.next().unwrap(), &[0], "2026-09-23");
        }
        let rendered = doc.to_string();
        assert!(rendered.contains("vendor = \"A\"") && !rendered.contains("vendor = \"B\""));
        assert!(!rendered.contains("vendor = \"C\""));
        assert!(rendered.contains("buy_checked = \"2026-09-23\""));
        assert_eq!(rendered.matches("buy_checked").count(), 1);
    }

    #[test]
    fn a_digikey_address_names_the_part_and_the_listing() {
        assert!(is_digikey(
            "https://www.digikey.com/en/products/detail/texas-instruments/ULN2003AN/277624"
        ));
        assert!(!is_digikey("https://www.adafruit.com/product/2652"));
        assert!(!is_digikey("https://notdigikey.com/x"));
        assert_eq!(
            digikey_part(
                "https://www.digikey.com/en/products/detail/texas-instruments/ULN2003AN/277624?s=1"
            ),
            Some(("ULN2003AN".to_owned(), "277624"))
        );
        assert_eq!(
            digikey_part("https://www.digikey.com/en/products/detail/maker/AB%2FC-1/99/"),
            Some(("AB/C-1".to_owned(), "99"))
        );
        assert_eq!(
            digikey_part("https://www.digikey.com/en/products/result?keywords=x"),
            None
        );
        assert_eq!(encode("AB/C 1"), "AB%2FC%201");
        assert_eq!(decode(&encode("a+b/c~d")), "a+b/c~d");
        assert_eq!(decode("100%"), "100%");
    }

    fn product(fields: &str) -> String {
        format!(
            r#"{{"SearchLocaleUsed":{{"Site":"US","Language":"en","Currency":"USD"}},"Product":{{"ProductUrl":"https://www.digikey.com/en/products/detail/texas-instruments/ULN2003AN/277624",{fields}}}}}"#
        )
    }

    #[test]
    fn an_active_digikey_part_is_read_at_its_single_unit_price() {
        let body = product(
            r#""UnitPrice":0.71,"QuantityAvailable":5000,"Discontinued":false,"EndOfLife":false,"ProductStatus":{"Id":0,"Status":"Active"}"#,
        );
        let (read, note) = details(&body, "277624").unwrap();
        assert_eq!(money(&read), "US$0.71");
        assert_eq!(note, None);
    }

    #[test]
    fn a_digikey_part_it_no_longer_sells_is_gone_and_one_at_end_of_life_is_noted() {
        let discontinued = product(
            r#""UnitPrice":0.71,"QuantityAvailable":0,"Discontinued":true,"ProductStatus":{"Status":"Discontinued at Digi-Key"}"#,
        );
        let unread = details(&discontinued, "277624").unwrap_err();
        assert!(unread.gone && unread.reason.contains("Discontinued at Digi-Key"));

        let obsolete_and_empty =
            product(r#""UnitPrice":0,"QuantityAvailable":0,"ProductStatus":{"Status":"Obsolete"}"#);
        assert!(details(&obsolete_and_empty, "277624").unwrap_err().gone);

        let ending = product(
            r#""UnitPrice":1.20,"QuantityAvailable":40,"EndOfLife":true,"ProductStatus":{"Status":"Last Time Buy"}"#,
        );
        let (read, note) = details(&ending, "277624").unwrap();
        assert_eq!(money(&read), "US$1.20");
        assert!(note.unwrap().contains("the maker has ended it"));
    }

    #[test]
    fn a_digikey_answer_for_another_listing_or_without_a_price_is_kept() {
        let body = product(r#""UnitPrice":0.71,"ProductStatus":{"Status":"Active"}"#);
        let other = details(&body, "999999").unwrap_err();
        assert!(!other.gone && other.reason.contains("a different listing"));
        let unpriced = product(r#""UnitPrice":0,"ProductStatus":{"Status":"Active"}"#);
        assert_eq!(
            details(&unpriced, "277624").unwrap_err(),
            Unread::kept("Digi-Key states no single-unit price")
        );
        assert!(!details("not json", "277624").unwrap_err().gone);
    }
}
