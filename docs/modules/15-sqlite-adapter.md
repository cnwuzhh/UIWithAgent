# 15 SQLite Adapter 模块设计

- 所有权：SQLx
- crate：`crates/agentos-sqlite`

## 目标

实现 Repository ports、Schema migration、SQLite transaction、领域映射、备份和损坏诊断。

## 非目标

不向 Agent/UI 暴露 SQL、不实现领域校验或 Policy、不保存明文 Secret、不把数据库 entity 传播到应用层。

## 数据库设置

连接初始化执行并验证：

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
```

MVP 使用单写者队列和受限读连接。数据库位于 Tauri app data dir，权限为当前用户私有。

## Schema

```text
schema_migrations
Documents(id, revision, schema_version, checksum)
Surfaces(id, document_id, parent_surface_id, title, icon_json, width, height)
Elements(id, surface_id, type, rect_json, payload_json, z_index)
BuildTasks(...)
OperationTransactions(...)
OperationEvents(...)
LocalApps(...)
ExternalAppSessions(...)
Settings(...)
AuditRecords(...)
```

实际命名统一 snake_case。父子关系和引用使用 foreign key；开放 Element payload 使用 versioned JSON。

## 映射

SQL row 先映射为 persistence record，再通过 mapper 构造 Domain。Mapper 显式处理 enum/version，禁止 `unwrap`。Domain ID 存 UUID 文本或 16-byte blob，选型需在首个 migration 固定。

## Commit

```mermaid
flowchart LR
  Begin --> CompareRevision
  CompareRevision --> WriteAggregates
  WriteAggregates --> WriteEvents
  WriteEvents --> WriteAudit
  WriteAudit --> IncrementRevision
  IncrementRevision --> Commit
```

任一步失败 rollback。compare revision 必须在同一 transaction 中。CommitReceipt 只在数据库 commit 成功后返回。

## Migration

- migration 编译进应用并按版本只向前执行。
- 启动升级前创建一致性备份。
- migration 在独立 transaction 中；失败阻止进入可写状态。
- CI 从空库和上一发布版本升级测试。
- destructive migration 先 copy/verify，再删除旧列或表。

## 备份与恢复

使用 SQLite backup API 或安全 checkpoint 后复制，不直接复制活跃 WAL 主文件。保留有限版本并记录 checksum。恢复是显式用户动作，原损坏库先隔离保存。

## 安全

- 所有值使用绑定参数。
- Secret 只保存 SecretRef。
- SQLx query error 对上层脱敏。
- Agent API crate 依赖图不得到达本 crate。
- 导出诊断不包含完整用户数据表。

## 测试

- 共享 Repository contract suite。
- 每个 migration 的 up 测试和历史 fixture 升级。
- foreign key、revision conflict、rollback、WAL restart。
- 模拟损坏/缺列/checksum mismatch。
- 大事务和并发读取性能基线。

## 验收标准

1. 空数据库可一次迁移到最新版本。
2. commit 原子写入 change、event、audit 和 revision。
3. crash/restart 后不存在半提交领域状态。
4. 上层错误和 Agent 响应不包含 SQL 或数据库路径。
