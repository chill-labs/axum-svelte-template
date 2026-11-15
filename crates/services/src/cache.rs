// =============================================================================
// IN-MEMORY MOKA CACHE SERVICE
// =============================================================================
//
// Provides bounded, TTL/TTI-managed in-memory Moka caches for authentication
// sessions, user profiles, workspace lists, and workspace Todo lists.
//
// ## Key Concepts
//
// - **CachedList<V>**: Weighted Moka cache keyed by `i32` (`user_id` or
//   `workspace_id`) holding `Vec<V>` with `get_or_load`, `insert`, and `invalidate`.
// - **Session & User Lookup Caches**: Fast O(1) lookup for hot requests while
//   honoring explicit invalidation on profile updates, role edits, and logout.
//
// ## Architecture Notes
//
// Stored inside `AppState` and injected into every domain service and the auth
// middleware. All caches use Moka's async concurrent implementation.

// SECTION: IMPORTS

// `moka` - High-performance concurrent async in-memory cache with LRU/LFU eviction.
// - `Cache` - Thread-safe async cache handle supporting `get`, `insert`, `invalidate`
use moka::future::Cache;

// `std` - Standard library future trait and duration helpers.
// - `Future` - Async loader trait bound for `CachedList::get_or_load`
// - `Duration` - Configures TTL and TTI eviction intervals
use std::future::Future;
use std::time::Duration;

// `shared` - Shared configuration and DTO types cached across services.
// - `CacheConfig` - Runtime TTL and capacity limits from `APP__CACHE__*`
// - `AppError` - Standardized error type propagated from cache loaders
// - `TodoResponse` - Cached per-workspace Todo DTO
// - `WorkspaceResponse` - Cached per-user Workspace DTO
use shared::config::settings::CacheConfig;
use shared::domain::error::AppError;
use shared::domain::todo::TodoResponse;
use shared::domain::workspace::WorkspaceResponse;

// `crate::entities` - SeaORM entity row types cached for fast auth hydration.
// - `users` - Database `users::Model` cached by `user_id`
use crate::entities::users;

// SECTION: WEIGHTED CACHED LIST

/// Bounded, weighted Moka list cache keyed by `i32` (`user_id` or `workspace_id`).
#[derive(Clone)]
pub struct CachedList<V: Clone + Send + Sync + 'static> {
    /// Underlying weighted async Moka cache.
    cache: Cache<i32, Vec<V>>,
}

impl<V: Clone + Send + Sync + 'static> std::fmt::Debug for CachedList<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedList")
            .field("entry_count", &self.cache.entry_count())
            .finish()
    }
}

impl<V: Clone + Send + Sync + 'static> CachedList<V> {
    /// Constructs a weighted `CachedList<V>` bounded by total byte weight and idle time.
    ///
    /// # Arguments
    ///
    /// * `max_weight` - Total weighted budget across all cached keys
    /// * `weight_per_item` - Estimated weight per `V` element in the vector
    /// * `tti_secs` - Time-to-idle expiration in seconds
    pub fn new(max_weight: u64, weight_per_item: u32, tti_secs: u64) -> Self {
        let safe_budget = max_weight.max(1024);
        let cache = Cache::builder()
            .max_capacity(safe_budget)
            .weigher(move |_key: &i32, items: &Vec<V>| -> u32 {
                let len = items.len() as u64;
                let weight = len
                    .saturating_mul(u64::from(weight_per_item))
                    .saturating_add(64);
                weight.min(u64::from(u32::MAX)) as u32
            })
            .time_to_idle(Duration::from_secs(tti_secs.max(1)))
            .build();

        Self { cache }
    }

    /// Returns cached items for `key` or executes `loader` on cache miss and stores the result.
    ///
    /// # Arguments
    ///
    /// * `key` - Entity scope key (`user_id` or `workspace_id`)
    /// * `loader` - Async database loader invoked when `key` is absent
    pub async fn get_or_load<F, Fut>(&self, key: i32, loader: F) -> Result<Vec<V>, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<V>, AppError>>,
    {
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let loaded = loader().await?;
        self.cache.insert(key, loaded.clone()).await;
        Ok(loaded)
    }

    /// Writes or replaces the cached list for `key`.
    pub async fn insert(&self, key: i32, items: Vec<V>) {
        self.cache.insert(key, items).await;
    }

    /// Evicts cached items for `key` so the next read reloads from PostgreSQL.
    pub async fn invalidate(&self, key: i32) {
        self.cache.invalidate(&key).await;
    }

    /// Evicts all entries stored in this list cache.
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }
}

