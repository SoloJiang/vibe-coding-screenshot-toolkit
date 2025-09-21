# platform_mac 模块技术设计

## 职责
专注于交互式框选截图，完整支持多显示器环境和跨显示器截图。基于自研 UI overlay 提供的区域选择，进行精确的截图裁剪和多显示器合成。

## 能力
- **交互式截图**：集成 ui_overlay 框选器，用户交互选择区域后进行截图
- **多显示器感知**：自动检测多显示器环境，构建完整的虚拟桌面坐标系统
- **跨显示器支持**：✅ **已实现** 支持选择和捕获跨越多个显示器的区域
- **虚拟桌面合成**：自动合成所有显示器为统一的虚拟桌面截图
- **智能区域处理**：根据选择区域自动判断是单显示器裁剪还是跨显示器合成
- **剪贴板集成**：写入 PNG 到 macOS NSPasteboard

## 架构设计

### 虚拟桌面坐标系统
通过 `VirtualDesktop` 和 `DisplayInfo` 结构体管理多显示器环境：

```rust
pub struct VirtualDesktop {
    pub displays: Vec<DisplayInfo>,           // 所有显示器信息
    pub total_bounds: VirtualBounds,          // 虚拟桌面总边界
}

pub struct DisplayInfo {
    pub id: u32,                              // 显示器唯一ID
    pub name: String,                         // 显示器名称
    pub is_primary: bool,                     // 是否为主显示器
    pub x: i32, pub y: i32,                   // 在虚拟桌面中的位置
    pub width: u32, pub height: u32,          // 像素尺寸
    pub scale_factor: f64,                    // DPI缩放因子
}
```

### 多显示器捕获策略

**1. 虚拟桌面检测 (`VirtualDesktop::detect()`)**
- 枚举所有显示器并收集元数据
- 计算虚拟桌面的总边界框（支持负坐标）
- 建立统一的坐标映射系统

**2. 交互式选择流程**
- 在主显示器上启动 ui_overlay 交互界面
- 使用 `select_with_virtual_background()` 方法支持虚拟桌面坐标
- 返回的Region坐标已转换为虚拟桌面全局坐标

**3. 智能截图处理**
- **单显示器情况**：直接从对应显示器截图中裁剪，性能最优
- **跨显示器情况**：调用 `capture_all()` 合成虚拟桌面，然后裁剪目标区域

### 捕获方法实现

#### `capture_all()` - 多显示器合成
```rust
pub fn capture_all() -> Result<Screenshot>
```
- 捕获所有显示器并合成为完整的虚拟桌面截图
- 每个显示器按其在虚拟坐标系中的位置放置到画布上
- 支持显示器之间的间隙和负坐标偏移

#### `capture_region_interactive_custom()` - 全屏多显示器交互截图
```rust
pub fn capture_region_interactive_custom(
    selector: &dyn ui_overlay::RegionSelector,
) -> Result<Screenshot>
```
- 使用完整虚拟桌面作为交互背景
- 创建跨越所有显示器的选择界面
- 用户可以看到并选择任意显示器上的内容
- 支持跨显示器区域选择和精确裁剪

## UI Overlay 集成

### 虚拟桌面选择器扩展
`RegionSelector` trait 新增虚拟桌面支持：

```rust
fn select_with_virtual_background(
    &self,
    rgb: &[u8], width: u32, height: u32,
    virtual_bounds: (i32, i32, u32, u32),  // 虚拟桌面边界
    display_offset: (i32, i32),             // 当前显示器偏移
) -> MaybeRegion
```

- 在主显示器上进行交互选择
- 自动将本地坐标转换为虚拟桌面全局坐标
- 支持跨显示器的区域选择和可视化

## 捕获策略 (基于 XCap)
使用 `xcap` (基于 CoreGraphics) 获取显示器图像：

**xcap 路径说明：**
- 成熟稳定的 Rust 生态库
- 基于 CoreGraphics，性能优秀
- 完善的多显示器支持，包含坐标和元数据
- 社区活跃，维护良好

**多显示器处理：**
- 通过 `Monitor::all()` 枚举所有显示器
- 获取每个显示器的位置、尺寸、DPI等完整信息
- 支持复杂的多显示器配置（不同DPI、旋转、负坐标等）

## 组件
- `VirtualDesktop`：虚拟桌面坐标系统管理
- `DisplayInfo`：单个显示器信息封装
- `MacCapturer`：核心截图实现，支持多显示器
- `MacClipboard`：剪贴板集成

## 错误映射
权限不足 -> E_NO_PERMISSION；捕获失败 -> E_CAPTURE_FAIL；用户取消 -> 用户友好的取消消息；跨显示器合成失败 -> E_CAPTURE_FAIL。

## 性能优化
| 场景 | 策略 | 性能 |
|------|------|------|
| 单显示器选择 | 直接裁剪，避免全虚拟桌面合成 | 最优 |
| 跨显示器选择 | 按需合成虚拟桌面，缓存可复用 | 良好 |
| 多显示器检测 | 启动时一次性检测，缓存结果 | 高效 |
| 坐标转换 | 预计算偏移，避免重复计算 | 快速 |

## 已解决的技术挑战
| 挑战 | 解决方案 |
|------|----------|
| 多显示器坐标映射 | ✅ 虚拟桌面坐标系统，支持负坐标和间隙 |
| 跨显示器区域选择 | ✅ 扩展RegionSelector，自动坐标转换 |
| 显示器配置复杂性 | ✅ 完整的DisplayInfo元数据收集 |
| 性能vs功能平衡 | ✅ 智能策略选择（单显示器优化，跨显示器按需） |
| DPI差异处理 | ✅ 记录并使用每个显示器的scale_factor |

## 风险与缓解
| 风险 | 缓解 |
|------|------|
| 权限阻塞 | 首次失败提示用户授权；授权后重试 |
| 显示器配置动态变化 | 每次捕获时重新检测虚拟桌面配置 |
| 大尺寸虚拟桌面内存消耗 | 单显示器情况下避免虚拟桌面合成 |
| 跨显示器选择复杂度 | 分层实现：UI交互 + 后端智能处理 |
