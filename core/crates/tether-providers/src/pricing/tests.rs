use std::collections::HashSet;

use super::*;
use tether_types::{ModelCaps, ModelPrice, PriceSource, Provider};

#[test]
fn parses_decimal_usd_to_micros() {
    assert_eq!(parse_usd_micros("3", false), Some(3_000_000));
    assert_eq!(parse_usd_micros("0.125", false), Some(125_000));
    assert_eq!(parse_usd_micros("0.000003", true), Some(3_000_000));
    assert_eq!(
        parse_usd_micros("0.0000000833333333333333", true),
        Some(83_333)
    );
    assert_eq!(parse_usd_micros("0.0000000000005", true), Some(1));
    assert_eq!(parse_usd_micros("0", false), Some(0));

    for text in ["", "+1", "-1", "1e3", " 1", ".", "1.2.3"] {
        assert_eq!(parse_usd_micros(text, false), None, "{text}");
    }
    assert_eq!(parse_usd_micros("18446744073709551616", false), None);
}

#[test]
fn bundled_files_are_valid_and_match_their_provider() {
    let files = [
        (
            Provider::Anthropic,
            include_str!("../../data/prices/anthropic.json"),
        ),
        (
            Provider::OpenAi,
            include_str!("../../data/prices/openai.json"),
        ),
        (
            Provider::Gemini,
            include_str!("../../data/prices/gemini.json"),
        ),
        (
            Provider::DeepSeek,
            include_str!("../../data/prices/deepseek.json"),
        ),
    ];

    for (provider, text) in files {
        let raw: RawTable = serde_json::from_str(text).unwrap();
        assert_eq!(raw.provider.parse::<Provider>().unwrap(), provider);
        assert_valid_date(&raw.as_of);
        assert!(!raw.models.is_empty());

        let mut ids = HashSet::new();
        for model in raw.models {
            assert_ne!(model.id, "");
            assert!(ids.insert(model.id));
            assert!(parse_usd_micros(&model.input_per_million, false).is_some());
            assert!(parse_usd_micros(&model.output_per_million, false).is_some());
            assert!(
                model
                    .cache_read_per_million
                    .as_deref()
                    .is_none_or(|value| parse_usd_micros(value, false).is_some())
            );
            assert_valid_detail(model.tools.as_deref());
            assert_valid_detail(model.images.as_deref());
            assert_valid_detail(model.reasoning.as_deref());
            assert!(
                model
                    .cache_write_per_million
                    .as_deref()
                    .is_none_or(|value| parse_usd_micros(value, false).is_some())
            );
        }
        assert!(!table_for(provider).unwrap().rows.is_empty());
    }
}

fn assert_valid_detail(value: Option<&str>) {
    assert!(value.is_none_or(|value| matches!(value, "yes" | "no")));
}

fn assert_valid_date(value: &str) {
    assert!(value.len() == 10 && value.as_bytes()[4] == b'-' && value.as_bytes()[7] == b'-');
}

#[test]
fn returns_bundled_prices_and_dates() {
    let cases = [
        (
            Provider::Anthropic,
            "claude-fable-5-2",
            8_000_000,
            40_000_000,
            Some(200_000),
            Some(10_000_000),
        ),
        (
            Provider::OpenAi,
            "gpt-6.2-astra",
            8_000_000,
            40_000_000,
            Some(400_000),
            None,
        ),
        (
            Provider::Gemini,
            "gemini-4-hydrogen",
            1_500_000,
            7_500_000,
            Some(75_000),
            None,
        ),
        (
            Provider::DeepSeek,
            "deepseek-flash",
            300_000,
            1_200_000,
            Some(6_000),
            None,
        ),
    ];

    for case in cases {
        assert_bundled_case(case);
    }
    assert!(bundled_price(Provider::Anthropic, "gpt-6.2-astra").is_none());
    assert!(bundled_price(Provider::OpenRouter, "any-model").is_none());
    assert_eq!(bundled_as_of(Provider::OpenRouter), None);
}

#[test]
fn exact_model_rows_beat_snapshot_rows() {
    let base = PriceRow {
        id: "model".to_owned(),
        price: ModelPrice {
            input_micros: 1,
            output_micros: 2,
            cache_read_micros: None,
            cache_write_micros: None,
            source: PriceSource::Bundled,
        },
        details: BundledDetails {
            context_window: None,
            max_output_tokens: None,
            caps: ModelCaps::UNKNOWN,
        },
    };
    let exact = PriceRow {
        id: "model-2026-11-19".to_owned(),
        price: ModelPrice {
            input_micros: 3,
            output_micros: 4,
            ..base.price
        },
        ..base
    };
    assert_eq!(
        find_row(&[base, exact], "model-2026-11-19").map(|row| row.price.input_micros),
        Some(3)
    );
}

