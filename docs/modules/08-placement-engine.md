# 08 Placement Engine 模块设计

- 所有权：Rust
- crate：`crates/agentos-domain::placement`

## 目标

提供确定性的网格吸附、边界、碰撞、自动放置和 Surface 高度扩展计算。

## 非目标

不修改文档、不处理 pointer event、不决定 z-index 业务语义、不执行动画或像素布局。

## 坐标模型

```rust
pub struct GridPoint { pub x: u32, pub y: u32 }
pub struct GridSize { pub width: NonZeroU32, pub height: NonZeroU32 }
pub struct GridRect { pub origin: GridPoint, pub size: GridSize }
pub struct SurfaceBounds { pub columns: NonZeroU32, pub rows: NonZeroU32 }
```

逻辑坐标零基；WebView 负责转换为 CSS Grid。Engine 不接收屏幕像素和缩放比例。

## 接口

```rust
pub trait PlacementEngine {
    fn intersects(&self, left: GridRect, right: GridRect) -> bool;
    fn validate_rect(
        &self,
        bounds: SurfaceBounds,
        occupied: &[GridRect],
        candidate: GridRect,
        ignored: Option<ElementId>,
    ) -> Result<(), PlacementError>;
    fn find_first_fit(&self, request: PlacementRequest) -> PlacementResult;
}
```

## 自动放置

默认使用 row-major first fit：从左到右、从上到下扫描。无可用区域时返回需要扩展的最小 rows 和位置，由 Operation Engine 决定是否允许扩展。

确定性要求：occupied rect 先按 `y/x/elementId` 排序；相同输入必须得到相同结果。

## 碰撞语义

- 边接触不算重叠。
- resize/move 可通过 ignored element 排除自身旧 Rect。
- MVP 禁止普通 Element 重叠。
- Shell overlay 和编辑手柄不属于领域 Rect，不参与碰撞。
- zIndex 不允许绕过碰撞规则。

## 拖动吸附

前端可做视觉吸附，但提交时必须传逻辑 GridRect。若未来支持自由像素，需新增坐标模式 ADR，不能改变现有 Rect 语义。

## 限制

设置最大 columns、rows、Element 数量和扫描面积，避免 Agent 构造高成本请求。超限返回 `PlacementLimitExceeded`。

## 错误

`OutOfBounds`、`Collision { with }`、`NoSpace`、`InvalidBounds`、`PlacementLimitExceeded`。错误只引用领域 ID，不携带 DOM 或像素信息。

## 测试

- 边界接触、包含、部分交叉和零尺寸拒绝。
- first fit 的固定案例与 property tests。
- occupied 输入顺序不影响结果。
- 扩展 rows 为最小值。
- 大量元素和恶意尺寸的性能上限。

## 验收标准

1. Engine 是纯函数式且无 I/O。
2. 同一输入在不同平台结果一致。
3. 所有成功位置均通过 `validate_rect`。
4. 自动放置不会改变已有 Element Rect。
