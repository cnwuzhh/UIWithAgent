# 20 Agent Ledger 模块设计

- 所有权：Rust application port + SQLite adapter
- 建议 crate：`crates/agentos-agent-ledger`
- 上游：Build Orchestrator、Agent Runtime、Generated App Runtime、Agent System API、Operation Engine
- 下游：Repository、SQLite Adapter、Artifact Store
- 核心原则：Agent 的全部输入、输出、代码与行为必须可按因果顺序追溯，不允许静默丢弃

## 1. 目标

Agent Ledger 是 AgentOS 的权威行为记录，不是普通诊断日志。它必须完整记录：

1. 用户与 Agent 的全部对话；
2. 实际发送给模型的 system/session/turn message、tool schema 和 provider 参数；
3. 模型原始响应、结构化解析结果和修复轮次；
4. tool call 请求、参数、结果、错误和耗时；
5. Agent 生成、修改和删除的每个代码文件版本；
6. Generated App manifest、build/run command 和环境变量名称；
7. stdout、stderr、exit code、取消、超时和 crash；
8. 用户确认请求与决定；
9. validation、Operation draft、commit、revision 和 inverse；
10. BuildTask、AgentSession、Generated App 与 Surface Element binding 的完整关联。

用户应能从任一 BuildTask 打开时间线，查看 Agent 为什么采取某个动作、当时看到了什么、生成了什么，以及系统实际执行了什么。

## 2. 与 Observability 的区别

| 维度 | Observability | Agent Ledger |
| --- | --- | --- |
| 用途 | 性能、错误和运行诊断 | 完整行为审计与重建 |
| 内容 | 默认脱敏、摘要、可采样 | 原始完整内容，不采样、不截断 |
| 失败策略 | 可降级到 stderr/内存 | fail closed，阻止后续 Agent 行为 |
| 生命周期 | 可轮转和短期保留 | 用户删除前持久保留 |
| 完整 prompt/code | 不保存 | 必须保存 |
| 权威性 | 非权威 | Agent 行为唯一事实源 |

Observability event 可以引用 `ledgerEventId`，但不能替代 Ledger。

## 3. 不变量

1. 每个 BuildTask 在第一个模型请求前创建唯一 Ledger stream；
2. stream 内事件使用严格递增 `sequence`，不依赖 wall clock 排序；
3. 每个事件包含 `previousEventHash` 和自身 hash，形成可验证链；
4. 外部副作用必须先写 `*.requested` 或 `*.started` 事件，再执行，再写完成事件；
5. Ledger 写入失败时，不得继续模型调用、tool call、process spawn、确认解析或 Operation commit；
6. conversation、代码、tool 参数和 process output 不采样、不截断；
7. 大内容存 Artifact Store，事件保存 hash、长度、media type 和引用；
8. 生成文件每个 revision 保存完整快照；diff 仅用于展示，不能作为唯一恢复依据；
9. AgentOS crash 后，只有 started 没有 terminal event 的动作标记为 `Interrupted`，不能伪造成功；
10. 所有最终 Operation commit 必须能追溯到模型响应、代码 revision、用户确认和 validation token。

## 4. 数据模型

```rust
pub struct AgentLedgerEvent {
    pub event_id: LedgerEventId,
    pub stream_id: LedgerStreamId,
    pub sequence: u64,
    pub occurred_at: SystemTime,
    pub recorded_at: SystemTime,
    pub event_type: LedgerEventType,
    pub actor: LedgerActor,
    pub correlation: LedgerCorrelation,
    pub causation_event_id: Option<LedgerEventId>,
    pub payload: LedgerPayload,
    pub previous_event_hash: Option<ContentHash>,
    pub event_hash: ContentHash,
}
```

