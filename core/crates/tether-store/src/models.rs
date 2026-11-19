//! Model catalog cache queries.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use tether_types::{ModelCaps, ModelInfo, ModelPrice, Provider, Support};

use crate::{Store, StoreError};

/// How long a cached list stays fresh: 24 hours, in milliseconds.
pub const CATALOG_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// Returns whether a catalog timestamp is fresh at `now_ms`.
#[must_use]
pub fn catalog_is_fresh(fetched_at_ms: i64, now_ms: i64) -> bool {
    now_ms
        .checked_sub(fetched_at_ms)
        .is_some_and(|age| (0..CATALOG_TTL_MS).contains(&age))
}

impl Store {
    /// Replaces the whole cached list for `provider` and stamps it `fetched_at_ms`.
    ///
    /// # Errors
    /// Returns an error when a model belongs to another provider, a price overflows
    /// SQLite's integer type, capability JSON cannot be encoded, or SQLite fails.
    pub fn replace_models(
        &mut self,
        provider: Provider,
        models: &[ModelInfo],
        fetched_at_ms: i64,
    ) -> Result<(), StoreError> {
        let rows = prepare_rows(provider, models)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM models WHERE provider = ?1",
            [provider.as_str()],
        )?;
        insert_rows(&tx, provider, &rows)?;
        tx.execute(
            "INSERT INTO models_fetch (provider, fetched_at) VALUES (?1, ?2)
             ON CONFLICT(provider) DO UPDATE SET fetched_at = excluded.fetched_at",
            params![provider.as_str(), fetched_at_ms],
        )?;
        Ok(tx.commit()?)
    }

    /// Returns the cached list for `provider` in its replacement order.
    ///
    /// # Errors
    /// Returns an error when SQLite fails or a cached row has invalid stored data.
    pub fn models(&self, provider: Provider) -> Result<Vec<ModelInfo>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT provider, model_id, display_name, context_window, max_output_tokens,
                    input_price_micros, output_price_micros, cache_read_price_micros,
                    cache_write_price_micros, price_source, caps_json
             FROM models WHERE provider = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([provider.as_str()], read_row)?;
        rows.map(|row| row.map_err(StoreError::from).and_then(decode_row))
            .collect::<Result<_, _>>()
    }

    /// Returns when `provider`'s cached list was fetched, or `None` if it is absent.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn models_fetched_at(&self, provider: Provider) -> Result<Option<i64>, StoreError> {
        self.conn
            .query_row(
                "SELECT fetched_at FROM models_fetch WHERE provider = ?1",
                [provider.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Returns how many cached models `provider` has. Rows are counted, not decoded.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn models_count(&self, provider: Provider) -> Result<u32, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM models WHERE provider = ?1",
            [provider.as_str()],
            |row| row.get(0),
        )?)
    }

    /// Removes `provider`'s cached list and its fetch time in one transaction.
    /// Other providers are untouched.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn clear_models(&mut self, provider: Provider) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM models WHERE provider = ?1",
            [provider.as_str()],
        )?;
        tx.execute(
            "DELETE FROM models_fetch WHERE provider = ?1",
            [provider.as_str()],
        )?;
        Ok(tx.commit()?)
    }
}

type StoredModel = (
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    String,
);

fn prepare_rows(
    provider: Provider,
    models: &[ModelInfo],
) -> Result<Vec<(ModelInfo, String)>, StoreError> {
    let mut seen = HashSet::new();
    models
        .iter()
        .filter(|model| seen.insert(model.id.clone()))
        .map(|model| {
            if model.provider != provider {
                return Err(StoreError::ModelProviderMismatch {
                    expected: provider.as_str(),
                    found: model.provider.as_str(),
                });
            }
            Ok((model.clone(), caps_json(model.caps)?))
        })
        .collect()
}

