# 14 Repository 模块设计

- 所有权：Rust traits
- crate：`crates/agentos-repository`

## 目标

定义应用层所需的持久化 ports、Unit of Work、revision 并发和领域记录边界，使应用服务不依赖 SQLx 或数据库 Schema。

## 非目标

不包含 SQL、不决定权限或业务规则、不向 Agent/WebView 暴露 trait、不实现缓存一致性策略。

## Port 设计

```rust
#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn load(&self) -> Result<DocumentSnapshot, RepositoryError>;
    async fn commit(
        &self,
        expected_revision: Revision,
        change_set: ChangeSet,
        audit: AuditMetadata,
    ) -> Result<CommitReceipt, RepositoryError>;
}

#[async_trait]
pub trait BuildTaskRepository: Send + Sync {
    async fn get(&self, id: BuildTaskId) -> Result<Option<BuildTask>, RepositoryError>;
    async fn save_transition(&self, transition: BuildTaskTransition) -> Result<(), RepositoryError>;
}
```

按聚合和用例设计接口，禁止 `query(table, filter)`、通用 CRUD 和泄漏数据库 entity。

## Unit of Work

需要同时提交 Document、BuildTask、operation event 和 audit 时，Application 使用 `UnitOfWorkFactory`：

```rust
#[async_trait]
pub trait UnitOfWork: Send {
    fn documents(&mut self) -> &mut dyn DocumentWriter;
    fn build_tasks(&mut self) -> &mut dyn BuildTaskWriter;
    fn audit(&mut self) -> &mut dyn AuditWriter;
    async fn commit(self: Box<Self>) -> Result<CommitReceipt, RepositoryError>;
    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError>;
}
```

Drop 时 adapter 必须回滚未提交事务。

## ChangeSet

ChangeSet 是领域变化描述，不是 SQL patch：包含 before/after revision、受影响聚合、operation events、inverse 和 checksum。Repository 实现负责映射到表。

## 并发

expected revision 不匹配返回 `Conflict { expected, actual }`，不能 last-write-wins。MVP Runtime 串行化写入，但 Repository 仍必须执行数据库级 compare-and-set。

## 错误

`NotFound`、`Conflict`、`ConstraintViolation`、`CorruptData`、`MigrationRequired`、`Unavailable`、`Internal`。错误不包含 SQL statement 或数据库绝对路径；source 仅进入 Host 日志。

## 数据读取

load 返回领域对象或专用 snapshot，不返回 rowid。Adapter 读出后必须通过 Domain constructor/Validator；损坏时返回诊断，不用默认值覆盖。

## 测试

- 每个 adapter 必须运行共享 repository contract suite。
- revision compare-and-set、rollback 和 Drop rollback。
- 原子提交 Document/BuildTask/Audit。
- corrupt record 映射为 CorruptData。
- 编译依赖测试：port crate 无 SQLx。

## 验收标准

1. Application crate 可使用 fake repository 完成全部服务测试。
2. Repository 接口不出现 SQL、表名、rowid 或 SQLx 类型。
3. 所有领域写入支持 expected revision。
4. 多聚合用例可在单个 Unit of Work 中原子提交。