#[test]
fn exact_model_rows_beat_snapshot_details() {
    let base = PriceRow {
        id: "model".to_owned(),
        price: ModelPrice {
            input_micros: 1,
            output_micros: 2,
            cache_read_micros: None,
            cache_write_micros: None,
            source: PriceSource::Bundled,
        },
        details: BundledDetails {
            context_window: Some(1),
            max_output_tokens: None,
            caps: ModelCaps::UNKNOWN,
        },
    };
    let exact = PriceRow {
        id: "model-2026-11-19".to_owned(),
        details: BundledDetails {
            context_window: Some(2),
            ..base.details
        },
        ..base
    };

    assert_eq!(
        find_row(&[base, exact], "model-2026-11-19").map(|row| row.details.context_window),
        Some(Some(2))
    );
}

fn assert_bundled_case(case: (Provider, &str, u64, u64, Option<u64>, Option<u64>)) {
    let (provider, id, input, output, cache_read, cache_write) = case;
    let expected = Some(ModelPrice {
        input_micros: input,
        output_micros: output,
        cache_read_micros: cache_read,
        cache_write_micros: cache_write,
        source: PriceSource::Bundled,
    });
    assert_eq!(bundled_price(provider, id), expected);
    assert_eq!(bundled_price(provider, &format!("{id}-20261101")), expected);
    assert_eq!(
        bundled_price(provider, &format!("{id}-2026-11-01")),
        expected
    );
    assert_eq!(bundled_as_of(provider), Some("2026-11-19"));
    assert!(bundled_price(provider, &format!("{id}-preview")).is_none());
    assert!(bundled_price(provider, &format!("{id}x")).is_none());
}

#[test]
fn calculates_cost_with_cache_prices() {
    let price = ModelPrice {
        input_micros: 2_000_000,
        output_micros: 4_000_000,
        cache_read_micros: Some(500_000),
        cache_write_micros: Some(1_000_000),
        source: PriceSource::Provider,
    };
    let usage = ResponseUsage {
        input_tokens: 1_000,
        output_tokens: 500,
        cache_read_tokens: 100,
        cache_write_tokens: 20,
    };
    assert_eq!(cost_micros(&price, &usage), 4_070);
}

#[test]
fn calculates_cost_with_fallback_rounding_and_saturation() {
    let price = ModelPrice {
        input_micros: u64::MAX,
        output_micros: 1,
        cache_read_micros: None,
        cache_write_micros: None,
        source: PriceSource::Provider,
    };
    let rounding_price = ModelPrice {
        input_micros: 500_000,
        ..price
    };
    let one_token = ResponseUsage {
        input_tokens: 1,
        ..ResponseUsage::default()
    };
    assert_eq!(cost_micros(&rounding_price, &one_token), 1);
    assert_eq!(cost_micros(&rounding_price, &ResponseUsage::default()), 0);
    assert_eq!(
        cost_micros(
            &price,
            &ResponseUsage {
                input_tokens: u64::MAX,
                ..ResponseUsage::default()
            }
        ),
        u64::MAX
    );
}

#[test]
fn missing_cache_prices_charge_the_input_rate_for_cache_tokens() {
    let price = ModelPrice {
        input_micros: 2_000_000,
        output_micros: 4_000_000,
        cache_read_micros: None,
        cache_write_micros: None,
        source: PriceSource::Bundled,
    };
    let usage = ResponseUsage {
        cache_read_tokens: 100,
        cache_write_tokens: 50,
        ..ResponseUsage::default()
    };
    assert_eq!(cost_micros(&price, &usage), 300);
}

#[test]
fn cost_below_half_a_micro_rounds_down() {
    let price = ModelPrice {
        input_micros: 499_999,
        output_micros: 0,
        cache_read_micros: None,
        cache_write_micros: None,
        source: PriceSource::Bundled,
    };
    let usage = ResponseUsage {
        input_tokens: 1,
        ..ResponseUsage::default()
    };
    assert_eq!(cost_micros(&price, &usage), 0);
}

#[test]
fn snapshot_matching_accepts_date_shapes_without_calendar_validation() {
    assert!(is_snapshot_id("model", "model-00000000"));
    assert!(is_snapshot_id("model", "model-2026-99-99"));
    assert!(!is_snapshot_id("model", "model-2026-1-01"));
    assert!(!is_snapshot_id("model", "model-2026-11-19-extra"));
}

#[test]
fn bundled_details_map_no_and_unknown_and_drop_zero_context() {
    let text = r#"{"provider":"openai","as_of":"2026-11-19","models":[{"id":"model","input_per_million":"1","output_per_million":"2","context_window":0,"tools":"no","images":"maybe"}]}"#;
    let table = load_table(text, Provider::OpenAi).unwrap();
    let details = find_row(&table.rows, "model").unwrap().details;

    assert_eq!(details.context_window, None);
    assert_eq!(details.max_output_tokens, None);
    assert_eq!(details.caps.tools, Support::No);
    assert_eq!(details.caps.images, Support::Unknown);
    assert_eq!(details.caps.reasoning, Support::Unknown);
}

#[test]
fn provider_mismatch_yields_no_price_table() {
    let text = r#"{"provider":"gemini","as_of":"2026-11-19","models":[{"id":"model","input_per_million":"1","output_per_million":"2"}]}"#;
    assert!(load_table(text, Provider::OpenAi).is_none());
}
