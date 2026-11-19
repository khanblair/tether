use std::path::Path;

use serde_json::{Value, json};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider, Support};

use crate::{CATALOG_TTL_MS, Store, StoreError, catalog_is_fresh};

fn model(provider: Provider, id: &str) -> ModelInfo {
    ModelInfo {
        provider,
        id: id.to_owned(),
        display_name: id.to_owned(),
        context_window: Some(1),
        max_output_tokens: Some(1),
        price: None,
        caps: ModelCaps::UNKNOWN,
    }
}

fn full_model() -> ModelInfo {
    ModelInfo {
        provider: Provider::Anthropic,
        id: "claude-π".to_owned(),
        display_name: "多言語モデル".to_owned(),
        context_window: Some(u32::MAX),
        max_output_tokens: Some(0),
        price: Some(ModelPrice {
            input_micros: 11,
            output_micros: 22,
            cache_read_micros: Some(3),
            cache_write_micros: Some(4),
            source: PriceSource::Bundled,
        }),
        caps: ModelCaps {
            tools: Support::Yes,
            images: Support::Unknown,
            reasoning: Support::No,
        },
    }
}

#[test]
fn model_fields_and_order_round_trip() {
    let mut store = Store::open_in_memory().unwrap();
    let first = full_model();
    let mut stored_first = first.clone();
    stored_first.max_output_tokens = None;
    let second = ModelInfo {
        provider: Provider::Anthropic,
        id: "plain".into(),
        display_name: "Plain".into(),
        context_window: None,
        max_output_tokens: None,
        price: Some(ModelPrice {
            input_micros: 5,
            output_micros: 6,
            cache_read_micros: None,
            cache_write_micros: None,
            source: PriceSource::Provider,
        }),
        caps: ModelCaps::UNKNOWN,
    };

    store
        .replace_models(Provider::Anthropic, &[second.clone(), first.clone()], 42)
        .unwrap();

    let caps_json: String = store
        .conn
        .query_row(
            "SELECT caps_json FROM models WHERE model_id = 'claude-π'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&caps_json).unwrap(),
        json!({"tools": "yes", "images": "unknown", "reasoning": "no"})
    );
    assert_eq!(
        store.models(Provider::Anthropic).unwrap(),
        [second, stored_first]
    );
    assert_eq!(
        store.models_fetched_at(Provider::Anthropic).unwrap(),
        Some(42)
    );
}

#[test]
fn duplicate_ids_keep_the_first_and_other_providers_are_untouched() {
    let mut store = Store::open_in_memory().unwrap();
    let first = model(Provider::OpenAi, "same");
    let mut later = first.clone();
    later.display_name = "later".into();
    store
        .replace_models(Provider::OpenAi, &[first.clone(), later], 1)
        .unwrap();
    store
        .replace_models(Provider::Gemini, &[model(Provider::Gemini, "g")], 2)
        .unwrap();

    assert_eq!(store.models(Provider::OpenAi).unwrap(), [first]);
    assert_eq!(store.models(Provider::Gemini).unwrap().len(), 1);
}

#[test]
fn empty_replacement_clears_models_but_records_fetch_time() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .replace_models(Provider::DeepSeek, &[model(Provider::DeepSeek, "d")], 1)
        .unwrap();
    store.replace_models(Provider::DeepSeek, &[], 2).unwrap();

    assert_eq!(store.models(Provider::DeepSeek).unwrap().len(), 0);
    assert_eq!(
        store.models_fetched_at(Provider::DeepSeek).unwrap(),
        Some(2)
    );
    assert_eq!(store.models_fetched_at(Provider::OpenRouter).unwrap(), None);
}

#[test]
fn overflowing_price_leaves_the_old_list_untouched() {
    let mut store = Store::open_in_memory().unwrap();
    let old = model(Provider::Anthropic, "old");
    store
        .replace_models(Provider::Anthropic, std::slice::from_ref(&old), 1)
        .unwrap();
    let mut overflowing = model(Provider::Anthropic, "new");
    overflowing.price = Some(ModelPrice {
        input_micros: u64::MAX,
        output_micros: 1,
        cache_read_micros: None,
        cache_write_micros: None,
        source: PriceSource::Provider,
    });

    let error = store
        .replace_models(Provider::Anthropic, &[overflowing], 2)
        .unwrap_err();

    assert!(matches!(
        error,
        StoreError::ModelPriceOverflow { field: "input", .. }
    ));
    assert_eq!(store.models(Provider::Anthropic).unwrap(), [old]);
    assert_eq!(
        store.models_fetched_at(Provider::Anthropic).unwrap(),
        Some(1)
    );
}

#[test]
fn file_backed_store_reopens_with_the_cached_list() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.db");
    let expected = full_model();
    let mut stored_expected = expected.clone();
    stored_expected.max_output_tokens = None;
    {
        let mut store = Store::open(Path::new(&path)).unwrap();
        store
            .replace_models(Provider::Anthropic, std::slice::from_ref(&expected), 77)
            .unwrap();
    }

    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.models(Provider::Anthropic).unwrap(),
        [stored_expected]
    );
    assert_eq!(
        store.models_fetched_at(Provider::Anthropic).unwrap(),
        Some(77)
    );
}

