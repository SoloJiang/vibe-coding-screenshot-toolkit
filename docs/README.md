# 文档索引 / Documentation Index

本项目所有 PRD、技术设计与模块任务文档集中在 `docs/` 目录，采用分层 & snake_case 命名。

## 结构 Structure
```
docs/
  prd/
    prd.md
  tech_design/
    overview.md
    core.md infra.md renderer.md services.md api_cli.md api_napi.md
    platform_mac.md platform_win.md ocr_adapter.md privacy.md macros.md
  todo/
    core.md infra.md renderer.md services.md api_cli.md api_napi.md
    platform_mac.md platform_win.md ocr_adapter.md privacy.md macros.md
```

## 快速入口 Quick Links
### PRD
- 产品需求: [prd/prd.md](./prd/prd.md)

### 技术设计 Technical Design
- 总览: [tech_design/overview.md](./tech_design/overview.md)
- Core: [tech_design/core.md](./tech_design/core.md)
- Infra: [tech_design/infra.md](./tech_design/infra.md)
- Renderer: [tech_design/renderer.md](./tech_design/renderer.md)
- Services: [tech_design/services.md](./tech_design/services.md)
- API CLI: [tech_design/api_cli.md](./tech_design/api_cli.md)
- API N-API: [tech_design/api_napi.md](./tech_design/api_napi.md)
- Platform macOS: [tech_design/platform_mac.md](./tech_design/platform_mac.md)
- Platform Windows: [tech_design/platform_win.md](./tech_design/platform_win.md)
- OCR Adapter: [tech_design/ocr_adapter.md](./tech_design/ocr_adapter.md)
- Privacy: [tech_design/privacy.md](./tech_design/privacy.md)
- Macros: [tech_design/macros.md](./tech_design/macros.md)

### 模块任务 TODO
- Core: [todo/core.md](./todo/core.md)
- Infra: [todo/infra.md](./todo/infra.md)
- Renderer: [todo/renderer.md](./todo/renderer.md)
- Services: [todo/services.md](./todo/services.md)
- API CLI: [todo/api_cli.md](./todo/api_cli.md)
- API N-API: [todo/api_napi.md](./todo/api_napi.md)
- Platform macOS: [todo/platform_mac.md](./todo/platform_mac.md)
- Platform Windows: [todo/platform_win.md](./todo/platform_win.md)
- OCR Adapter: [todo/ocr_adapter.md](./todo/ocr_adapter.md)
- Privacy: [todo/privacy.md](./todo/privacy.md)
- Macros: [todo/macros.md](./todo/macros.md)

## 维护约定 Maintenance
- 文档仅描述“当前实现状态”；规划与路线请移至 issue/PR 讨论。
- 新增模块：同时添加 `tech_design/<module>.md` 与 `todo/<module>.md`。
- 重大架构调整：先更新 overview 再更新各模块文档，保持一致性。
- 命名规范：snake_case；模块名对齐 crate。

## 使用指南 Usage Guidelines
1. **查看产品需求**：从 `prd/prd.md` 了解功能规格和验收标准。
2. **理解技术架构**：阅读 `tech_design/overview.md` 获得全局视图，再查看具体模块设计。
3. **参与开发**：查看 `todo/<module>.md` 了解当前任务状态和优先级。
4. **贡献文档**：更新对应模块文档时，保持 PRD、技术设计、TODO 的一致性。

## 文档状态 Document Status
- ✅ PRD/MVP 范围与验收标准
- ✅ 技术设计已覆盖核心模块
- ✅ TODO 列表在持续维护

English readers: see structure above; Chinese files can be machine‑translated if needed. Core code is English‑centric for identifiers.