```rust
pub struct LedgerCorrelation {
    pub build_task_id: BuildTaskId,
    pub session_id: Option<AgentSessionId>,
    pub model_request_id: Option<ModelRequestId>,
    pub tool_call_id: Option<ToolCallId>,
    pub generated_app_id: Option<GeneratedAppId>,
    pub package_revision: Option<u64>,
    pub transaction_id: Option<TransactionId>,
    pub confirmation_id: Option<ConfirmationId>,
    pub surface_id: Option<SurfaceId>,
}
```

`causationEventId` 表示“这个事件由哪个事件直接引起”。例如 process start 由用户确认引起，代码修复由 build failure 引起。

## 5. Artifact Store

大文本和二进制不直接塞入事件行：

```rust
pub struct LedgerArtifact {
    pub hash: ContentHash,
    pub byte_length: u64,
    pub media_type: String,
    pub encoding: Option<String>,
    pub storage_kind: ArtifactStorageKind,
}
```

Artifact 按 SHA-256 内容寻址并去重。首期可以存 SQLite BLOB；超过阈值后存 app data 目录的不可变 blob 文件，SQLite 保存索引。

必须保存的 Artifact：

- 完整 assembled prompt 和每条 message；
- model request JSON 与原始 response body；
- tool arguments/result；
- 每个生成文件的完整 bytes；
- package manifest；
- stdout/stderr chunk；
- Operation draft/normalized transaction；
- Generated App output batch；
- export 时使用的 Schema 与 Runtime Descriptor。

Artifact 写入使用临时文件 + fsync + atomic rename。事件只有在 Artifact 持久化成功后才能引用它。

## 6. 事件类型

```rust
pub enum LedgerEventType {
    BuildTaskSubmitted,
    SessionStarted,
    SessionEnded,
    ConversationMessage,
    PromptAssembled,
    ModelRequestStarted,
    ModelResponseReceived,
    ModelDecodeFailed,
    ToolCallRequested,
    ToolCallCompleted,
    ToolCallFailed,
    PackageRevisionCreated,
    GeneratedFileWritten,
    GeneratedFileDeleted,
    CommandProposed,
    ConfirmationRequested,
    ConfirmationResolved,
    ProcessStarted,
    ProcessStdout,
    ProcessStderr,
    ProcessExited,
    OutputValidated,
    ElementBindingProposed,
    OperationValidationCompleted,
    OperationCommitStarted,
    OperationCommitted,
    OperationCommitFailed,
    BuildTaskCompleted,
    BuildTaskFailed,
    BuildTaskCancelled,
    RecoveryMarkedInterrupted,
}
```

事件类型是稳定 enum；动态动作名放 payload。

## 7. 对话记录

`ConversationMessage` 保存精确角色和内容：

```rust
pub struct ConversationMessagePayload {
    pub role: ConversationRole,
    pub source: MessageSource,
    pub content: ArtifactRef,
    pub content_type: String,
    pub provider_message_id: Option<String>,
}
```

必须分别记录：

- 用户原始输入；
- system prompt；
- Runtime Descriptor；
- Skill 正文；
- Surface snapshot；
- tool schema；
- assistant text/reasoning（provider 实际返回多少就保存多少）；
- tool call 与 tool result；
- repair prompt；
- 最终展示给用户的回复。

同时保存 provider 实际 request/response bytes，避免应用层解析遗漏字段。

## 8. 代码版本记录

每次 Agent 产生代码变化时创建 `PackageRevisionCreated`：

```rust
pub struct GeneratedFileSnapshot {
    pub relative_path: String,
    pub content: ArtifactRef,
    pub executable: bool,
    pub deleted: bool,
}
```

每个 package revision 保存完整文件树清单和每个文件内容 hash。相同内容去重，因此完整快照不会重复保存 bytes。

必须记录：

- Agent 提议的原始文件内容；
- materialize 后的实际 bytes；
- formatter、package manager 或 build script 产生的文件变化；
- 用户手工修改；
- 每轮自动修复前后 revision；
- manifest、lockfile 和依赖解析结果。

