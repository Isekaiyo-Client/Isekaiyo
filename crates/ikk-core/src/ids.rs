//! Typed identifiers. Newtypes prevent ID mix-ups (instance vs version vs …)
//! and keep random strings from becoming the universal data model.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_newtype {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_newtype!(
    /// Unique, generated identifier of a Minecraft instance.
    InstanceId
);
id_newtype!(
    /// A Minecraft version identifier as served by Mojang's manifest.
    MinecraftVersionId
);
id_newtype!(
    /// Unique identifier of an installed Isekaiyo plugin.
    PluginId
);
id_newtype!(
    /// Identifier of a first-party client module, e.g. `ikk.keystrokes`.
    ModuleId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_ids_are_opaque_and_distinct_types() {
        let a = InstanceId::new("inst_abc123");
        let v = MinecraftVersionId::new("26.2");
        assert_eq!(a.as_str(), "inst_abc123");
        assert_eq!(v.to_string(), "26.2");
        // Compile-time guarantee: different newtypes never unify.
        fn takes_instance(_: &InstanceId) {}
        takes_instance(&a);
    }
}
