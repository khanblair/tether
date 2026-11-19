//! Bundled model prices and response cost calculations.

use std::sync::OnceLock;

use serde::Deserialize;
use tether_types::{ModelCaps, ModelPrice, PriceSource, Provider, Support};

use crate::ResponseUsage;

#[derive(Debug, Deserialize)]
struct RawTable {
    provider: String,
    as_of: String,
    models: Vec<RawModel>,
}

#[derive(Debug, Deserialize)]
struct RawModel {
    id: String,
    input_per_million: String,
    output_per_million: String,
    cache_read_per_million: Option<String>,
    cache_write_per_million: Option<String>,
    context_window: Option<u64>,
    max_output_tokens: Option<u64>,
    tools: Option<String>,
    images: Option<String>,
    reasoning: Option<String>,
}

#[derive(Debug)]
struct PriceRow {
    id: String,
    price: ModelPrice,
    details: BundledDetails,
}

/// Optional context, output, and capability values from a bundled table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledDetails {
    /// Maximum context size, when valid.
    pub context_window: Option<u32>,
    /// Maximum generated output size, when valid.
    pub max_output_tokens: Option<u32>,
    /// Bundled capability values.
    pub caps: ModelCaps,
}

#[derive(Debug)]
struct PriceTable {
    as_of: String,
    rows: Vec<PriceRow>,
}

static ANTHROPIC: OnceLock<Option<PriceTable>> = OnceLock::new();
static OPENAI: OnceLock<Option<PriceTable>> = OnceLock::new();
static GEMINI: OnceLock<Option<PriceTable>> = OnceLock::new();
static DEEPSEEK: OnceLock<Option<PriceTable>> = OnceLock::new();

/// Parses a decimal USD amount into micros per million tokens.
///
/// With `per_token` false, the text is USD per million tokens. With `per_token` true,
/// the text is USD per single token, as used by OpenRouter.
#[must_use]
pub fn parse_usd_micros(text: &str, per_token: bool) -> Option<u64> {
    let scale: u128 = if per_token {
        1_000_000_000_000
    } else {
        1_000_000
    };
    let (whole_text, fraction_text) = text.split_once('.').unwrap_or((text, ""));
    if text.matches('.').count() > 1
        || whole_text.is_empty() && fraction_text.is_empty()
        || !whole_text.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole = if whole_text.is_empty() {
        0
    } else {
        whole_text.parse::<u128>().ok()?
    };
    let scaled_whole = whole.checked_mul(scale)?;
    let (fraction, rounded) = scaled_fraction(fraction_text, scale)?;
    let result = scaled_whole.checked_add(fraction)?.checked_add(rounded)?;
    u64::try_from(result).ok()
}

fn scaled_fraction(text: &str, scale: u128) -> Option<(u128, u128)> {
    let precision = if scale == 1_000_000 { 6 } else { 12 };
    let digits = text.as_bytes();
    let kept_len = digits.len().min(precision);
    let kept = if kept_len == 0 {
        0
    } else {
        text[..kept_len].parse::<u128>().ok()?
    };
    let exponent = u32::try_from(precision - kept_len).ok()?;
    let fraction = kept.checked_mul(10_u128.pow(exponent))?;
    let rounded = u128::from(digits.len() > precision && digits[precision] >= b'5');
    Some((fraction, rounded))
}

/// Returns the bundled price for a model, including dated table snapshots.
#[must_use]
pub fn bundled_price(provider: Provider, model_id: &str) -> Option<ModelPrice> {
    let table = table_for(provider)?;
    find_row(&table.rows, model_id).map(|row| row.price)
}

/// Returns optional bundled details for a model, including dated snapshots.
#[must_use]
pub fn bundled_details(provider: Provider, model_id: &str) -> Option<BundledDetails> {
    let table = table_for(provider)?;
    find_row(&table.rows, model_id).map(|row| row.details)
}

fn find_row<'a>(rows: &'a [PriceRow], model_id: &str) -> Option<&'a PriceRow> {
    rows.iter()
        .find(|row| row.id == model_id)
        .or_else(|| rows.iter().find(|row| is_snapshot_id(&row.id, model_id)))
}

