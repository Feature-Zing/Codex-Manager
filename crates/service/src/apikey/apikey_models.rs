use std::collections::HashSet;

use codexmanager_core::rpc::types::ApiKeyModelListResult;
use codexmanager_core::rpc::types::ModelOption;
use codexmanager_core::storage::now_ts;

use crate::gateway;
use crate::storage_helpers;

const MODEL_CACHE_SCOPE_DEFAULT: &str = "default";

/// 函数 `read_model_options`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn read_model_options(refresh_remote: bool) -> Result<ApiKeyModelListResult, String> {
    let cached = read_cached_model_options()?;
    if !refresh_remote {
        return Ok(ApiKeyModelListResult { items: cached });
    }

    match gateway::fetch_models_for_picker_with_source("ui_model_refresh") {
        Ok(items) => {
            let (merged_items, changed) = merge_model_options(&cached, &items);
            if changed {
                let _ = save_model_options_cache(&merged_items);
            }
            Ok(ApiKeyModelListResult {
                items: merged_items,
            })
        }
        Err(err) => {
            if !cached.is_empty() {
                return Ok(ApiKeyModelListResult { items: cached });
            }
            Err(err)
        }
    }
}

/// 函数 `save_model_options_cache`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - items: 参数 items
///
/// # 返回
/// 返回函数执行结果
fn save_model_options_cache(items: &[ModelOption]) -> Result<(), String> {
    let storage =
        storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let items_json = serde_json::to_string(items).map_err(|e| e.to_string())?;
    storage
        .upsert_model_options_cache(MODEL_CACHE_SCOPE_DEFAULT, &items_json, now_ts())
        .map_err(|e| e.to_string())
}

/// 函数 `read_cached_model_options`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn read_cached_model_options() -> Result<Vec<ModelOption>, String> {
    let storage =
        storage_helpers::open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let Some(cache) = storage
        .get_model_options_cache(MODEL_CACHE_SCOPE_DEFAULT)
        .map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    let items = serde_json::from_str::<Vec<ModelOption>>(&cache.items_json).unwrap_or_default();
    Ok(normalize_model_options(&items))
}

/// 函数 `merge_model_options`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - cached: 参数 cached
/// - fetched: 参数 fetched
///
/// # 返回
/// 返回函数执行结果
fn merge_model_options(
    cached: &[ModelOption],
    fetched: &[ModelOption],
) -> (Vec<ModelOption>, bool) {
    let mut merged = normalize_model_options(cached);
    let mut seen = cached
        .iter()
        .map(|item| item.slug.trim())
        .filter(|slug| !slug.is_empty())
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let mut changed = false;

    for item in fetched {
        let slug = item.slug.trim();
        if slug.is_empty() || !seen.insert(slug.to_string()) {
            continue;
        }

        merged.push(ModelOption {
            slug: slug.to_string(),
            display_name: slug.to_string(),
        });
        changed = true;
    }

    (merged, changed)
}

/// 函数 `normalize_model_options`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - items: 参数 items
///
/// # 返回
/// 返回函数执行结果
fn normalize_model_options(items: &[ModelOption]) -> Vec<ModelOption> {
    items
        .iter()
        .filter_map(|item| {
            let slug = item.slug.trim().to_string();
            if slug.is_empty() {
                return None;
            }
            Some(ModelOption {
                slug: slug.clone(),
                display_name: slug,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use codexmanager_core::rpc::types::ModelOption;

    use super::merge_model_options;

    /// 函数 `as_pairs`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - items: 参数 items
    ///
    /// # 返回
    /// 返回函数执行结果
    fn as_pairs(items: &[ModelOption]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|item| (item.slug.clone(), item.display_name.clone()))
            .collect()
    }

    /// 函数 `merge_model_options_appends_only_new_models`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn merge_model_options_appends_only_new_models() {
        let cached = vec![
            ModelOption {
                slug: "gpt-4.1".to_string(),
                display_name: "GPT-4.1".to_string(),
            },
            ModelOption {
                slug: "gpt-5".to_string(),
                display_name: "GPT-5".to_string(),
            },
        ];
        let fetched = vec![
            ModelOption {
                slug: "gpt-5".to_string(),
                display_name: "GPT-5 Latest".to_string(),
            },
            ModelOption {
                slug: "o3".to_string(),
                display_name: "o3".to_string(),
            },
        ];

        let (merged, changed) = merge_model_options(&cached, &fetched);

        assert!(changed);
        assert_eq!(
            as_pairs(&merged),
            vec![
                ("gpt-4.1".to_string(), "gpt-4.1".to_string()),
                ("gpt-5".to_string(), "gpt-5".to_string()),
                ("o3".to_string(), "o3".to_string()),
            ]
        );
    }

    /// 函数 `merge_model_options_keeps_cache_when_remote_has_no_new_items`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn merge_model_options_keeps_cache_when_remote_has_no_new_items() {
        let cached = vec![ModelOption {
            slug: "gpt-5".to_string(),
            display_name: "GPT-5".to_string(),
        }];
        let fetched = vec![
            ModelOption {
                slug: "gpt-5".to_string(),
                display_name: "GPT-5 Latest".to_string(),
            },
            ModelOption {
                slug: " ".to_string(),
                display_name: "".to_string(),
            },
        ];

        let (merged, changed) = merge_model_options(&cached, &fetched);

        assert!(!changed);
        assert_eq!(
            as_pairs(&merged),
            vec![("gpt-5".to_string(), "gpt-5".to_string())]
        );
    }
}
