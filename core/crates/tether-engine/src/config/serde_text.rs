//! Serde helpers that store config enums as their text form.

pub(super) mod executor_text {
    use serde::{Deserialize, Deserializer, Serializer};
    use tether_types::Executor;

    // Serde's `with` hook passes a reference, even for small `Copy` types.
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's with hook passes &Executor"
    )]
    pub(in crate::config) fn serialize<S>(
        value: &Executor,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value.as_str())
    }

    pub(in crate::config) fn deserialize<'de, D>(deserializer: D) -> Result<Executor, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

pub(super) mod provider_option_text {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use tether_types::Provider;

    // Serde's `with` hook passes &Option<T> for the field type.
    #[expect(
        clippy::ref_option,
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's with hook passes &Option<Provider>"
    )]
    pub(in crate::config) fn serialize<S>(
        value: &Option<Provider>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.map(Provider::as_str).serialize(serializer)
    }

    pub(in crate::config) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<Provider>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|value| value.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}