任何 build/run 都绑定精确 `packageRevision`，不能只指向“最新代码”。

## 9. 行为记录协议

### 9.1 模型调用

```text
ModelRequestStarted(request artifact)
  -> provider call
  -> ModelResponseReceived(response artifact)
```

`ModelRequestStarted` 写入失败则不调用 provider。

### 9.2 Tool call

```text
ToolCallRequested(method + exact args)
  -> Agent System API
  -> ToolCallCompleted(exact result) | ToolCallFailed(error)
```

### 9.3 Process

```text
CommandProposed(program + args + cwd + env names)
  -> ConfirmationResolved(accepted)
  -> ProcessStarted(pid metadata + package revision)
  -> ProcessStdout/ProcessStderr chunks
  -> ProcessExited(code/signal/duration)
```

stdout/stderr 必须完整保存。磁盘写入跟不上时对 pipe 施加 backpressure；不能为了维持进程吞吐而丢日志。

### 9.4 Operation commit

```text
OperationCommitStarted(normalized transaction + token hash)
  -> atomic domain commit + ledger terminal event
  -> OperationCommitted(revision + inverse + auditId)
```

`OperationCommitted` 与领域 revision 尽可能在同一 Unit of Work 写入。无法原子写时，使用可恢复 outbox 并在启动时对账。

## 10. Port

```rust
#[async_trait]
pub trait AgentLedger: Send + Sync {
    async fn create_stream(
        &self,
        build_task_id: BuildTaskId,
        metadata: LedgerStreamMetadata,
    ) -> Result<LedgerStreamId, LedgerError>;

    async fn append(
        &self,
        event: NewLedgerEvent,
        artifacts: Vec<NewArtifact>,
    ) -> Result<LedgerAppendReceipt, LedgerError>;

    async fn read_stream(
        &self,
        stream_id: LedgerStreamId,
        cursor: Option<LedgerCursor>,
    ) -> Result<LedgerPage, LedgerError>;

    async fn verify_stream(
        &self,
        stream_id: LedgerStreamId,
    ) -> Result<LedgerVerification, LedgerError>;
}
```

写入按 stream 串行化。调用方不能自行分配 sequence 或 event hash。

## 11. SQLite Schema

```text
AgentLedgerStreams(
  id, build_task_id, created_at, last_sequence, last_event_hash, status
)

AgentLedgerEvents(
  id, stream_id, sequence, occurred_at, recorded_at,
  event_type, actor, causation_event_id,
  correlation_json, payload_json,
  previous_event_hash, event_hash
)

AgentLedgerArtifacts(
  hash, byte_length, media_type, encoding, storage_kind, inline_blob, file_name
)

AgentLedgerEventArtifacts(
  event_id, artifact_hash, relation, ordinal
)
```

约束：

- unique `(stream_id, sequence)`；
- `previous_event_hash` 必须等于 stream 当前尾 hash；
- Artifact immutable；
- event immutable，无 update/delete API；
- stream 删除只能由显式用户删除整个 BuildTask 审计记录触发。

## 12. 完整性与删除

哈希链用于检测意外修改和缺失，不宣称抵御拥有本机数据库写权限的攻击者。后续可加入 OS key 签名 checkpoint。

PoC 默认保留全部 Ledger，直到用户显式删除。删除前显示：

- 涉及的 BuildTask 和 Generated App；
- event 与 Artifact 数量；
- 总字节数；
- 删除后哪些行为将不可追溯。

删除操作本身写入独立的系统审计 stream；被删除内容无法继续存在于原 stream，因此 UI 必须明确显示这是用户主动擦除，不是正常轮转。

## 13. Secret 与敏感内容

Ledger 不对用户对话、模型内容、生成代码或进程输出做脱敏，否则无法保证完整追溯。这些内容按本地敏感数据处理：

