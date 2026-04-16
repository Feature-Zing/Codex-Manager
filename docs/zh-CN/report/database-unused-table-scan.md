# 数据库未用表扫描报告

扫描时间：2026-04-16

## 结论

当前没有发现可以直接移除的“业务无引用且仍存在”的表。

本机只读扫描到的实际数据库为：

`C:\Users\<当前用户>\AppData\Roaming\com.codexmanager.desktop\codexmanager.db`

当前实际库共有 25 张表，其中 `sqlite_sequence` 是 SQLite 内部表，`schema_migrations` 是迁移记录表，均不应删除。

## 可移除表

无。

原因：

- 所有仍存在的业务表都能在 storage/service/frontend 或网关链路中找到调用点。
- 空表不等于废表，例如 `plugin_installs`、`plugin_tasks`、`plugin_run_logs` 当前为 0 行，但插件中心、插件任务调度和插件运行日志会使用。
- `account_metadata` 当前为 0 行，但账号导入、导出、列表展示、备注/标签更新会使用。

## 实际库表和行数

| 表名 | 行数 | 结论 |
|---|---:|---|
| account_metadata | 0 | 保留，账号备注/标签元数据 |
| accounts | 7 | 保留，账号主表 |
| aggregate_api_secrets | 1 | 保留，聚合 API 密钥 |
| aggregate_apis | 1 | 保留，聚合 API 配置 |
| api_key_profiles | 2 | 保留，API Key 协议/上游配置 |
| api_key_secrets | 2 | 保留，API Key 明文密钥 |
| api_keys | 2 | 保留，API Key 主表 |
| app_settings | 31 | 保留，应用设置 |
| conversation_bindings | 3 | 保留，会话粘性/线程锚点 |
| events | 930 | 保留，账号状态事件和原因追踪 |
| gateway_error_logs | 23 | 保留，网关诊断日志 |
| login_sessions | 44 | 保留，登录会话 |
| model_catalog_models | 6 | 保留，模型目录主表 |
| model_catalog_reasoning_levels | 24 | 保留，模型 reasoning level |
| model_catalog_scopes | 1 | 保留，模型目录 scope 元数据 |
| model_catalog_string_items | 105 | 保留，模型多值属性汇总表 |
| plugin_installs | 0 | 保留，插件安装表 |
| plugin_run_logs | 0 | 保留，插件运行日志 |
| plugin_tasks | 0 | 保留，插件任务表 |
| request_logs | 119 | 保留，请求日志 |
| request_token_stats | 39929 | 保留，请求 token 统计 |
| schema_migrations | 51 | 保留，迁移记录 |
| sqlite_sequence | 6 | 保留，SQLite 内部自增序列表 |
| tokens | 7 | 保留，账号 token |
| usage_snapshots | 3617 | 保留，用量快照 |

## 已经不存在的历史废表

这些表在历史迁移中已经被 drop，当前实际库中不存在，不需要再单独处理：

| 历史表 | 替代/处理方式 |
|---|---|
| model_options_cache | 已由 `048_drop_model_options_cache.sql` 删除 |
| model_catalog_additional_speed_tiers | 已迁移到 `model_catalog_string_items` |
| model_catalog_experimental_supported_tools | 已迁移到 `model_catalog_string_items` |
| model_catalog_input_modalities | 已迁移到 `model_catalog_string_items` |
| model_catalog_available_in_plans | 已迁移到 `model_catalog_string_items` |

## 后续建议

如果要进一步瘦身，建议做“数据清理策略”，而不是删表：

- 对 `events`、`login_sessions`、`usage_snapshots`、`request_token_stats`、`request_logs`、`gateway_error_logs` 做保留天数或最大行数清理。
- 插件三张空表不建议删除，因为一旦删表且 `schema_migrations` 已标记 `040_plugins`，后续启动不会自动重建，会导致插件功能报错。
