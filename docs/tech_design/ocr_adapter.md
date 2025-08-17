# ocr_adapter 模块技术设计
(迁移自 TechDesign_ocr_adapter.md)

## 职责
封装 tesseract：初始化、预处理、识别、缓存。

## 预处理
resize_if_large -> 灰度 -> OTSU -> 中值去噪。

## 线程池
固定 N; channel 提交 -> Future。

## 缓存
LRU(16) 键=尺寸+crc32。

## 风险
| 风险 | 缓解 |
|------|------|
| 初始化慢 | 懒加载+持久线程 |
| 多语言准确度低 | 置信度回退 |
