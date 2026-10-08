# 07 Validator 模块设计

- 所有权：Rust
- crate：`crates/agentos-domain::validation`

## 目标

提供分层、确定性的结构与语义校验，保护 Domain、Operation、Repository 恢复和 API 边界。

## 非目标

不授权调用者、不决定用户确认、不修复数据、不访问数据库或网络。

## 校验层级

```text
L1 Transport Schema   JSON/DTO 字段和联合类型
L2 Value Object       Rect、颜色、标题、timezone 等局部约束
L3 Aggregate          ID、引用、状态组合和 Surface 树
L4 Geometry           边界、重叠和尺寸
L5 Operation          操作前置条件和事务内引用
L6 Persistence Load   migration 后完整文档一致性
```

L1 位于 transport/contracts；本模块权威负责 L2-L6。

## 接口

```rust
pub trait DocumentValidator {
    fn validate_document(&self, document: &GuiDocument) -> ValidationReport;
    fn validate_operation(
        &self,
        document: &GuiDocument,
        operation: &GuiOperation,
        index: usize,
    ) -> ValidationReport;
}

pub struct ValidationIssue {
    pub code: ValidationCode,
    pub severity: Severity,
    pub path: DomainPath,
    pub operation_index: Option<usize>,
    pub details: IssueDetails,
}
```

用户文本在 UI/API error mapper 中本地化，Validator 不产生最终文案。

## 规则注册

规则按稳定 ID 注册，例如：

- `surface.root.exists`
- `surface.parent.valid`
- `surface.parent.acyclic`
- `element.rect.in_bounds`
- `element.rect.no_overlap`
- `app_icon.target.exists`
- `build_task.state.reference_match`
- `time_panel.timezone.iana`

规则执行顺序稳定，报告排序为 path、code、operation index，便于快照和 Agent 消费。

## 性能

完整文档校验构建 ID index、parent adjacency 和空间索引一次。Operation 预检优先增量校验受影响 Surface，commit 前仍执行关键全局规则。MVP 文档规模下完整校验必须在 50 ms 目标内。

## 恢复策略

从 Repository 加载发现 error severity 时拒绝进入可写 Runtime，并返回诊断报告。warning 可只读打开，但不能静默删除未知字段。自动修复必须是显式 migration 或用户确认 Operation。

## 安全

限制 issue details 大小；不包含 Secret、SQL、完整 executable path。处理 Agent 输入时设置最大集合、字符串和递归深度。

## 测试

- 每条规则至少一组正反例。
- parent cycle、深链和断链 property tests。
- timezone、Unicode title、极值 Rect 和元素数量限制。
- 报告顺序稳定性。
- malformed 数据不 panic 的 fuzz test。

## 验收标准

1. 所有领域不变量有稳定 ValidationCode。
2. 报告可同时返回多个问题，不只首错。
3. Validator 无 I/O 和基础设施依赖。
4. 加载损坏数据时不会静默覆盖或 panic。
