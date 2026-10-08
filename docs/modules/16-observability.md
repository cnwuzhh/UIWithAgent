# 16 Observability 模块设计

- 所有权：Rust `tracing`，前端 telemetry adapter
- crate：`crates/agentos-observability`

## 目标

提供跨 WebView IPC、Runtime、AgentSession、Operation、Repository 和 Native Service 的结构化日志、关联 ID、指标、诊断导出和隐私控制。

## 非目标

不作为审计记录的唯一存储、不记录完整用户内容/模型提示/Secret、不依赖特定云监控供应商。

## 关联标识

```text
requestId       单次 IPC/Agent API 请求
sessionId       AgentSession
buildTaskId     构建任务
transactionId   OperationTransaction
confirmationId  用户确认
externalSessionId 本地应用会话
auditId         可向用户展示的诊断引用
```

所有 transport 入口创建或验证 request ID，并建立 tracing span；跨异步任务显式传播 span/context。

## 事件模型

```rust
pub struct DiagnosticEvent {
    pub timestamp: SystemTime,
    pub level: DiagnosticLevel,
    pub component: ComponentId,
    pub event_name: EventName,
    pub correlation: CorrelationIds,
    pub fields: RedactedFields,
}
```

event name 使用稳定命名，例如 `operation.commit.succeeded`，不把动态 ID 放入名称。

## 日志层

- 开发：pretty console + file JSON。
- 生产：滚动 JSONL 文件，默认 info，按大小/日期轮转。
- 用户诊断模式：短期提高指定 component 日志级别，自动过期。
- audit：由 Repository 写不可变业务审计，本模块只关联 audit ID。

## 脱敏

字段按类型分级：public、identifier、user-content、path、secret。Secret 永不记录；user-content 默认只记录长度/hash；path 默认只记录 basename 或类别。禁止对 Agent request/response 使用通用 `Debug` 日志。

## 指标

MVP 本地聚合：启动耗时、snapshot load、validate/commit latency、revision conflict、Agent API deny、Worker duration、model latency/token、SQLite commit、local app launch。默认不上传；导出诊断时生成摘要。

## 前端错误

WebView error boundary 发送结构化 `ui.error`：component、release、request ID 和脱敏 stack。不得发送 DOM 文本、Surface 内容或表单输入。

## 诊断包

用户显式生成 ZIP，包含版本、OS/WebKitGTK、migration 状态、依赖检查、指标摘要和限量日志。生成前显示内容清单；排除数据库、Secret、模型提示和 Surface payload。

## 失败策略

日志写入失败不能阻止领域 commit；降级到 stderr/内存环形缓冲并通知。audit 写入失败属于 commit failure，因为审计与领域事务在同一 Unit of Work。

## 测试

- correlation ID 跨 IPC/Agent/Repository 传播。
- Secret/path/user-content 脱敏 property tests。
- rotation、磁盘满和日志不可写降级。
- 诊断包内容 allowlist 测试。
- tracing subscriber 初始化一次且测试可替换。

## 验收标准

1. 任意 Operation commit 可由 audit ID 关联请求、任务和 transaction。
2. 日志扫描不包含测试 Secret、SQL、完整 executable path 或 prompt。
3. Observability 故障不破坏已验证领域操作。
4. 默认没有网络 telemetry 出站。
