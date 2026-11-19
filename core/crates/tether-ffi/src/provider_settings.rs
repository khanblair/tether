//! Provider key management and connection status for Settings.
//!
//! Keys stay in the secret vault. The database keeps only the vault reference, the saved
//! status, the test time, and the cached model list.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use tether_engine::ProviderCheck;
use tether_secrets::SecretRef;
use tether_store::Store;
use tether_types::{ConnectionStatus, Provider};

use crate::{ConnectionStatusView, ProviderView, SecretVault};

/// The number of providers with independently changing key revisions.
const PROVIDER_COUNT: usize = 5;

/// A provider settings operation failed. Details never contain secret bytes.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ProviderSettingsError {
    /// The vault or database was unavailable.
    #[error("provider settings are unavailable")]
    Unavailable,
    /// The key was empty after trimming spaces and line breaks. Nothing was saved.
    #[error("the key is empty")]
    EmptyKey,
}

/// What Settings shows for one provider.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProviderStatusView {
    /// The saved status. Always `Unset` when the provider has no key.
    pub status: ConnectionStatusView,
    /// How many models the cache holds for this provider.
    pub model_count: u32,
    /// When the last test ran, in Unix milliseconds. `None` when never tested.
    pub tested_at_ms: Option<i64>,
}

/// Runs the provider key check. The app uses the engine's check; tests pass a fake.
trait Checker: Send + Sync {
    /// Checks `key` against the provider's model-list endpoint.
    fn check<'a>(
        &'a self,
        provider: Provider,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = ProviderCheck> + Send + 'a>>;
}

/// Stores the database and the revisions that protect in-flight key tests.
struct State {
    /// The saved provider status and model cache.
    store: Store,
    /// The latest key revision for each provider.
    revisions: [u64; PROVIDER_COUNT],
}

/// The engine's real key check, which sends one authenticated request.
struct EngineChecker;

impl Checker for EngineChecker {
    fn check<'a>(
        &'a self,
        provider: Provider,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = ProviderCheck> + Send + 'a>> {
        Box::pin(tether_engine::check_provider(provider, key))
    }
}

/// Owns the saved connection statuses and uses a vault to read wrapped keys.
#[derive(uniffi::Object)]
pub struct ProviderSettings {
    vault: Arc<SecretVault>,
    state: Mutex<State>,
    checker: Arc<dyn Checker>,
}

#[uniffi::export]
impl ProviderSettings {
    /// Opens the app's private database and uses its existing secret vault.
    ///
    /// # Errors
    /// Returns an error when the database cannot open.
    #[uniffi::constructor]
    pub fn new(
        database_path: &str,
        vault: Arc<SecretVault>,
    ) -> Result<Arc<Self>, ProviderSettingsError> {
        Self::open(database_path, vault, Arc::new(EngineChecker))
    }

    /// Reads what Settings shows for `provider`. A provider with no key reads `Unset`.
    ///
    /// # Errors
    /// Returns an error when the vault or database cannot be read.
    pub fn status(
        &self,
        provider: ProviderView,
    ) -> Result<ProviderStatusView, ProviderSettingsError> {
        self.view(provider.into())
    }

    /// Reports whether a wrapped key exists.
    ///
    /// # Errors
    /// Returns an error when the vault cannot be read.
    pub fn has_key(&self, provider: ProviderView) -> Result<bool, ProviderSettingsError> {
        self.key_exists(provider.into())
    }

    /// Saves a trimmed key wrapped by the platform. Resets the saved status and test time,
    /// and keeps the cached model list.
    ///
    /// # Errors
    /// Returns `EmptyKey` when the key is blank after trimming, and nothing is saved.
    /// Returns `Unavailable` when the vault or database cannot save the change.
    pub fn set_key(&self, provider: ProviderView, key: &str) -> Result<(), ProviderSettingsError> {
        let provider: Provider = provider.into();
        let key = key.trim();
        if key.is_empty() {
            return Err(ProviderSettingsError::EmptyKey);
        }
        let reference = secret_ref(provider)?;
        let mut state = self.lock_state()?;
        let revision = next_revision(&state, provider)?;
        self.vault
            .store
            .put(&reference, &tether_secrets::Secret::from_text(key))
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        state.revisions[provider_index(provider)] = revision;
        state
            .store
            .save_provider_status(
                provider,
                Some(reference.as_str()),
                ConnectionStatus::Unset,
                None,
            )
            .map_err(|_| ProviderSettingsError::Unavailable)
    }

