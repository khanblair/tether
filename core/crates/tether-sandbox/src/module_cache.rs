//! One process engine and an in-memory compiled module cache.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};
use wasmtime::{Config, Engine, Module, OptLevel};

static ENGINE: OnceLock<Result<Arc<Engine>, String>> = OnceLock::new();

/// Engine or compilation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    /// The process engine could not start.
    Engine(String),
    /// The module bytes could not compile.
    Module(String),
    /// The module cache lock was poisoned.
    CacheUnavailable,
}

/// Compiled modules keyed by a SHA-256 hash of their exact WASM bytes.
#[derive(Debug)]
pub struct ModuleCache {
    engine: Arc<Engine>,
    modules: Mutex<HashMap<[u8; 32], Arc<Module>>>,
}

impl ModuleCache {
    /// Reuses the single process engine and starts an empty cache.
    ///
    /// # Errors
    /// Returns a Wasmtime engine configuration failure.
    pub fn new() -> Result<Self, SandboxError> {
        Ok(Self {
            engine: process_engine()?,
            modules: Mutex::new(HashMap::new()),
        })
    }

    /// Returns an already compiled module or compiles and saves it.
    ///
    /// # Errors
    /// Returns an invalid WASM module or cache lock failure.
    pub fn load(&self, bytes: &[u8]) -> Result<Arc<Module>, SandboxError> {
        let key: [u8; 32] = Sha256::digest(bytes).into();
        if let Some(module) = self
            .modules
            .lock()
            .map_err(|_| SandboxError::CacheUnavailable)?
            .get(&key)
            .cloned()
        {
            return Ok(module);
        }
        let compiled = Arc::new(
            Module::new(&self.engine, bytes)
                .map_err(|error| SandboxError::Module(error.to_string()))?,
        );
        let mut modules = self
            .modules
            .lock()
            .map_err(|_| SandboxError::CacheUnavailable)?;
        Ok(Arc::clone(modules.entry(key).or_insert(compiled)))
    }

    /// The number of compiled modules currently held.
    ///
    /// # Errors
    /// Returns a cache lock failure.
    pub fn len(&self) -> Result<usize, SandboxError> {
        Ok(self
            .modules
            .lock()
            .map_err(|_| SandboxError::CacheUnavailable)?
            .len())
    }

    /// Whether no modules are cached.
    ///
    /// # Errors
    /// Returns a cache lock failure.
    pub fn is_empty(&self) -> Result<bool, SandboxError> {
        Ok(self.len()? == 0)
    }
}

fn process_engine() -> Result<Arc<Engine>, SandboxError> {
    ENGINE
        .get_or_init(|| {
            let mut config = Config::new();
            config.wasm_multi_memory(false);
            config.epoch_interruption(true);
            config.cranelift_opt_level(OptLevel::SpeedAndSize);
            Engine::new(&config)
                .map(Arc::new)
                .map_err(|error| error.to_string())
        })
        .clone()
        .map_err(SandboxError::Engine)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_WASM: &[u8] = b"\0asm\x01\0\0\0";

    #[test]
    fn one_engine_and_one_compilation_per_cache_key() {
        let first = ModuleCache::new().unwrap();
        let second = ModuleCache::new().unwrap();
        assert!(Arc::ptr_eq(&first.engine, &second.engine));
        let original = first.load(EMPTY_WASM).unwrap();
        let again = first.load(EMPTY_WASM).unwrap();
        assert!(Arc::ptr_eq(&original, &again));
        assert_eq!(first.len().unwrap(), 1);
        assert!(second.is_empty().unwrap());
    }

    #[test]
    fn invalid_module_does_not_enter_cache() {
        let cache = ModuleCache::new().unwrap();
        assert!(matches!(cache.load(b"bad"), Err(SandboxError::Module(_))));
        assert!(cache.is_empty().unwrap());
    }
}
