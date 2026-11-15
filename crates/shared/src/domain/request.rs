// =============================================================================
// SHARED REQUEST HELPERS, SANITIZATION & DOUBLE-OPTION SERDE
// =============================================================================
//
// Provides whitespace normalization, `Option<Option<T>>` JSON PATCH semantics
// (distinguishing omitted fields vs explicit `null` clear vs new value), and
// canonical pagination/sorting query parameters.

// `serde` - Custom deserializer for three-state `Option<Option<T>>` fields.
use serde::{Deserialize, Deserializer, Serialize};

/// Distinguishes omitted JSON keys (`None`), explicit `null` (`Some(None)`), and values (`Some(Some(v))`).
pub fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// Trait implemented by incoming request DTOs to trim/normalize user strings before validation.
pub trait Sanitize {
    /// Normalizes fields in place.
    fn sanitize(&mut self);
}

/// Trims leading/trailing whitespace and collapses internal runs of whitespace to single spaces.
pub fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Trims an optional string and turns empty/whitespace-only inputs into `None`.
pub fn sanitize_opt(opt: &mut Option<String>) {
    if let Some(val) = opt.take() {
        let trimmed = val.trim().to_string();
        if !trimmed.is_empty() {
            *opt = Some(trimmed);
        }
    }
}

/// Normalizes whitespace on a three-state `Option<Option<String>>` field while preserving explicit `null`.
pub fn sanitize_double_opt(opt: &mut Option<Option<String>>) {
    match opt {
        None | Some(None) => {}
        Some(Some(val)) => {
            let trimmed = val.trim().to_string();
            if trimmed.is_empty() {
                *opt = Some(None);
            } else {
                *val = trimmed;
            }
        }
    }
}

/// Sort direction parameter shared across list endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// Common pagination query bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationQuery {
    /// 1-based page index (defaults to `1`).
    pub page: Option<u64>,
    /// Requested page size (defaults to `20`, capped at `100`).
    pub page_size: Option<u64>,
}

impl PaginationQuery {
    /// Returns safe `(page, page_size)` clamped to `[1..]` and `[1..100]`.
    pub fn normalized(&self) -> (u64, u64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        (page, page_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct PatchDto {
        #[serde(default, deserialize_with = "deserialize_double_option")]
        description: Option<Option<String>>,
    }

    #[test]
    fn double_option_distinguishes_omitted_null_and_value() {
        let omitted: PatchDto = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.description, None);

        let cleared: PatchDto = serde_json::from_str(r#"{"description": null}"#).unwrap();
        assert_eq!(cleared.description, Some(None));

        let updated: PatchDto = serde_json::from_str(r#"{"description": " hello "}"#).unwrap();
        assert_eq!(updated.description, Some(Some(" hello ".to_string())));
    }

    #[test]
    fn sanitize_double_opt_clears_whitespace_only_string() {
        let mut field = Some(Some("   ".to_string()));
        sanitize_double_opt(&mut field);
        assert_eq!(field, Some(None));
    }

    #[test]
    fn sanitize_double_opt_preserves_explicit_null_and_omitted() {
        let mut omitted: Option<Option<String>> = None;
        sanitize_double_opt(&mut omitted);
        assert_eq!(omitted, None);

        let mut explicit_null: Option<Option<String>> = Some(None);
        sanitize_double_opt(&mut explicit_null);
        assert_eq!(explicit_null, Some(None));
    }
}
