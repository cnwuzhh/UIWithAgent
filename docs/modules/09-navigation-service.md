# 09 Navigation Service 模块设计

- 所有权：Rust
- crate：`crates/agentos-application::navigation`

## 目标

管理当前 Surface、唯一父链路径、历史、跨节点链接跳转和文本路径解析，为 Shell 提供稳定导航投影。

## 非目标

不修改 Surface 树、不渲染 breadcrumb、不保存文本对话内容到 `GuiDocument`、不处理外部应用窗口。

## 状态

```rust
pub struct NavigationState {
    pub current_surface_id: SurfaceId,
    pub back_stack: Vec<NavigationLocation>,
    pub forward_stack: Vec<NavigationLocation>,
    pub scroll_positions: HashMap<SurfaceId, ScrollPosition>,
}

pub struct NavigationProjection {
    pub current_surface_id: SurfaceId,
    pub breadcrumb: Vec<SurfacePathSegment>,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}
```

文本导航的 draft/messages 属于 Shell 状态；Service 只返回解析结果和完成跳转。

## 接口

```rust
pub trait NavigationService {
    fn projection(&self, document: &GuiDocument) -> Result<NavigationProjection, NavigationError>;
    fn open_surface(&mut self, document: &GuiDocument, target: SurfaceId) -> Result<NavigationProjection, NavigationError>;
    fn open_link(&mut self, document: &GuiDocument, source: SurfaceId, element: ElementId) -> Result<NavigationProjection, NavigationError>;
    fn resolve_path(&self, document: &GuiDocument, query: &str) -> PathResolution;
    fn go_back(&mut self, document: &GuiDocument) -> Result<NavigationProjection, NavigationError>;
}
```

## 路径语义

breadcrumb 始终由 target 沿 `parentSurfaceId` 到根反向计算。跨节点链接只决定 target，不把 source 插入目标路径。点击 breadcrumb 相当于 `open_surface(segment.surface_id)`。

## 文本解析

1. 标准化空白和路径分隔符。
2. 完整规范路径优先。
3. 当前子树内名称其次。
4. 全树名称匹配最后。
5. 多个候选返回 `Ambiguous`，包含稳定 ID 和完整路径；不自动猜测。

MVP 支持 Unicode 大小写折叠，但不做模糊编辑距离，避免意外跳转。

## 文档变化

Surface 删除或重挂后，Service 在下次 projection 校验当前位置与 history：失效当前位置回到最近有效祖先，否则回根；失效 history 项移除并产生诊断事件。

## 持久化

只持久化最后 `currentSurfaceId` 和有限历史。scroll position 可作为 Shell setting 保存。启动恢复时必须重新验证 ID，不将 breadcrumb 快照视为权威。

## 错误与安全

错误包括 `SurfaceNotFound`、`InvalidLink`、`AmbiguousPath`、`BrokenParentChain`。解析限制查询长度和候选数量；返回纯领域信息，不泄漏数据库数据。

## 测试

- 父链路径、深树、断链和环防御测试。
- 跨树链接后 breadcrumb 使用目标父链。
- 同名 Surface 的 ambiguous 测试。
- history 在删除/重挂后的恢复。
- Unicode 名称和路径分隔符。

## 验收标准

1. 任意成功导航的 breadcrumb 最后一项等于 current Surface。
2. 链接来源不进入目标 breadcrumb。
3. 多匹配路径不自动选择。
4. Service 不修改 `GuiDocument`。