// SECTION: CACHE SERVICE CONTAINER

/// Centralized Moka cache registry shared via `AppState`.
#[derive(Clone, Debug)]
pub struct CacheService {
    /// Maps SHA-256 hashed session tokens (`sessions.id`) to owning `user_id`.
    pub sessions: Cache<String, i32>,
    /// Maps `user_id` to hydrated `users::Model` rows.
    pub users: Cache<i32, users::Model>,
    /// Cached workspace summaries keyed by `user_id`.
    pub user_workspaces: CachedList<WorkspaceResponse>,
    /// Cached Todo lists keyed by `workspace_id`.
    pub workspace_todos: CachedList<TodoResponse>,
}

impl CacheService {
    /// Initializes all application caches according to `CacheConfig`.
    ///
    /// # Arguments
    ///
    /// * `config` - Cache TTL and entry capacity configuration
    pub fn new(config: &CacheConfig) -> Self {
        let cap = config.max_capacity.max(100);
        let session_ttl = Duration::from_secs(config.session_ttl_secs.max(1));
        let user_ttl = Duration::from_secs(config.user_ttl_secs.max(1));

        let sessions = Cache::builder()
            .max_capacity(cap)
            .time_to_live(session_ttl)
            .build();

        let users = Cache::builder()
            .max_capacity(cap)
            .time_to_live(user_ttl)
            .build();

        let list_budget = cap.saturating_mul(1024);
        let user_workspaces = CachedList::new(list_budget, 512, config.user_ttl_secs);
        let workspace_todos = CachedList::new(list_budget, 384, config.user_ttl_secs);

        Self {
            sessions,
            users,
            user_workspaces,
            workspace_todos,
        }
    }

    /// Evicts a cached user row by `user_id`.
    pub async fn invalidate_user(&self, user_id: i32) {
        self.users.invalidate(&user_id).await;
    }

    /// Evicts a cached session entry by SHA-256 `hashed_token`.
    pub async fn invalidate_session(&self, hashed_token: &str) {
        self.sessions.invalidate(hashed_token).await;
    }

    /// Evicts a user's cached workspace summary list.
    pub async fn invalidate_workspace_list(&self, user_id: i32) {
        self.user_workspaces.invalidate(user_id).await;
    }

    /// Evicts all cached Todo items for `workspace_id`.
    pub async fn invalidate_todos(&self, workspace_id: i32) {
        self.workspace_todos.invalidate(workspace_id).await;
    }
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn cached_list_caches_and_invalidates() {
        let list: CachedList<String> = CachedList::new(64_000, 128, 60);
        let calls = Arc::new(AtomicUsize::new(0));

        let c1 = calls.clone();
        let first = list
            .get_or_load(7, || async move {
                c1.fetch_add(1, Ordering::SeqCst);
                Ok(vec!["alpha".to_string(), "beta".to_string()])
            })
            .await
            .expect("first load");
        assert_eq!(first, vec!["alpha", "beta"]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let c2 = calls.clone();
        let second = list
            .get_or_load(7, || async move {
                c2.fetch_add(1, Ordering::SeqCst);
                Ok(vec!["should-not-run".to_string()])
            })
            .await
            .expect("cached load");
        assert_eq!(second, vec!["alpha", "beta"]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        list.invalidate(7).await;
        let c3 = calls.clone();
        let third = list
            .get_or_load(7, || async move {
                c3.fetch_add(1, Ordering::SeqCst);
                Ok(vec!["reloaded".to_string()])
            })
            .await
            .expect("reloaded after invalidate");
        assert_eq!(third, vec!["reloaded"]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cache_service_helpers_clear_expected_keys() {
        let cache = CacheService::new(&CacheConfig::default());
        cache.sessions.insert("hashed_tok".to_string(), 42).await;
        assert_eq!(cache.sessions.get("hashed_tok").await, Some(42));

        cache.invalidate_session("hashed_tok").await;
        assert_eq!(cache.sessions.get("hashed_tok").await, None);
    }
}
