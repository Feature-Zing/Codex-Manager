<p align="center">
  <img src="assets/logo/logo.png" alt="CodexManager Logo" width="220" />
</p>

<h1 align="center">CodexManager</h1>

<p align="center">A local desktop app + service process manager and gateway relay for Codex accounts.</p>

<p align="center">
  <a href="README.md">中文</a>|
  <a href="https://github.com/qxcnm/Codex-Manager">GitHub Main Repository</a>|
  <a href="https://qxnm.top">Official Website</a>|
  <a href="#sponsors">Sponsors</a>
</p>

<p align="center"><strong>A local desktop app + service process account pool manager for Codex</strong></p>
<p align="center">Manage accounts, usage, and platform keys in one place, with a built-in local gateway.</p>

## Recognized Community
<p align="left">
  <a href="https://linux.do/t/topic/1688401" title="LINUX DO">
    <img
      src="https://cdn3.linux.do/original/4X/d/1/4/d146c68151340881c884d95e0da4acdf369258c6.png"
      alt="LINUX DO"
      width="100"
      hight="100"
    />
  </a>
</p>

## Source Notes
> This project was built under my direction with AI assistance: Codex (98%) and Gemini (2%). If you run into issues while using it, please communicate in a friendly way. I open-sourced it because I thought it could help someone, and the core functionality is already usable.
> I also do not have enough environments to verify every package on every platform. I still have a day job, and I cannot afford devices like Macs, so I only guarantee availability for the Windows desktop app. If there are issues on other platforms, please report them in the community group or submit Issues after sufficient testing. I will handle them when I have time.
> Finally, thanks to everyone who reported platform-specific issues in the group and helped with part of the testing.

## Disclaimer

- This project is for learning and development purposes only.

- Users must comply with the terms of service of the relevant platforms, such as OpenAI and Anthropic.

- The author does not provide or distribute any accounts, API keys, or proxy services, and is not responsible for specific usage of this software.

- Do not use this project to bypass rate limits or service restrictions.

## Sponsors

