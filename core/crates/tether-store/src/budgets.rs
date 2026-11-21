//! Budget settings and session overrides.

use rusqlite::{OptionalExtension, params};
use tether_types::SessionId;

use crate::{Store, StoreError};

/// The configured daily and session limits in USD micros.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BudgetSettings {
    /// Daily limit, or no limit.
    pub daily_limit_micros: Option<u64>,
    /// Default session limit, or no limit.
    pub session_limit_micros: Option<u64>,
}

impl Store {
    /// Reads the configured budget limits, treating an absent row as no limits.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn budgets(&self) -> Result<BudgetSettings, StoreError> {
        let row: Option<(Option<i64>, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT daily_limit_micros, session_limit_micros FROM budget_settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map_or(Ok(BudgetSettings::default()), |(daily, session)| {
            Ok(BudgetSettings {
                daily_limit_micros: decode_limit(daily)?,
                session_limit_micros: decode_limit(session)?,
            })
        })
    }

    /// Saves the default daily and session limits.
    ///
    /// # Errors
    /// Returns an error when a limit is invalid or SQLite fails.
    pub fn set_budgets(&self, settings: BudgetSettings, now_ms: i64) -> Result<(), StoreError> {
        validate_limit(settings.daily_limit_micros)?;
        validate_limit(settings.session_limit_micros)?;
        self.conn.execute(
            "INSERT INTO budget_settings (id, daily_limit_micros, session_limit_micros, updated_at)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET daily_limit_micros = excluded.daily_limit_micros,
               session_limit_micros = excluded.session_limit_micros, updated_at = excluded.updated_at",
            params![
                settings.daily_limit_micros.map(budget_count).transpose()?,
                settings.session_limit_micros.map(budget_count).transpose()?,
                now_ms
            ],
        )?;
        Ok(())
    }

    /// Sets or removes one session budget override.
    ///
    /// # Errors
    /// Returns an error when a limit is invalid, the session is absent, or SQLite fails.
    pub fn set_session_budget(
        &self,
        session_id: &SessionId,
        limit_micros: Option<u64>,
    ) -> Result<(), StoreError> {
        validate_limit(limit_micros)?;
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM sessions WHERE id = ?1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(StoreError::MissingSessionRow);
        }
        if let Some(limit) = limit_micros {
            self.conn.execute(
                "INSERT INTO session_budget_overrides (session_id, limit_micros, continued_past_micros)
                 VALUES (?1, ?2, NULL)
                 ON CONFLICT(session_id) DO UPDATE SET limit_micros = excluded.limit_micros,
                   continued_past_micros = NULL",
                params![session_id.as_str(), budget_count(limit)?],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM session_budget_overrides WHERE session_id = ?1",
                [session_id.as_str()],
            )?;
        }
        Ok(())
    }

    /// Returns a session override and its continuation anchor.
    ///
    /// # Errors
    /// Returns an error when SQLite contains an invalid limit or fails.
    pub fn session_budget(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<(u64, Option<u64>)>, StoreError> {
        let row: Option<(i64, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT limit_micros, continued_past_micros FROM session_budget_overrides
                 WHERE session_id = ?1",
                [session_id.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()?;
        row.map(|(limit, continued)| {
            Ok((decode_i64(limit)?, continued.map(decode_i64).transpose()?))
        })
        .transpose()
    }

    /// Saves the amount spent when a session continues past its override.
    ///
    /// # Errors
    /// Returns an error when SQLite fails.
    pub fn set_continued_past(
        &self,
        session_id: &SessionId,
        spent_micros: u64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE session_budget_overrides SET continued_past_micros = ?2 WHERE session_id = ?1",
            params![session_id.as_str(), budget_count(spent_micros)?],
        )?;
        Ok(())
    }
}

fn budget_count(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::SessionCounterOverflow)
}

fn decode_i64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::SessionCounterOverflow)
}

fn decode_limit(value: Option<i64>) -> Result<Option<u64>, StoreError> {
    value.map(decode_i64).transpose()
}

fn validate_limit(value: Option<u64>) -> Result<(), StoreError> {
    if value.is_some_and(|limit| !(10_000..=10_000_000_000).contains(&limit)) {
        return Err(StoreError::InvalidBudget);
    }
    Ok(())
}
