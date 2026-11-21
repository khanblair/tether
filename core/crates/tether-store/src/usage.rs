//! Usage history and budget persistence.

use rusqlite::params;
use tether_types::{Provider, SessionId, UsageSnapshot};

use crate::{Store, StoreError};

/// Aggregated token and cost totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageTotals {
    /// Input tokens, including cached input tokens.
    pub tokens_in: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Cost in USD micros.
    pub cost_micros: u64,
}

/// Usage grouped by UTC day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayUsage {
    /// UTC date in `YYYY-MM-DD` form.
    pub day: String,
    /// Usage totals for the day.
    pub totals: UsageTotals,
}

/// Usage grouped by project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUsage {
    /// Project id, retained after project deletion.
    pub project_id: String,
    /// Current project name, or empty after deletion.
    pub project_name: String,
    /// Usage totals for the project.
    pub totals: UsageTotals,
}

/// Usage grouped by provider and model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    /// Provider name.
    pub provider: Provider,
    /// Provider model id.
    pub model_id: String,
    /// Usage totals for the model.
    pub totals: UsageTotals,
}

/// Usage windows ending on one display day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageWindows {
    /// Totals for the selected day.
    pub today: UsageTotals,
    /// Totals for the selected day and six preceding days.
    pub last_7_days: UsageTotals,
    /// Totals for the selected day and 29 preceding days.
    pub last_30_days: UsageTotals,
}

/// One priced provider response to commit to session and daily totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseUsageRecord {
    /// Session receiving the response.
    pub session_id: SessionId,
    /// Provider that served the response.
    pub provider: Provider,
    /// Provider model id.
    pub model_id: String,
    /// Project receiving the usage.
    pub project_id: String,
    /// UTC date of the response.
    pub day: String,
    /// Priced token totals.
    pub usage: UsageSnapshot,
    /// Whether the model price was known.
    pub price_known: bool,
    /// Session update timestamp.
    pub now_ms: i64,
}

impl Store {
    /// Adds one provider response to the session and its UTC daily row atomically.
    ///
    /// `day` is the UTC date of the response. This transaction means a crash cannot
    /// commit one total without the other.
    ///
    /// # Errors
    /// Returns an error when a counter overflows, the session is absent, or SQLite fails.
    pub fn record_response_usage(
        &mut self,
        record: &ResponseUsageRecord,
    ) -> Result<(), StoreError> {
        let input = sql_count(record.usage.tokens_in)?;
        let output = sql_count(record.usage.tokens_out)?;
        let cost = sql_count(record.usage.cost_micros)?;
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE sessions SET tokens_in = tokens_in + ?2, tokens_out = tokens_out + ?3,
             cost_micros = cost_micros + ?4, price_known = price_known AND ?5,
             updated_at = ?6 WHERE id = ?1",
            params![
                record.session_id.as_str(),
                input,
                output,
                cost,
                record.price_known,
                record.now_ms
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::MissingSessionRow);
        }
        tx.execute(
            "INSERT INTO usage_daily
             (day, provider, model_id, project_id, tokens_in, tokens_out, cost_micros)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(day, provider, model_id, project_id) DO UPDATE SET
               tokens_in = tokens_in + excluded.tokens_in,
               tokens_out = tokens_out + excluded.tokens_out,
               cost_micros = cost_micros + excluded.cost_micros",
            params![
                record.day.as_str(),
                record.provider.as_str(),
                record.model_id.as_str(),
                record.project_id.as_str(),
                input,
                output,
                cost
            ],
        )?;
        Ok(tx.commit()?)
    }

    /// Returns usage grouped by UTC day in inclusive date order.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid counter or fails to query.
    pub fn usage_by_day(&self, from_day: &str, to_day: &str) -> Result<Vec<DayUsage>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT day, SUM(tokens_in), SUM(tokens_out), SUM(cost_micros)
             FROM usage_daily WHERE day BETWEEN ?1 AND ?2 GROUP BY day ORDER BY day",
        )?;
        let rows = statement.query_map([from_day, to_day], read_usage_row)?;
        rows.map(|row| row.map_err(StoreError::from).and_then(day_usage))
            .collect()
    }

    /// Returns usage grouped by project in id order.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid counter or fails to query.
    pub fn usage_by_project(
        &self,
        from_day: &str,
        to_day: &str,
    ) -> Result<Vec<ProjectUsage>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT usage_daily.project_id, COALESCE(projects.name, ''),
                    SUM(tokens_in), SUM(tokens_out), SUM(cost_micros)
             FROM usage_daily LEFT JOIN projects ON projects.id = usage_daily.project_id
             WHERE day BETWEEN ?1 AND ?2
             GROUP BY usage_daily.project_id, projects.name ORDER BY usage_daily.project_id",
        )?;
        let rows = statement.query_map([from_day, to_day], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        rows.map(|row| row.map_err(StoreError::from).and_then(project_usage))
            .collect()
    }

    /// Returns usage grouped by provider and model in provider/model order.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid counter or provider.
    pub fn usage_by_model(
        &self,
        from_day: &str,
        to_day: &str,
    ) -> Result<Vec<ModelUsage>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT provider, model_id, SUM(tokens_in), SUM(tokens_out), SUM(cost_micros)
             FROM usage_daily WHERE day BETWEEN ?1 AND ?2
             GROUP BY provider, model_id ORDER BY provider, model_id",
        )?;
        let rows = statement.query_map([from_day, to_day], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        rows.map(|row| row.map_err(StoreError::from).and_then(model_usage))
            .collect()
    }

    /// Returns totals for one UTC day.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid counter or fails to query.
    pub fn usage_totals(&self, day: &str) -> Result<UsageTotals, StoreError> {
        self.range_totals(day, day)
    }

    /// Returns today, seven-day, and thirty-day totals for a selected UTC date.
    ///
    /// The caller may choose a local display date. Stored rows remain UTC dates.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid counter or fails to query.
    pub fn usage_windows(&self, today: &str) -> Result<UsageWindows, StoreError> {
        Ok(UsageWindows {
            today: self.range_totals(today, today)?,
            last_7_days: self.range_totals(&usage_day_offset(today, -6)?, today)?,
            last_30_days: self.range_totals(&usage_day_offset(today, -29)?, today)?,
        })
    }

    fn range_totals(&self, from_day: &str, to_day: &str) -> Result<UsageTotals, StoreError> {
        let row = self.conn.query_row(
            "SELECT COALESCE(SUM(tokens_in), 0), COALESCE(SUM(tokens_out), 0),
                    COALESCE(SUM(cost_micros), 0) FROM usage_daily WHERE day BETWEEN ?1 AND ?2",
            [from_day, to_day],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        totals(row)
    }
}

