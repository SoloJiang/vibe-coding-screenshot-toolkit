# api_cli 模块技术设计

## 职责
基准/诊断 CLI。

## 命令
MVP: capture, capture-region (最小导出)。
Later: capture-bench, export-demo, ocr-bench, diag。

## 输出
JSON 基准统计 avg/p95。

## 风险
| 风险 | 缓解 |
|------|------|
| 基准波动 | 预热+多样本 |
