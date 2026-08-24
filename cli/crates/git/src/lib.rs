mod cache;
mod model;
mod repository;
mod traverse;

pub use cache::{CacheKey, load_cached, store_cached};
pub use model::{
    CoChangeFact, CommitFact, Completeness, ContributorIdentity, GitHistoryError,
    GitHistorySnapshot, HistoryMetric, KnowledgeSignal, PathChange, RepositoryIdentity,
    SelectedRef,
};
pub use repository::{RefScope, open_repository, resolve_refs};
pub use traverse::{HistoryOptions, MailmapMode, collect_history, collect_history_cached};