fn insert_rows(
    tx: &rusqlite::Transaction<'_>,
    provider: Provider,
    rows: &[(ModelInfo, String)],
) -> Result<(), StoreError> {
    let mut insert = tx.prepare(
        "INSERT INTO models (
            provider, model_id, display_name, context_window, max_output_tokens,
            input_price_micros, output_price_micros, cache_read_price_micros,
            cache_write_price_micros, price_source, caps_json, position
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for (position, (model, caps)) in rows.iter().enumerate() {
        let price = model.price.map(sql_price).transpose()?;
        let position = i64::try_from(position).map_err(|_| StoreError::PositionOverflow)?;
        insert.execute(params![
            provider.as_str(),
            model.id,
            model.display_name,
            positive_i64(model.context_window),
            positive_i64(model.max_output_tokens),
            price.as_ref().map(|p| p.0),
            price.as_ref().map(|p| p.1),
            price.as_ref().and_then(|p| p.2),
            price.as_ref().and_then(|p| p.3),
            price.as_ref().map(|p| p.4),
            caps,
            position,
        ])?;
    }
    Ok(())
}

type SqlPrice = (i64, i64, Option<i64>, Option<i64>, &'static str);

fn sql_price(price: ModelPrice) -> Result<SqlPrice, StoreError> {
    Ok((
        sql_amount("input", price.input_micros)?,
        sql_amount("output", price.output_micros)?,
        price
            .cache_read_micros
            .map(|value| sql_amount("cache_read", value))
            .transpose()?,
        price
            .cache_write_micros
            .map(|value| sql_amount("cache_write", value))
            .transpose()?,
        price.source.as_str(),
    ))
}

fn sql_amount(field: &'static str, value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::ModelPriceOverflow { field, value })
}

fn positive_i64(value: Option<u32>) -> Option<i64> {
    value.filter(|&value| value > 0).map(i64::from)
}

fn caps_json(caps: ModelCaps) -> Result<String, StoreError> {
    Ok(serde_json::to_string(&json!({
        "tools": caps.tools.as_str(),
        "images": caps.images.as_str(),
        "reasoning": caps.reasoning.as_str(),
    }))?)
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredModel> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
    ))
}

fn decode_row(row: StoredModel) -> Result<ModelInfo, StoreError> {
    let (
        provider,
        id,
        display_name,
        context_window,
        max_output_tokens,
        input,
        output,
        cache_read,
        cache_write,
        source,
        caps,
    ) = row;
    let provider = parse_provider(provider)?;
    let price = decode_price(input, output, cache_read, cache_write, source)?;
    Ok(ModelInfo {
        provider,
        id,
        display_name,
        context_window: decode_positive("context_window", context_window)?,
        max_output_tokens: decode_positive("max_output_tokens", max_output_tokens)?,
        price,
        caps: decode_caps(&caps),
    })
}

fn decode_positive(field: &'static str, value: Option<i64>) -> Result<Option<u32>, StoreError> {
    match value {
        None | Some(0) => Ok(None),
        Some(value) if value > 0 => u32::try_from(value)
            .map(Some)
            .map_err(|_| StoreError::InvalidModelValue { field, value }),
        Some(value) => Err(StoreError::InvalidModelValue { field, value }),
    }
}

fn decode_price(
    input: Option<i64>,
    output: Option<i64>,
    cache_read: Option<i64>,
    cache_write: Option<i64>,
    source: Option<String>,
) -> Result<Option<ModelPrice>, StoreError> {
    let Some(input) = input else { return Ok(None) };
    let output = output.ok_or_else(|| StoreError::InvalidModelValue {
        field: "output_price_micros",
        value: 0,
    })?;
    let source = source.ok_or_else(|| StoreError::InvalidModelEnum {
        field: "price_source",
        value: "NULL".to_owned(),
    })?;
    Ok(Some(ModelPrice {
        input_micros: decode_amount("input_price_micros", input)?,
        output_micros: decode_amount("output_price_micros", output)?,
        cache_read_micros: decode_amount_option("cache_read_price_micros", cache_read)?,
        cache_write_micros: decode_amount_option("cache_write_price_micros", cache_write)?,
        source: source.parse().map_err(|_| StoreError::InvalidModelEnum {
            field: "price_source",
            value: source,
        })?,
    }))
}

fn decode_amount(field: &'static str, value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::InvalidModelValue { field, value })
}

fn decode_amount_option(
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u64>, StoreError> {
    value.map(|value| decode_amount(field, value)).transpose()
}

fn parse_provider(value: String) -> Result<Provider, StoreError> {
    value.parse().map_err(|_| StoreError::InvalidModelEnum {
        field: "provider",
        value,
    })
}

fn decode_caps(text: &str) -> ModelCaps {
    let Ok(Value::Object(object)) = serde_json::from_str(text) else {
        return ModelCaps::UNKNOWN;
    };
    ModelCaps {
        tools: decode_support(object.get("tools")),
        images: decode_support(object.get("images")),
        reasoning: decode_support(object.get("reasoning")),
    }
}

fn decode_support(value: Option<&Value>) -> Support {
    value
        .and_then(Value::as_str)
        .and_then(|text| text.parse().ok())
        .unwrap_or(Support::Unknown)
}