    /// Deletes the key, resets the saved status, and clears the cached models.
    ///
    /// # Errors
    /// Returns an error when the vault or database cannot save the change.
    pub fn remove_key(&self, provider: ProviderView) -> Result<(), ProviderSettingsError> {
        let provider: Provider = provider.into();
        let mut state = self.lock_state()?;
        let revision = next_revision(&state, provider)?;
        self.vault
            .store
            .delete(&secret_ref(provider)?)
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        state.revisions[provider_index(provider)] = revision;
        state
            .store
            .save_provider_status(provider, None, ConnectionStatus::Unset, None)
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        state
            .store
            .clear_models(provider)
            .map_err(|_| ProviderSettingsError::Unavailable)
    }

    /// Returns the saved key text, or `None` when there is no key. Kotlin must check
    /// biometrics before it calls this.
    ///
    /// # Errors
    /// Returns an error when the vault cannot read or unwrap the key.
    pub fn reveal_key(
        &self,
        provider: ProviderView,
    ) -> Result<Option<String>, ProviderSettingsError> {
        let reference = secret_ref(provider.into())?;
        self.vault
            .reveal(reference.as_str())
            .map_err(|_| ProviderSettingsError::Unavailable)
    }

    /// The URL that a key test sends its first request to. It never holds a key.
    #[must_use]
    pub fn test_target(&self, provider: ProviderView) -> String {
        tether_engine::provider_test_target(provider.into())
    }

    /// Checks the saved key with the provider and saves the result. A key that is not
    /// `Ok` leaves the cached models alone. Blocks until the check ends, which can take up
    /// to 20 seconds. Kotlin must call this off the main thread.
    ///
    /// # Errors
    /// Returns an error when the vault, runtime, or database is unavailable.
    pub fn test_connection(
        &self,
        provider: ProviderView,
    ) -> Result<ProviderStatusView, ProviderSettingsError> {
        let provider: Provider = provider.into();
        let reference = secret_ref(provider)?;
        let revision = self.provider_revision(provider)?;
        let Some(secret) = self
            .vault
            .store
            .get(&reference)
            .map_err(|_| ProviderSettingsError::Unavailable)?
        else {
            return self.view(provider);
        };
        let key = secret
            .expose_text()
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        let check = self.run_check(provider, key)?;
        if !self.save_check(provider, revision, &reference, &check)? {
            return self.view(provider);
        }
        self.view(provider)
    }
}

impl ProviderSettings {
    /// Opens the database and keeps `checker` for later key tests.
    fn open(
        database_path: &str,
        vault: Arc<SecretVault>,
        checker: Arc<dyn Checker>,
    ) -> Result<Arc<Self>, ProviderSettingsError> {
        let store = Store::open(Path::new(database_path))
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        Ok(Arc::new(Self {
            vault,
            state: Mutex::new(State {
                store,
                revisions: [0; PROVIDER_COUNT],
            }),
            checker,
        }))
    }

    /// Builds the status view. The saved status is read only when a key exists.
    fn view(&self, provider: Provider) -> Result<ProviderStatusView, ProviderSettingsError> {
        let status = if self.key_exists(provider)? {
            self.lock_state()?
                .store
                .provider_status(provider)
                .map_err(|_| ProviderSettingsError::Unavailable)?
        } else {
            ConnectionStatus::Unset
        };
        let state = self.lock_state()?;
        Ok(ProviderStatusView {
            status: status.into(),
            model_count: state
                .store
                .models_count(provider)
                .map_err(|_| ProviderSettingsError::Unavailable)?,
            tested_at_ms: state
                .store
                .provider_tested_at(provider)
                .map_err(|_| ProviderSettingsError::Unavailable)?,
        })
    }

    fn key_exists(&self, provider: Provider) -> Result<bool, ProviderSettingsError> {
        self.vault
            .store
            .exists(&secret_ref(provider)?)
            .map_err(|_| ProviderSettingsError::Unavailable)
    }