fn read_usage_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, i64, i64, i64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn day_usage(row: (String, i64, i64, i64)) -> Result<DayUsage, StoreError> {
    let (day, input, output, cost) = row;
    Ok(DayUsage {
        day,
        totals: totals((input, output, cost))?,
    })
}

fn project_usage(row: (String, String, i64, i64, i64)) -> Result<ProjectUsage, StoreError> {
    let (project_id, project_name, input, output, cost) = row;
    Ok(ProjectUsage {
        project_id,
        project_name,
        totals: totals((input, output, cost))?,
    })
}

fn model_usage(row: (String, String, i64, i64, i64)) -> Result<ModelUsage, StoreError> {
    let (provider, model_id, input, output, cost) = row;
    Ok(ModelUsage {
        provider: provider.parse().map_err(|_| StoreError::InvalidModelEnum {
            field: "provider",
            value: provider,
        })?,
        model_id,
        totals: totals((input, output, cost))?,
    })
}

fn totals(row: (i64, i64, i64)) -> Result<UsageTotals, StoreError> {
    Ok(UsageTotals {
        tokens_in: decode_i64(row.0)?,
        tokens_out: decode_i64(row.1)?,
        cost_micros: decode_i64(row.2)?,
    })
}

fn sql_count(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::SessionCounterOverflow)
}

fn decode_i64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::SessionCounterOverflow)
}

/// Moves a UTC date by a number of calendar days.
///
/// # Errors
/// Returns an error when `day` is not a date in `YYYY-MM-DD` form.
pub fn usage_day_offset(day: &str, offset: i32) -> Result<String, StoreError> {
    let parts = day
        .split('-')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StoreError::InvalidDay)?;
    let [year, month, date]: [i64; 3] = parts.try_into().map_err(|_| StoreError::InvalidDay)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&date) {
        return Err(StoreError::InvalidDay);
    }
    let days = days_from_civil(year, month, date) + i64::from(offset);
    let (year, month, date) = civil_from_days(days);
    Ok(format!("{year:04}-{month:02}-{date:02}"))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let month_part = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_part + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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
