//! Provider response pricing and UTC day helpers.

use tether_providers::{ResponseUsage, bundled_price, cost_micros};
use tether_store::{Store, StoreError};
use tether_types::{SessionId, UsageSnapshot};

/// A response's priced totals and whether its price was known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PricedUsage {
    /// Token and cost totals.
    pub snapshot: UsageSnapshot,
    /// Whether a catalog or bundled price was used.
    pub price_known: bool,
}

/// Prices one provider response from the cached catalog, then bundled prices.
///
/// # Errors
/// Returns an error when the session or model catalog cannot be read.
pub(crate) fn price_response(
    store: &Store,
    session_id: &SessionId,
    model_id: &str,
    response: ResponseUsage,
) -> Result<PricedUsage, StoreError> {
    let session = store
        .load_session(session_id)?
        .ok_or(StoreError::MissingSessionRow)?;
    let price = store
        .model_price(session.provider, model_id)?
        .or_else(|| bundled_price(session.provider, model_id));
    let snapshot = UsageSnapshot {
        tokens_in: response
            .input_tokens
            .saturating_add(response.cache_read_tokens)
            .saturating_add(response.cache_write_tokens),
        tokens_out: response.output_tokens,
        cost_micros: price.map_or(0, |value| cost_micros(&value, &response)),
    };
    Ok(PricedUsage {
        snapshot,
        price_known: price.is_some(),
    })
}

/// Formats a Unix millisecond timestamp as its UTC date.
#[must_use]
pub(crate) fn utc_day_from_ms(now_ms: i64) -> String {
    let days = now_ms.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = (if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }) / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use tether_store::Store;
    use tether_types::{ModelCaps, ModelInfo, ModelPrice, PriceSource, Provider};

    use super::*;

    fn seeded(model: &str) -> (Store, SessionId) {
        let mut store = Store::open_in_memory().unwrap();
        let project = store.create_project("Test", 1).unwrap().id;
        let session = store
            .create_session(
                &project,
                &tether_types::ModelRef {
                    provider: Provider::OpenAi,
                    model_id: model.into(),
                },
                tether_types::PermissionMode::Ask,
                1,
            )
            .unwrap();
        (store, session.id)
    }

    #[test]
    fn cached_token_prices_use_the_provider_price_shape() {
        let (mut store, session) = seeded("cached-model");
        store
            .replace_models(
                Provider::OpenAi,
                &[ModelInfo {
                    provider: Provider::OpenAi,
                    id: "cached-model".into(),
                    display_name: "Cached".into(),
                    context_window: None,
                    max_output_tokens: None,
                    price: Some(ModelPrice {
                        input_micros: 2_000_000,
                        output_micros: 4_000_000,
                        cache_read_micros: Some(500_000),
                        cache_write_micros: Some(1_000_000),
                        source: PriceSource::Provider,
                    }),
                    caps: ModelCaps::UNKNOWN,
                }],
                1,
            )
            .unwrap();
        let priced = price_response(
            &store,
            &session,
            "cached-model",
            ResponseUsage {
                input_tokens: 10,
                output_tokens: 2,
                cache_read_tokens: 3,
                cache_write_tokens: 4,
            },
        )
        .unwrap();
        assert_eq!(priced.snapshot.cost_micros, 34);
        assert!(priced.price_known);
    }

    #[test]
    fn unknown_models_have_zero_cost_and_unknown_price() {
        let (store, session) = seeded("unknown-model");
        let priced = price_response(
            &store,
            &session,
            "unknown-model",
            ResponseUsage {
                input_tokens: 10,
                output_tokens: 2,
                ..ResponseUsage::default()
            },
        )
        .unwrap();
        assert_eq!(priced.snapshot.cost_micros, 0);
        assert!(!priced.price_known);
    }
}