#[test]
fn freshness_rejects_future_and_expired_timestamps_without_overflow() {
    assert!(catalog_is_fresh(1, 1 + CATALOG_TTL_MS - 1));
    assert!(!catalog_is_fresh(1, 1 + CATALOG_TTL_MS));
    assert!(!catalog_is_fresh(1, 1 + CATALOG_TTL_MS + 1));
    assert!(!catalog_is_fresh(2, 1));
    assert!(catalog_is_fresh(-1, -1));
    assert!(catalog_is_fresh(-1, 0));
    assert!(!catalog_is_fresh(i64::MIN, i64::MAX));
}

#[test]
fn malformed_capabilities_become_unknown() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .replace_models(Provider::Gemini, &[model(Provider::Gemini, "g")], 1)
        .unwrap();
    for (text, expected) in [
        ("{}", ModelCaps::UNKNOWN),
        (r#"{"tools":"maybe"}"#, ModelCaps::UNKNOWN),
        ("[]", ModelCaps::UNKNOWN),
    ] {
        store
            .conn
            .execute("UPDATE models SET caps_json = ?1", [text])
            .unwrap();
        assert_eq!(store.models(Provider::Gemini).unwrap()[0].caps, expected);
    }
}

#[test]
fn invalid_stored_price_source_is_an_error() {
    let store = Store::open_in_memory().unwrap();
    store
        .conn
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO models (
                provider, model_id, display_name, input_price_micros, output_price_micros,
                price_source, caps_json, position
             ) VALUES ('anthropic', 'm', 'M', 1, 1, 'bogus', '{}', 0)",
            [],
        )
        .unwrap();
    let error = store.models(Provider::Anthropic).unwrap_err();
    assert!(matches!(
        error,
        StoreError::InvalidModelEnum {
            field: "price_source",
            ..
        }
    ));
}

#[test]
fn models_only_decodes_the_requested_provider() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .conn
        .execute(
            "INSERT INTO models (provider, model_id, display_name, context_window, caps_json, position)
             VALUES ('openai', 'too-large', 'Too large', 5000000000, '{}', 0)",
            [],
        )
        .unwrap();
    let expected = model(Provider::Anthropic, "claude");
    store
        .replace_models(Provider::Anthropic, std::slice::from_ref(&expected), 1)
        .unwrap();

    assert_eq!(store.models(Provider::Anthropic).unwrap(), [expected]);
    assert!(matches!(
        store.models(Provider::OpenAi),
        Err(StoreError::InvalidModelValue {
            field: "context_window",
            value: 5_000_000_000,
        })
    ));
}

#[test]
fn every_support_value_round_trips_in_each_capability() {
    let mut store = Store::open_in_memory().unwrap();
    let values = [Support::Yes, Support::No, Support::Unknown];
    let expected = values
        .iter()
        .enumerate()
        .map(|(index, &value)| ModelInfo {
            provider: Provider::Gemini,
            id: format!("m{index}"),
            display_name: format!("M{index}"),
            context_window: None,
            max_output_tokens: None,
            price: None,
            caps: ModelCaps {
                tools: value,
                images: values[(index + 1) % 3],
                reasoning: values[(index + 2) % 3],
            },
        })
        .collect::<Vec<_>>();
    store
        .replace_models(Provider::Gemini, &expected, 1)
        .unwrap();

    assert_eq!(store.models(Provider::Gemini).unwrap(), expected);
}

#[test]
fn models_count_counts_only_the_named_provider() {
    let mut store = Store::open_in_memory().unwrap();
    assert_eq!(store.models_count(Provider::DeepSeek).unwrap(), 0);

    store
        .replace_models(
            Provider::Anthropic,
            &[
                model(Provider::Anthropic, "a1"),
                model(Provider::Anthropic, "a2"),
            ],
            1,
        )
        .unwrap();
    store
        .replace_models(Provider::OpenAi, &[model(Provider::OpenAi, "o1")], 1)
        .unwrap();

    assert_eq!(store.models_count(Provider::Anthropic).unwrap(), 2);
    assert_eq!(store.models_count(Provider::OpenAi).unwrap(), 1);
    assert_eq!(store.models_count(Provider::DeepSeek).unwrap(), 0);
}

#[test]
fn clear_models_removes_one_provider_and_keeps_the_others() {
    let mut store = Store::open_in_memory().unwrap();
    let openai = model(Provider::OpenAi, "o1");
    store
        .replace_models(
            Provider::Anthropic,
            &[
                model(Provider::Anthropic, "a1"),
                model(Provider::Anthropic, "a2"),
            ],
            10,
        )
        .unwrap();
    store
        .replace_models(Provider::OpenAi, std::slice::from_ref(&openai), 20)
        .unwrap();

    store.clear_models(Provider::Anthropic).unwrap();

    assert_eq!(store.models(Provider::Anthropic).unwrap(), []);
    assert_eq!(store.models_count(Provider::Anthropic).unwrap(), 0);
    assert_eq!(store.models_fetched_at(Provider::Anthropic).unwrap(), None);
    assert_eq!(store.models(Provider::OpenAi).unwrap(), [openai]);
    assert_eq!(store.models_count(Provider::OpenAi).unwrap(), 1);
    assert_eq!(store.models_fetched_at(Provider::OpenAi).unwrap(), Some(20));
}

#[test]
fn clear_models_on_a_provider_never_fetched_is_a_no_op() {
    let mut store = Store::open_in_memory().unwrap();
    store.clear_models(Provider::Gemini).unwrap();

    assert_eq!(store.models_count(Provider::Gemini).unwrap(), 0);
    assert_eq!(store.models_fetched_at(Provider::Gemini).unwrap(), None);
}
