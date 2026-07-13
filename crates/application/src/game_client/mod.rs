mod actions;
pub(crate) mod lifecycle;
mod processor;
mod runtime;

pub use actions::GameClientActions;
pub use processor::{
    GameClientCacheActions, GameClientLocationSource, GameClientWindowActions,
    NoopGameClientCacheActions, NoopGameClientWindowActions,
};
pub use runtime::{GameClientRuntime, GameClientRuntimeDeps};