Thanks to the following friends and partners for supporting CodexManager.
Mo Duan Xia: thank you for providing token support. The GPT card service supports self-service purchase and activation, offers stable availability, includes a guarantee, and supports Codex 5.4. Website: [小末AI](https://www.aixiamo.com)

[Wonderdch](https://github.com/Wonderdch), Catch_Bat, [suxinwl](https://github.com/suxinwl), [Hermit](https://github.com/HermitChen), [Suifeng023](https://github.com/Suifeng023), [HK-hub](https://github.com/HK-hub)

## ☕ Support the Project (Support)

If this project has been helpful to you, you are welcome to support the author.
<table>
  <tr>
    <th>Alipay</th>
    <th>WeChat</th>
  </tr>
  <tr>
    <td align="center"><img src="assets/images/AliPay.jpg" alt="Alipay sponsor QR code" width="220" /></td>
    <td align="center"><img src="assets/images/wechatPay.jpg" alt="WeChat sponsor QR code" width="220" /></td>
  </tr>
</table>

## Star History

<a href="https://www.star-history.com/?repos=qxcnm%2FCodex-Manager&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/image?repos=qxcnm/Codex-Manager&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/image?repos=qxcnm/Codex-Manager&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/image?repos=qxcnm/Codex-Manager&type=date&legend=top-left" />
 </picture>
</a>

## Navigation
| What You Want To Do | Go Directly To |
| --- | --- |
| First launch, deployment, Docker, macOS allowlisting | [Runtime and Deployment Guide](docs/report/运行与部署指南.md) |
| Configure ports, proxy, database, Web password, and environment variables | [Environment Variables and Runtime Configuration](docs/report/环境变量与运行配置说明.md) |
| Troubleshoot account matching, import failures, challenge interception, and request errors | [FAQ and Account Matching Rules](docs/report/FAQ与账号命中规则.md) |
| Troubleshoot why scheduled tasks skip accounts or why accounts are disabled | [Scheduled Task Account Skip Notes](docs/report/后台任务账号跳过说明.md) |
| Minimal plugin center integration and quick onboarding | [Plugin Center Minimal Integration Guide](docs/report/插件中心最小接入说明.md) |
| Integrate with the plugin center, view interface lists, market modes, and Rhai interfaces | [Plugin Center Integration and Interface Inventory](docs/report/插件中心对接与接口清单.md) |
| View all internal interfaces exposed by the system | [System Internal Interface Inventory](docs/report/系统内部接口总表.md) |
| Build locally, package, release, and run scripts | [Build, Release, and Script Guide](docs/release/构建发布与脚本说明.md) |

## Recent Changes
  - Current latest version: `v0.1.16` (2026-04-05, pre-release)
  - This release mainly closes out around `service_tier` observability and log consistency. HTTP and WebSocket request logs now record `fast` only when the client explicitly sends `service_tier`, instead of incorrectly treating platform-key defaults as an explicit request choice.
  - The gateway trace now includes a `CLIENT_SERVICE_TIER` diagnostic event that records whether the original HTTP or WebSocket request contained `service_tier`, the raw incoming value, and the normalized value used for logging. This makes it much easier to tell client-sent `fast / priority` apart from server-side defaults.
  - Service-tier display in the logs page is now aligned as well: on-wire `priority` is shown as `fast`, while requests without an explicit tier continue to show `auto`, so HTTP and WebSocket views follow the same rules.
  - Version alignment for this round is complete too: the workspace, frontend package, Tauri desktop app, lockfile, README, and CHANGELOG have all been updated to `0.1.16`.

### Recent Commit Summary
- `b762a65`: fixed `service_tier` log semantics and added raw client-side `service_tier` diagnostics for both HTTP and WebSocket requests.
- `7e7b76f`: separated leftover formatting-only changes into their own cleanup commit.
- `be73359`: adjusted abbreviated token displays to keep two decimal places for more stable number formatting across dashboard, logs, and platform key pages.
- `dfb4494`: merged PR #86, which consolidates fixes for Anthropic SSE tool-call argument compatibility during streaming bridge conversion.
- `981bc6e`: aliased `chat.completion` usage fields to OpenAI `prompt/completion tokens`, reducing usage accounting mismatches.
- `480f847`: fixed a case where empty `edits` in completed events could overwrite streamed edit arguments.
- `7bbc5fc`: fixed `chat/completions` SSE handling so completed tool arguments are merged correctly even when content is non-empty.
- `aa2c09c`: merged streamed tool arguments before Anthropic SSE conversion to avoid losing arguments at completion time.
- `29c3b6b`: prevented placeholder tool arguments from wiping real streamed edit payloads, further hardening streaming tool-call stability.
- `c1844b7`: standardized stream disconnect messaging to a clearer network-jitter style prompt.
- `a89cd9c`: preserved upstream raw error text and tightened log messaging to make real failures easier to diagnose.
- `8d619a0`: added export-for-selected-accounts support and improved account switching during usage refresh.

## Feature Overview
- Account pool management: groups, tags, sorting, notes, ban detection, and banned-account filtering
- Bulk import / export: supports multi-file import, recursive JSON folder import on desktop, and single-file export per account
- Usage display: supports both 5-hour + 7-day dual windows and accounts that only return a 7-day single window, with the corresponding reset times shown
- Authorized login: browser authorization plus manual callback parsing
- Platform keys: create, disable, delete, model binding, reasoning effort, and service tier (`Follow Request` / `Fast` / `Flex`)
- Aggregate API: manage third-party minimal relay upstreams, with create, edit, connectivity testing, provider name, sort priority, and grouped display by Codex / Claude
- Plugin center: route at `/plugins/`, supports built-in curated, enterprise private, and custom-source market modes, and provides plugin lists, tasks, logs, and Rhai integration interfaces
- Settings page: supports the `System Derive` button, single-account concurrency limit, and a more conservative high-concurrency degradation strategy
- System internal interface inventory: lists all currently available desktop commands, service RPC methods, and plugin built-in functions
- Local service: auto-start, customizable port, and listen address
- Local gateway: provides one unified OpenAI-compatible endpoint for CLI tools and third-party tooling

## Screenshots
![Dashboard](assets/images/dashboard.png)
![Account Management](assets/images/accounts.png)
![Platform Keys](assets/images/platform-key.png)
![Aggregate API](assets/images/aggregate-api.png)
![Plugin Center](assets/images/plug.png)
![Log View](assets/images/log.png)
![Settings](assets/images/themes.png)

## Quick Start
1. Launch the desktop app and click `Start Service`.
2. Go to `Account Management`, add an account, and complete authorization.
3. If callback parsing fails, paste the callback URL to complete parsing manually.
4. Refresh usage and confirm the account status.

## Default Data Directory
- By default, the desktop app writes the SQLite database to the app data directory, with the fixed filename `codexmanager.db`.
- Windows: `%APPDATA%\\com.codexmanager.desktop\\codexmanager.db`
- macOS: `~/Library/Application Support/com.codexmanager.desktop/codexmanager.db`
- Linux: `~/.local/share/com.codexmanager.desktop/codexmanager.db`
- If you need to adjust the database, proxy, listen address, or other runtime configuration, continue with [Environment Variables and Runtime Configuration](docs/report/环境变量与运行配置说明.md).

## Page Overview
### Desktop App
- Account Management: centrally import, export, and refresh accounts and usage, with low-quota / banned filters and reset-time display
- Platform Keys: bind platform keys by model, reasoning effort, and service tier, and view invocation logs
- Plugin Center: `/plugins/` route with built-in curated / enterprise private / custom-source market switching, plugin install, enable/disable, tasks, logs, and Rhai integration
- Settings: centrally manage ports, listen address, proxy, theme, auto-update, and background behavior

### Service Edition
- `codexmanager-service`: provides a local OpenAI-compatible gateway
- `codexmanager-web`: provides a browser management interface
- `codexmanager-start`: launches service + web with one command

## Common Documents
- Version history: [CHANGELOG.md](CHANGELOG.md)
- Collaboration guidelines: [CONTRIBUTING.md](CONTRIBUTING.md)
- Architecture notes: [ARCHITECTURE.md](ARCHITECTURE.md)
- Testing baseline: [TESTING.md](TESTING.md)
- Security notes: [SECURITY.md](SECURITY.md)
- Documentation index: [docs/README.md](docs/README.md)

## Topic Pages
| Page | Content |
| --- | --- |
| [Runtime and Deployment Guide](docs/report/运行与部署指南.md) | First launch, Docker, Service edition, macOS allowlisting |
| [Environment Variables and Runtime Configuration](docs/report/环境变量与运行配置说明.md) | App configuration, proxy, listen address, database, Web security |
| [FAQ and Account Matching Rules](docs/report/FAQ与账号命中规则.md) | Account matching, challenge interception, import/export, common exceptions |
| [Scheduled Task Account Skip Notes](docs/report/后台任务账号跳过说明.md) | Background task filtering, disabled accounts, and reasons why a workspace is deactivated |
| [Minimal Troubleshooting Guide](docs/report/最小排障手册.md) | Quickly locate service startup, request forwarding, and model refresh issues |
| [Plugin Center Integration and Interface Inventory](docs/report/插件中心对接与接口清单.md) | Plugin center routes, market modes, Tauri/RPC interfaces, manifest fields, and Rhai built-ins |
| [Build, Release, and Script Guide](docs/release/构建发布与脚本说明.md) | Local build, Tauri packaging, release workflow, and script parameters |
| [Release and Artifact Notes](docs/release/发布与产物说明.md) | Release artifacts for each platform, naming, and pre-release status |
| [Script and Release Responsibility Mapping](docs/report/脚本与发布职责对照.md) | What each script is responsible for and which one to use in each scenario |
| [Protocol Compatibility Regression Checklist](docs/report/协议兼容回归清单.md) | Regression items for `/v1/chat/completions`, `/v1/responses`, and tools |
| [Current Gateway vs Codex Header and Parameter Differences](docs/report/当前网关与Codex请求头和参数差异表.md) | Comparison of current gateway parameter passing, request headers, and request parameters against Codex |
| [System Internal Interface Inventory](docs/report/系统内部接口总表.md) | All internal interfaces exposed by the desktop app, service, and plugin center |
| [CHANGELOG.md](CHANGELOG.md) | Latest release notes, unreleased updates, and full version history |

## Directory Structure
```text
.
├─ apps/                # Frontend and Tauri desktop app
│  ├─ src/
│  ├─ src-tauri/
│  └─ dist/
├─ crates/              # Rust core/service
│  ├─ core
│  ├─ service
│  ├─ start              # One-click starter for the Service edition (launches service + web)
│  └─ web                # Service edition Web UI (can embed static assets + /api/rpc proxy)
├─ docs/                # Official documentation
├─ scripts/             # Build and release scripts
└─ README.md
```

## Acknowledgements and Reference Projects

- Codex (OpenAI): this project references its implementation and source structure for request flows, login semantics, and upstream compatibility behavior <https://github.com/openai/codex>

## Contact
- WeChat Official Account: 七线牛马
- WeChat: ProsperGao

- Community group answer: the project name, `CodexManager`

  <img src="assets/images/qq_group.jpg" alt="Community Group QR Code" width="280" />

- Telegram Group: [CodexManager TG Group](https://t.me/+OdpFa9GvjxhjMDhl)