    /// Runs one key check on a new current-thread runtime and waits for it.
    fn run_check(
        &self,
        provider: Provider,
        key: &str,
    ) -> Result<ProviderCheck, ProviderSettingsError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        Ok(runtime.block_on(self.checker.check(provider, key)))
    }

    /// Locks the store and the provider key revisions together.
    fn lock_state(&self) -> Result<MutexGuard<'_, State>, ProviderSettingsError> {
        self.state
            .lock()
            .map_err(|_| ProviderSettingsError::Unavailable)
    }

    /// Reads the key revision before an in-flight key check reads the vault.
    fn provider_revision(&self, provider: Provider) -> Result<u64, ProviderSettingsError> {
        Ok(self.lock_state()?.revisions[provider_index(provider)])
    }

    /// Saves a check only when its key revision is still current.
    fn save_check(
        &self,
        provider: Provider,
        revision: u64,
        reference: &SecretRef,
        check: &ProviderCheck,
    ) -> Result<bool, ProviderSettingsError> {
        let mut state = self.lock_state()?;
        if state.revisions[provider_index(provider)] != revision {
            return Ok(false);
        }
        let now = now_ms()?;
        state
            .store
            .save_provider_status(provider, Some(reference.as_str()), check.status, Some(now))
            .map_err(|_| ProviderSettingsError::Unavailable)?;
        if matches!(check.status, ConnectionStatus::Ok) {
            state
                .store
                .replace_models(provider, &check.models, now)
                .map_err(|_| ProviderSettingsError::Unavailable)?;
        }
        Ok(true)
    }
}

/// Returns the array index assigned to a provider.
fn provider_index(provider: Provider) -> usize {
    match provider {
        Provider::Anthropic => 0,
        Provider::OpenAi => 1,
        Provider::Gemini => 2,
        Provider::DeepSeek => 3,
        Provider::OpenRouter => 4,
    }
}

/// Calculates the next revision without changing state.
fn next_revision(state: &State, provider: Provider) -> Result<u64, ProviderSettingsError> {
    state.revisions[provider_index(provider)]
        .checked_add(1)
        .ok_or(ProviderSettingsError::Unavailable)
}

fn secret_ref(provider: Provider) -> Result<SecretRef, ProviderSettingsError> {
    SecretRef::new(&format!("provider-{}", provider.as_str()))
        .map_err(|_| ProviderSettingsError::Unavailable)
}