/// Returns the date on which a provider's bundled table was written.
#[must_use]
pub fn bundled_as_of(provider: Provider) -> Option<&'static str> {
    Some(&table_for(provider)?.as_of)
}

/// Calculates a response cost in USD micros.
#[must_use]
pub fn cost_micros(price: &ModelPrice, usage: &ResponseUsage) -> u64 {
    let cache_read = price.cache_read_micros.unwrap_or(price.input_micros);
    let cache_write = price.cache_write_micros.unwrap_or(price.input_micros);
    let terms = [
        (usage.input_tokens, price.input_micros),
        (usage.output_tokens, price.output_micros),
        (usage.cache_read_tokens, cache_read),
        (usage.cache_write_tokens, cache_write),
    ];
    let Some(total) = terms.iter().try_fold(0_u128, |total, &(tokens, rate)| {
        total.checked_add(u128::from(tokens).checked_mul(u128::from(rate))?)
    }) else {
        return u64::MAX;
    };
    let rounded = total.saturating_add(500_000) / 1_000_000;
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

fn is_snapshot_id(row_id: &str, model_id: &str) -> bool {
    let Some(date) = model_id
        .strip_prefix(row_id)
        .and_then(|value| value.strip_prefix('-'))
    else {
        return false;
    };
    (date.len() == 8 && date.bytes().all(|byte| byte.is_ascii_digit()))
        || (date.len() == 10
            && date.as_bytes()[4] == b'-'
            && date.as_bytes()[7] == b'-'
            && date[..4].bytes().all(|byte| byte.is_ascii_digit())
            && date[5..7].bytes().all(|byte| byte.is_ascii_digit())
            && date[8..].bytes().all(|byte| byte.is_ascii_digit()))
}

fn table_for(provider: Provider) -> Option<&'static PriceTable> {
    let slot = match provider {
        Provider::Anthropic => ANTHROPIC
            .get_or_init(|| load_table(include_str!("../../data/prices/anthropic.json"), provider)),
        Provider::OpenAi => OPENAI
            .get_or_init(|| load_table(include_str!("../../data/prices/openai.json"), provider)),
        Provider::Gemini => GEMINI
            .get_or_init(|| load_table(include_str!("../../data/prices/gemini.json"), provider)),
        Provider::DeepSeek => DEEPSEEK
            .get_or_init(|| load_table(include_str!("../../data/prices/deepseek.json"), provider)),
        Provider::OpenRouter => return None,
    };
    slot.as_ref()
}

fn load_table(text: &str, provider: Provider) -> Option<PriceTable> {
    let raw = serde_json::from_str::<RawTable>(text).ok()?;
    if raw.provider.parse::<Provider>().ok()? != provider {
        return None;
    }
    let rows = raw
        .models
        .into_iter()
        .filter_map(|model| {
            let input_micros = parse_usd_micros(&model.input_per_million, false)?;
            let output_micros = parse_usd_micros(&model.output_per_million, false)?;
            Some(PriceRow {
                id: model.id,
                price: ModelPrice {
                    input_micros,
                    output_micros,
                    cache_read_micros: model
                        .cache_read_per_million
                        .as_deref()
                        .and_then(|value| parse_usd_micros(value, false)),
                    cache_write_micros: model
                        .cache_write_per_million
                        .as_deref()
                        .and_then(|value| parse_usd_micros(value, false)),
                    source: PriceSource::Bundled,
                },
                details: BundledDetails {
                    context_window: valid_limit(model.context_window),
                    max_output_tokens: valid_limit(model.max_output_tokens),
                    caps: ModelCaps {
                        tools: parse_support(model.tools.as_deref()),
                        images: parse_support(model.images.as_deref()),
                        reasoning: parse_support(model.reasoning.as_deref()),
                    },
                },
            })
        })
        .collect();
    Some(PriceTable {
        as_of: raw.as_of,
        rows,
    })
}

fn valid_limit(value: Option<u64>) -> Option<u32> {
    value
        .filter(|value| *value > 0)
        .and_then(|value| u32::try_from(value).ok())
}

fn parse_support(value: Option<&str>) -> Support {
    match value {
        Some("yes") => Support::Yes,
        Some("no") => Support::No,
        _ => Support::Unknown,
    }
}

#[cfg(test)]
mod tests;
