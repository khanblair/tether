//! Saved provider connection state. Secret bytes remain in the platform vault.

use rusqlite::{OptionalExtension, params};
use tether_types::{ConnectionStatus, Provider};

use crate::{Store, StoreError};

impl Store {
    /// Reads a provider's last connection status. A stored `testing` reads as `Unset`,
    /// because a test that died mid-call leaves no result behind.
    ///
    /// # Errors
    /// Returns a database error or an invalid saved status.
    pub fn provider_status(&self, provider: Provider) -> Result<ConnectionStatus, StoreError> {
        let status: Option<String> = self
            .conn
            .query_row(
                "SELECT status FROM provider_configs WHERE provider = ?1",
                [provider.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        match status.as_deref() {
            None => Ok(ConnectionStatus::Unset),
            Some(text) if text == ConnectionStatus::Testing.as_str() => Ok(ConnectionStatus::Unset),
            Some(text) => text.parse().map_err(|_| StoreError::InvalidModelEnum {
                field: "provider status",
                value: text.to_owned(),
            }),
        }
    }

    /// Reads when a provider was last tested, in Unix milliseconds. `None` when it was
    /// never tested or has no row.
    ///
    /// # Errors
    /// Returns a database error.
    pub fn provider_tested_at(&self, provider: Provider) -> Result<Option<i64>, StoreError> {
        let tested_at: Option<Option<i64>> = self
            .conn
            .query_row(
                "SELECT tested_at FROM provider_configs WHERE provider = ?1",
                [provider.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        Ok(tested_at.flatten())
    }

    /// Saves a provider's key reference and status. The key itself never enters SQLite.
    /// Each save replaces the reference, status, and test time together.
    ///
    /// # Errors
    /// Returns `StoreError::TransientStatus` for `Testing`, which is screen state and is
    /// never stored. Returns a database error otherwise.
    pub fn save_provider_status(
        &self,
        provider: Provider,
        secret_ref: Option<&str>,
        status: ConnectionStatus,
        tested_at: Option<i64>,
    ) -> Result<(), StoreError> {
        if matches!(status, ConnectionStatus::Testing) {
            return Err(StoreError::TransientStatus {
                status: status.as_str(),
            });
        }
        self.conn.execute(
            "INSERT INTO provider_configs (provider, secret_ref, status, tested_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(provider) DO UPDATE SET secret_ref = excluded.secret_ref, status = excluded.status, tested_at = excluded.tested_at",
            params![provider.as_str(), secret_ref, status.as_str(), tested_at],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_reference(store: &Store, provider: &str) -> Option<String> {
        store
            .conn
            .query_row(
                "SELECT secret_ref FROM provider_configs WHERE provider = ?1",
                [provider],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn statuses_round_trip_without_secret_bytes() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(
            store.provider_status(Provider::Gemini).unwrap(),
            ConnectionStatus::Unset
        );
        let storable = ConnectionStatus::ALL
            .iter()
            .filter(|status| !matches!(status, ConnectionStatus::Testing));
        for &status in storable {
            store
                .save_provider_status(Provider::Gemini, Some("provider-gemini"), status, Some(42))
                .unwrap();
            assert_eq!(store.provider_status(Provider::Gemini).unwrap(), status);
        }
        assert_eq!(
            stored_reference(&store, "gemini").as_deref(),
            Some("provider-gemini")
        );
    }

    #[test]
    fn saving_testing_is_refused_and_leaves_the_row_alone() {
        let store = Store::open_in_memory().unwrap();
        store
            .save_provider_status(
                Provider::Gemini,
                Some("ref-a"),
                ConnectionStatus::Ok,
                Some(5),
            )
            .unwrap();

        let error = store
            .save_provider_status(
                Provider::Gemini,
                Some("ref-b"),
                ConnectionStatus::Testing,
                Some(9),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            StoreError::TransientStatus { status: "testing" }
        ));
        assert_eq!(
            store.provider_status(Provider::Gemini).unwrap(),
            ConnectionStatus::Ok
        );
        assert_eq!(store.provider_tested_at(Provider::Gemini).unwrap(), Some(5));
        assert_eq!(stored_reference(&store, "gemini").as_deref(), Some("ref-a"));
    }

    #[test]
    fn a_stored_testing_value_reads_as_unset() {
        let store = Store::open_in_memory().unwrap();
        store
            .conn
            .execute(
                "INSERT INTO provider_configs (provider, status) VALUES ('gemini', 'testing')",
                [],
            )
            .unwrap();

        assert_eq!(
            store.provider_status(Provider::Gemini).unwrap(),
            ConnectionStatus::Unset
        );
    }

    #[test]
    fn tested_at_is_none_without_a_row_or_a_test() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.provider_tested_at(Provider::OpenAi).unwrap(), None);

        store
            .save_provider_status(Provider::OpenAi, None, ConnectionStatus::Unset, None)
            .unwrap();
        assert_eq!(store.provider_tested_at(Provider::OpenAi).unwrap(), None);
    }

    #[test]
    fn tested_at_returns_the_saved_milliseconds() {
        let store = Store::open_in_memory().unwrap();
        store
            .save_provider_status(
                Provider::OpenAi,
                Some("provider-openai"),
                ConnectionStatus::Ok,
                Some(1_700_000_000_123),
            )
            .unwrap();

        assert_eq!(
            store.provider_tested_at(Provider::OpenAi).unwrap(),
            Some(1_700_000_000_123)
        );
        assert_eq!(store.provider_tested_at(Provider::Gemini).unwrap(), None);
    }

    #[test]
    fn a_second_save_replaces_reference_status_and_test_time() {
        let store = Store::open_in_memory().unwrap();
        store
            .save_provider_status(
                Provider::Anthropic,
                Some("provider-anthropic-1"),
                ConnectionStatus::Ok,
                Some(1),
            )
            .unwrap();

        store
            .save_provider_status(
                Provider::Anthropic,
                Some("provider-anthropic-2"),
                ConnectionStatus::BadKey,
                Some(2),
            )
            .unwrap();

        assert_eq!(
            store.provider_status(Provider::Anthropic).unwrap(),
            ConnectionStatus::BadKey
        );
        assert_eq!(
            store.provider_tested_at(Provider::Anthropic).unwrap(),
            Some(2)
        );
        assert_eq!(
            stored_reference(&store, "anthropic").as_deref(),
            Some("provider-anthropic-2")
        );
    }

    #[test]
    fn saving_no_reference_clears_it_and_the_test_time() {
        let store = Store::open_in_memory().unwrap();
        store
            .save_provider_status(
                Provider::DeepSeek,
                Some("provider-deepseek"),
                ConnectionStatus::Ok,
                Some(7),
            )
            .unwrap();

        store
            .save_provider_status(Provider::DeepSeek, None, ConnectionStatus::Unset, None)
            .unwrap();

        assert_eq!(stored_reference(&store, "deepseek"), None);
        assert_eq!(store.provider_tested_at(Provider::DeepSeek).unwrap(), None);
        assert_eq!(
            store.provider_status(Provider::DeepSeek).unwrap(),
            ConnectionStatus::Unset
        );
    }
}