/// The current time in Unix milliseconds.
fn now_ms() -> Result<i64, ProviderSettingsError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProviderSettingsError::Unavailable)?;
    i64::try_from(elapsed.as_millis()).map_err(|_| ProviderSettingsError::Unavailable)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use tether_types::{ModelCaps, ModelInfo};

    use super::*;
    use crate::{SecretsPlatform, SecretsPlatformFailure};

    const KEY: &str = "sk-gemini-test-key-7781";

    struct MemoryPlatform;

    impl SecretsPlatform for MemoryPlatform {
        fn wrap(
            &self,
            plaintext: Vec<u8>,
            _context: Vec<u8>,
        ) -> Result<Vec<u8>, SecretsPlatformFailure> {
            Ok(plaintext)
        }

        fn unwrap(
            &self,
            wrapped: Vec<u8>,
            _context: Vec<u8>,
        ) -> Result<Vec<u8>, SecretsPlatformFailure> {
            Ok(wrapped)
        }
    }

    /// Returns the answer it was last given and counts every check.
    struct FakeChecker {
        answer: Mutex<ProviderCheck>,
        calls: AtomicUsize,
        in_flight: Mutex<Box<dyn Fn() + Send + Sync>>,
    }

    /// Does nothing when no in-flight hook is installed.
    fn no_op_hook() {}

    impl FakeChecker {
        /// Replaces the answer returned by the next check.
        fn answer(&self, check: ProviderCheck) {
            *self.answer.lock().unwrap() = check;
        }

        /// Runs `hook` once during the next check.
        fn set_in_flight_hook(&self, hook: impl Fn() + Send + Sync + 'static) {
            *self.in_flight.lock().unwrap() = Box::new(hook);
        }

        /// Takes and runs the hook installed for the next check, then restores a no-op.
        fn run_in_flight_hook(&self) {
            let hook =
                std::mem::replace(&mut *self.in_flight.lock().unwrap(), Box::new(no_op_hook));
            hook();
        }
    }

    impl Checker for FakeChecker {
        fn check<'a>(
            &'a self,
            _provider: Provider,
            _key: &'a str,
        ) -> Pin<Box<dyn Future<Output = ProviderCheck> + Send + 'a>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.run_in_flight_hook();
            let answer = self.answer.lock().unwrap().clone();
            Box::pin(std::future::ready(answer))
        }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        db: PathBuf,
        settings: Arc<ProviderSettings>,
        checker: Arc<FakeChecker>,
    }

    fn fixture(answer: ProviderCheck) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let vault = SecretVault::new(
            dir.path().join("secrets").to_string_lossy().into_owned(),
            Arc::new(MemoryPlatform),
        );
        let db = dir.path().join("tether.db");
        let checker = Arc::new(FakeChecker {
            answer: Mutex::new(answer),
            calls: AtomicUsize::new(0),
            in_flight: Mutex::new(Box::new(no_op_hook)),
        });
        let shared: Arc<dyn Checker> = checker.clone();
        let settings = ProviderSettings::open(&db.to_string_lossy(), vault, shared).unwrap();
        Fixture {
            _dir: dir,
            db,
            settings,
            checker,
        }
    }

    fn model(id: &str) -> ModelInfo {
        ModelInfo {
            provider: Provider::Gemini,
            id: id.to_owned(),
            display_name: id.to_owned(),
            context_window: None,
            max_output_tokens: None,
            price: None,
            caps: ModelCaps::UNKNOWN,
        }
    }

    fn outcome(status: ConnectionStatus, ids: &[&str]) -> ProviderCheck {
        ProviderCheck {
            status,
            models: ids.iter().map(|id| model(id)).collect(),
        }
    }

    fn cached_ids(settings: &ProviderSettings) -> Vec<String> {
        settings
            .lock_state()
            .unwrap()
            .store
            .models(Provider::Gemini)
            .unwrap()
            .into_iter()
            .map(|model| model.id)
            .collect()
    }

    fn status_view(
        status: ConnectionStatusView,
        model_count: u32,
        tested_at_ms: Option<i64>,
    ) -> ProviderStatusView {
        ProviderStatusView {
            status,
            model_count,
            tested_at_ms,
        }
    }

    fn now() -> i64 {
        i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    }

    const THREE: [&str; 3] = ["gemini-a", "gemini-b", "gemini-c"];

    #[test]
    fn a_key_round_trips_through_set_has_and_reveal() {
        let fx = fixture(outcome(ConnectionStatus::Unset, &[]));
        let settings = &fx.settings;
        assert!(!settings.has_key(ProviderView::Gemini).unwrap());
        assert_eq!(settings.reveal_key(ProviderView::Gemini).unwrap(), None);
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        assert!(settings.has_key(ProviderView::Gemini).unwrap());
        assert_eq!(
            settings
                .reveal_key(ProviderView::Gemini)
                .unwrap()
                .as_deref(),
            Some(KEY)
        );
    }

    #[test]
    fn a_pasted_key_is_saved_trimmed() {
        let fx = fixture(outcome(ConnectionStatus::Unset, &[]));
        fx.settings
            .set_key(ProviderView::Gemini, "  \n sk-padded \r\n ")
            .unwrap();
        assert_eq!(
            fx.settings
                .reveal_key(ProviderView::Gemini)
                .unwrap()
                .as_deref(),
            Some("sk-padded")
        );
    }

    #[test]
    fn a_blank_key_is_refused_and_the_old_key_stays() {
        let fx = fixture(outcome(ConnectionStatus::Unset, &[]));
        let settings = &fx.settings;
        assert!(matches!(
            settings.set_key(ProviderView::Gemini, "").unwrap_err(),
            ProviderSettingsError::EmptyKey
        ));
        assert!(!settings.has_key(ProviderView::Gemini).unwrap());

        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        assert!(matches!(
            settings.set_key(ProviderView::Gemini, "  \n").unwrap_err(),
            ProviderSettingsError::EmptyKey
        ));
        assert_eq!(
            settings
                .reveal_key(ProviderView::Gemini)
                .unwrap()
                .as_deref(),
            Some(KEY)
        );
    }

    #[test]
    fn a_new_key_resets_a_tested_status_and_keeps_the_models() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        let tested = settings.test_connection(ProviderView::Gemini).unwrap();
        assert_eq!(tested.status, ConnectionStatusView::Ok);
        assert_eq!(tested.model_count, 3);
        assert!(tested.tested_at_ms.is_some());

        settings.set_key(ProviderView::Gemini, "sk-second").unwrap();
        assert_eq!(
            settings.status(ProviderView::Gemini).unwrap(),
            status_view(ConnectionStatusView::Unset, 3, None)
        );
        assert_eq!(cached_ids(settings), THREE);
    }

    #[test]
    fn remove_clears_the_key_the_status_and_the_models() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();

        settings.remove_key(ProviderView::Gemini).unwrap();
        assert!(!settings.has_key(ProviderView::Gemini).unwrap());
        assert_eq!(settings.reveal_key(ProviderView::Gemini).unwrap(), None);
        assert_eq!(
            settings.status(ProviderView::Gemini).unwrap(),
            status_view(ConnectionStatusView::Unset, 0, None)
        );
        assert_eq!(cached_ids(settings), Vec::<String>::new());
    }

    #[test]
    fn remove_key_saves_unset_and_clears_the_test_time_in_the_store() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();

        settings.remove_key(ProviderView::Gemini).unwrap();

        let state = settings.state.lock().unwrap();
        assert_eq!(
            state.store.provider_status(Provider::Gemini).unwrap(),
            ConnectionStatus::Unset
        );
        assert_eq!(
            state.store.provider_tested_at(Provider::Gemini).unwrap(),
            None
        );
    }

    #[test]
    fn status_is_unset_without_a_key_even_when_the_row_says_ok() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();
        settings
            .vault
            .store
            .delete(&secret_ref(Provider::Gemini).unwrap())
            .unwrap();

        assert_eq!(
            settings
                .lock_state()
                .unwrap()
                .store
                .provider_status(Provider::Gemini)
                .unwrap(),
            ConnectionStatus::Ok
        );
        assert_eq!(
            settings.status(ProviderView::Gemini).unwrap().status,
            ConnectionStatusView::Unset
        );
    }

    #[test]
    fn a_test_without_a_key_makes_no_call() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let result = fx.settings.test_connection(ProviderView::Gemini).unwrap();
        assert_eq!(result, status_view(ConnectionStatusView::Unset, 0, None));
        assert_eq!(fx.checker.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_passing_test_saves_the_time_and_the_three_models() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();

        let before = now();
        let result = settings.test_connection(ProviderView::Gemini).unwrap();
        let after = now();

        assert_eq!(result.status, ConnectionStatusView::Ok);
        assert_eq!(result.model_count, 3);
        let tested_at = result.tested_at_ms.unwrap();
        assert!(before <= tested_at && tested_at <= after, "{tested_at}");
        assert_eq!(cached_ids(settings), THREE);
        assert_eq!(fx.checker.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn replacing_a_key_during_a_test_discards_the_old_result() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &["old-model"]));
        let settings = Arc::clone(&fx.settings);
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();
        fx.checker.answer(outcome(ConnectionStatus::Ok, &THREE));

        let changed_settings = Arc::clone(&settings);
        fx.checker.set_in_flight_hook(move || {
            changed_settings
                .set_key(ProviderView::Gemini, "sk-replacement")
                .unwrap();
        });
        let result = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(result, status_view(ConnectionStatusView::Unset, 1, None));
        assert_eq!(
            settings
                .reveal_key(ProviderView::Gemini)
                .unwrap()
                .as_deref(),
            Some("sk-replacement")
        );
        assert_eq!(cached_ids(&settings), vec!["old-model"]);
    }

    #[test]
    fn removing_a_key_during_a_test_discards_the_old_result_and_cache() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = Arc::clone(&fx.settings);
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();

        let changed_settings = Arc::clone(&settings);
        fx.checker.set_in_flight_hook(move || {
            changed_settings.remove_key(ProviderView::Gemini).unwrap();
        });
        let result = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(result, status_view(ConnectionStatusView::Unset, 0, None));
        assert_eq!(settings.reveal_key(ProviderView::Gemini).unwrap(), None);
        assert_eq!(cached_ids(&settings), Vec::<String>::new());
    }

    #[test]
    fn an_unchanged_key_saves_the_test_result() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();

        let result = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(result.status, ConnectionStatusView::Ok);
        assert_eq!(result.model_count, 3);
        assert!(result.tested_at_ms.is_some());
        assert_eq!(cached_ids(settings), THREE);
    }

    #[test]
    fn changing_another_provider_key_does_not_discard_the_test_result() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = Arc::clone(&fx.settings);
        settings.set_key(ProviderView::Gemini, KEY).unwrap();

        let changed_settings = Arc::clone(&settings);
        fx.checker.set_in_flight_hook(move || {
            changed_settings
                .set_key(ProviderView::OpenAi, "sk-openai")
                .unwrap();
        });
        let result = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(result.status, ConnectionStatusView::Ok);
        assert_eq!(result.model_count, 3);
        assert_eq!(cached_ids(&settings), THREE);
        assert_eq!(
            settings
                .reveal_key(ProviderView::OpenAi)
                .unwrap()
                .as_deref(),
            Some("sk-openai")
        );
    }

    #[test]
    fn sequential_tests_of_one_key_keep_the_same_revision() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        let revision = settings.provider_revision(Provider::Gemini).unwrap();

        let first = settings.test_connection(ProviderView::Gemini).unwrap();
        let second = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(first.status, ConnectionStatusView::Ok);
        assert_eq!(second.status, ConnectionStatusView::Ok);
        assert_eq!(second.model_count, 3);
        assert_eq!(
            settings.provider_revision(Provider::Gemini).unwrap(),
            revision
        );
        assert_eq!(fx.checker.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_failed_test_saves_the_status_and_keeps_the_old_models() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        let settings = &fx.settings;
        settings.set_key(ProviderView::Gemini, KEY).unwrap();
        settings.test_connection(ProviderView::Gemini).unwrap();

        fx.checker.answer(outcome(ConnectionStatus::BadKey, &[]));
        let result = settings.test_connection(ProviderView::Gemini).unwrap();

        assert_eq!(result.status, ConnectionStatusView::BadKey);
        assert_eq!(result.model_count, 3);
        assert_eq!(
            settings.status(ProviderView::Gemini).unwrap().status,
            ConnectionStatusView::BadKey
        );
        assert_eq!(cached_ids(settings), THREE);
    }

    #[test]
    fn the_database_never_holds_the_key_text() {
        let fx = fixture(outcome(ConnectionStatus::Ok, &THREE));
        fx.settings.set_key(ProviderView::Gemini, KEY).unwrap();
        fx.settings.test_connection(ProviderView::Gemini).unwrap();

        let mut bytes = fs::read(&fx.db).unwrap();
        let wal = PathBuf::from(format!("{}-wal", fx.db.display()));
        if wal.exists() {
            bytes.extend(fs::read(&wal).unwrap());
        }
        let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
        assert!(contains(b"provider-gemini"), "the saved row was not found");
        assert!(!contains(KEY.as_bytes()));
    }

    #[test]
    fn test_target_matches_the_engine_for_every_provider() {
        let fx = fixture(outcome(ConnectionStatus::Unset, &[]));
        let cases = [
            (ProviderView::Anthropic, Provider::Anthropic),
            (ProviderView::OpenAi, Provider::OpenAi),
            (ProviderView::Gemini, Provider::Gemini),
            (ProviderView::DeepSeek, Provider::DeepSeek),
            (ProviderView::OpenRouter, Provider::OpenRouter),
        ];
        for (view, provider) in cases {
            assert_eq!(
                fx.settings.test_target(view),
                tether_engine::provider_test_target(provider),
                "{provider:?}"
            );
        }
    }

    #[test]
    fn every_provider_view_maps_to_its_provider_and_back() {
        let cases = [
            (ProviderView::Anthropic, Provider::Anthropic),
            (ProviderView::OpenAi, Provider::OpenAi),
            (ProviderView::Gemini, Provider::Gemini),
            (ProviderView::DeepSeek, Provider::DeepSeek),
            (ProviderView::OpenRouter, Provider::OpenRouter),
        ];
        for (view, provider) in cases {
            assert_eq!(Provider::from(view), provider);
            assert_eq!(ProviderView::from(provider), view);
        }
    }
}
