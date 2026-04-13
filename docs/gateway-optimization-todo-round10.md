# Gateway 优化 TODO Round 10

更新时间：2026-04-13

本轮目标：把 `transport.ts` 中的 `WEB_COMMAND_MAP` 与浏览器导入导出 helper 拆到独立模块，继续压缩 transport 入口职责。

- [x] 新建 web command map 模块
- [x] 让 `transport.ts` 复用共享 command map
- [x] 为 command map 补最小 Node 单测
- [x] 运行关键前端验证并记录结果

本轮验证：

- `pnpm test:runtime`
- `pnpm build:desktop`