- 数据目录权限仅当前 OS 用户可读；
- 默认不上传，不进入普通诊断包；
- 导出必须由用户显式触发并显示包含完整内容；
- UI 查看代码和对话时标记“可能包含敏感信息”；
- 后续增加本地加密与锁屏保护。

由 Secret Service 在执行时注入、且从未暴露给 Agent 的 Secret 值不写 Ledger；只记录 credential ref 和环境变量名称。如果 Secret 已经出现在对话、生成代码或 stdout/stderr 中，Ledger仍按原文保存，以维持完整性。

## 14. 查询与调试 UI

Shell 提供按 BuildTask 的 Agent Ledger 调试界面：

- 时间线：用户、模型、tool、代码、process、确认、Operation；
- 对话视图：按实际模型轮次展示 message；
- 代码视图：package revision 文件树、完整源码和 revision diff；
- Process 视图：command、stdout/stderr、exit；
- 因果跳转：从错误跳到触发修复的模型请求；
- 完整性状态：sequence 缺口、hash 验证和 interrupted action；
- 导出：完整 JSONL event + Artifact bundle。

任何摘要都必须能展开到原始 Artifact。

## 15. 故障与恢复

- Ledger unavailable：暂停 BuildTask，不允许继续行为；
- 磁盘满：停止读取 process pipe 以施加 backpressure，尝试终止进程并记录可写范围内的故障；
- Artifact 已写、event 未写：启动时垃圾回收无引用 Artifact；
- event started、无 terminal：恢复时追加 `RecoveryMarkedInterrupted`；
- hash chain mismatch：stream 标记 Corrupt，只读展示，禁止继续该 session；
- UI 查询失败不影响已经持久化的 Ledger，但禁止把“无记录”显示为“无行为”。

## 16. 测试

### 完整性

- sequence 并发追加严格单调；
- previous hash 与 event hash 可重复验证；
- 删除或修改任一 event 后 verification 失败；
- Artifact hash 与 bytes 不一致时失败；
- crash 点覆盖 Artifact、event、side effect 三个阶段。

### 完整记录

- 每次 model request/response 均有原始 Artifact；
- 每次 tool call 均有 request 和 terminal event；
- 每个 package revision 可还原完整文件树；
- stdout/stderr 大流量无丢失且顺序稳定；
- confirmation、process、Operation 可沿 causation 链回到用户请求；
- Ledger 写失败时 provider、tool、process 和 commit 均未发生。

### 查询

- 由 buildTaskId 找到唯一 stream；
- timeline 分页不跳过或重复 sequence；
- Artifact 去重不影响 revision 恢复；
- export 后在空库中可验证并还原。

## 17. 实施切片

### L1：对话与模型调用

- 建立 stream/event/artifact port 和内存实现；
- 记录用户输入、assembled prompt、model request/response；
- planning UI 增加只读 Ledger timeline；
- Ledger 失败时不调用模型。

### L2：代码 revision

- 记录 Generated App package 全量快照；
- 文件树、源码和 revision diff UI；
- build command 绑定 package revision。

### L3：Process 与修复循环

- 完整 stdout/stderr chunk；
- process lifecycle 与 build failure；
- failure -> repair prompt 因果链。

### L4：Operation 与持久化

- SQLite schema、Artifact Store 和 hash verification；
- confirmation、validation、commit 与 Ledger 关联；
- export、删除和 crash recovery。

## 18. 验收标准

1. 任一 BuildTask 可以重建完整用户/Agent 对话；
2. 任一模型轮次可以查看实际 request、response 和 tool schema；
3. 任一 build/run 可以还原精确代码 revision 与实际 command；
4. stdout、stderr、tool result 和错误不截断、不采样；
5. 任一 Surface commit 可以追溯到用户请求、模型响应、确认和代码 revision；
6. Ledger 写失败时系统 fail closed，不产生未记录 Agent 行为；
7. hash verification 可以检测事件缺失、重排和 Artifact 修改；
8. 完整 Ledger 默认本地保存，只有用户显式操作才能导出或删除。
