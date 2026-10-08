# 06 Operation Engine 模块设计

- 所有权：Rust
- crate：`crates/agentos-domain` 或独立 `agentos-operations`

## 目标

作为修改 `GuiDocument` 的唯一领域入口，执行 OperationTransaction 的规范化、预检、原子应用、影响摘要和逆操作生成。

## 非目标

不做身份认证、用户确认、数据库事务或网络 I/O；不调用 Agent 或 Tauri。

## 接口

```rust
pub trait OperationEngine {
    fn validate(
        &self,
        document: &GuiDocument,
        draft: OperationTransactionDraft,
    ) -> Result<ValidatedTransaction, OperationError>;

    fn apply(
        &self,
        document: &GuiDocument,
        transaction: ValidatedTransaction,
    ) -> Result<ApplyOutcome, OperationError>;
}

pub struct ApplyOutcome {
    pub document: GuiDocument,
    pub inverse: OperationTransaction,
    pub impact: ImpactSummary,
    pub events: Vec<DomainEvent>,
}
```

`ValidatedTransaction` 不能直接跨进程序列化；应用层以签名 validation token 引用它或其规范化哈希。

## 支持操作

- add/remove/reposition/resize Element；
- create/delete Surface；
- update Surface metadata/icon；
- move App entry；
- copy App with copied/empty data；
- create/transition BuildTask 所需的受控内部操作。

## 执行管线

```mermaid
flowchart LR
  Draft --> Normalize
  Normalize --> Validate[Validator]
  Validate --> Simulate[Apply to clone]
  Simulate --> Revalidate
  Revalidate --> Impact
  Impact --> Inverse
  Inverse --> Validated
```

apply 在不可变输入上产生新文档。任一 Operation 失败时不返回部分文档、事件或逆操作。

## 规范化

- ID 由权威 `IdGenerator` 生成或校验 client temporary ID。
- 缺少位置时调用 Placement Engine。
- title、icon、Rect 等转换为值对象。
- operation 顺序保留；后续操作可引用同事务内新建对象的 temporary ID。

## 影响与确认

Engine 只计算事实性 `ImpactSummary`：变更 Surface、增加/删除元素、树移动和不可逆数据影响。Policy Engine 根据 impact 决定确认级别。

## 并发边界

Engine 不读 revision store。Application 在 validate 时记录 base revision，在 commit 时重新加载同 revision 并应用；不匹配返回 conflict。

## 撤销

每个成功 Operation 必须产生逆操作。Copy 的逆操作删除新建副本；Move 恢复入口；删除包含数据时 inverse 可引用 transaction event payload 或 Repository 保存的 before image。超过保留期后标记不可撤销。

## 错误

错误带 operation index、稳定 reason、领域 path 和可恢复性，不包含用户文案。常见错误：invalid reference、collision、out of bounds、cycle、protected root、unsupported operation。

## 测试

- 每个 Operation 的成功/失败/逆操作测试。
- 事务中间失败不产生任何变化。
- `apply(inverse(apply(doc))) == doc` 的 property tests。
- Move/Copy 树和数据语义测试。
- 相同 document/draft 产生相同 outcome（可注入固定 ID generator）。

## 验收标准

1. 没有其他模块可直接修改 `GuiDocument` 内部集合。
2. apply 不执行 I/O 且原子。
3. 每个成功事务提供 impact、events 和 inverse。
4. Validator 在 apply 后再次确认全部文档不变量。
