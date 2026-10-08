# 05 GUI Domain 模块设计

- 所有权：Rust
- crate：`crates/agentos-domain`

## 目标

定义 AgentOS 的权威领域语言、值对象、聚合和不变量，为 Runtime、Operation、Repository 和公共 DTO 提供稳定语义。

## 非目标

不依赖 Tauri、Tokio、SQLx、Serde transport、React 或模型 SDK；不执行 I/O、权限判断和持久化。

## 领域对象

```text
GuiDocument
├── DocumentId / Revision
├── SurfaceTree
│   └── Surface
│       ├── SurfaceIcon
│       └── Element
├── BuildTask
├── LocalAppDefinition
└── OperationHistory
```

核心类型使用 newtype，避免跨 ID 混用：

```rust
pub struct SurfaceId(Uuid);
pub struct ElementId(Uuid);
pub struct BuildTaskId(Uuid);
pub struct Revision(u64);

pub struct Surface {
    pub id: SurfaceId,
    pub parent_id: Option<SurfaceId>,
    pub title: SurfaceTitle,
    pub icon: SurfaceIcon,
    pub size: SurfaceSize,
    pub elements: Vec<Element>,
}
```

## 聚合边界

`GuiDocument` 是 MVP 的一致性聚合，OperationTransaction 在单个 revision 上变更它。Repository 可拆表持久化，但加载后必须恢复完整聚合并通过 Validator。

## Element 联合

```rust
pub enum Element {
    TimePanel(TimePanel),
    AppIcon(AppIcon),
    Unsupported(UnsupportedElement),
}
```

`Unsupported` 只用于前向兼容读取，不能由新 Operation 主动创建。开放插件模型延后到独立 ADR。

## 不变量

1. `root_surface_id` 存在且无 parent。
2. 其他 Surface 恰有一个 parent，父链无环。
3. Surface、Element 和 BuildTask ID 在文档内唯一。
4. Surface target 与 BuildTask 引用有效。
5. ready AppIcon 有 target；building/failed AppIcon 有 build task。
6. Rect、SurfaceSize、颜色和 IANA timezone 是构造时校验的值对象。
7. Domain 不保存数据库 rowid、SQL 时间戳或 transport metadata。

## 领域事件

```rust
pub enum DomainEvent {
    SurfaceCreated { surface_id: SurfaceId },
    SurfaceChanged { surface_id: SurfaceId },
    SurfaceDeleted { surface_id: SurfaceId },
    BuildTaskChanged { task_id: BuildTaskId },
    LocalAppChanged { local_app_id: LocalAppId },
}
```

事件只描述已发生事实；request ID、audit ID 由应用层 envelope 添加。

## 错误

Domain error 使用稳定 enum，例如 `InvalidRect`、`DuplicateId`、`MissingTarget`、`SurfaceCycle`。错误不包含用户文案、SQL 或 backtrace。

## 版本与兼容

`DocumentSchemaVersion` 与 API version 分离。旧文档先由 SQLite Adapter migration 转换，再构造领域对象。Domain 不维护数据库 migration。

## 测试

- 值对象边界和 property-based tests。
- 任意 Surface parent 图的无环属性测试。
- Element 状态/引用组合测试。
- serialization contract 在 contracts crate 测试，不污染 Domain。
- 编译期依赖检查确保无 infrastructure crate。

## 验收标准

1. `cargo tree -p agentos-domain` 不含 Tauri、Tokio、SQLx。
2. 非法值无法通过公开构造器创建。
3. 相同输入产生相等领域对象和稳定事件。
4. Domain error 可完整映射为 Validator/API 错误。
