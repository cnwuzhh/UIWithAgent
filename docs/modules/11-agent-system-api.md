# 11 Agent System API 模块设计

- 所有权：Rust
- crate：`crates/agentos-agent-api`
- 协议规范：[Agent System API](../Agent-System-API.md)

## 目标

实现 Agent Worker 使用系统能力的唯一入口：JSON-RPC transport、session/capability guard、scope query、事务预检/提交、确认、事件、限流和审计。

## 非目标

不调用模型、不实现领域规则、不直接访问 SQLx/SQLite、不提供任意 Shell/fs/Tauri command。

## 内部结构

```text
StdioServer
  -> JsonRpcCodec
  -> RequestContextBuilder
  -> SessionGuard
  -> RateLimiter
  -> MethodRouter
      -> QueryFacade
      -> OperationFacade
      -> BuildFacade
      -> NativeRequestFacade
  -> ErrorMapper / AuditSink
```

## 方法所有权

| Facade | 方法 |
| --- | --- |
| System | `system.describeCapabilities` |
| Context | `context.getTask` |
| Query | `surface.get/getTree/resolvePath`, `build.getStatus` |
| Operation | `operation.validate/commit` |
| Build | `build.reportProgress/requestConfirmation` |
| Native Request | `localApp.listAuthorized/requestLaunch` |

协议字段、错误码和限制以权威规范为准，本文件不另建不兼容版本。

## 调用上下文

```rust
pub struct AgentCallContext {
    pub request_id: RequestId,
    pub session_id: AgentSessionId,
    pub build_task_id: BuildTaskId,
    pub agent_id: AgentId,
    pub capabilities: CapabilitySet,
    pub scope: ResourceScope,
    pub deadline: Instant,
}
```

Context 从 Host 管理的 session registry 构造，不信任请求中自报 capability。

## validate/commit

validate 经 scope、Policy、Validator 和 Operation Engine，返回绑定 session、draft hash、base revision、confirmation 和 expiry 的 token。commit 只接受 token/idempotency key，重新检查 session/revision/confirmation 后执行。token 只保存不可逆 hash 或 Host 内部记录，不是数据库事务句柄。

## DTO 隔离

Query Service 将 Domain 映射为脱敏 DTO：无 rowid、表名、可执行路径、Secret 和内部时间戳。Agent 输入 DTO 有严格大小、深度和集合上限。

## 事件与背压

notification 使用有界队列。关键 cancel/session expiry 不可丢弃；普通 progress/revision 可合并。Worker 长时间不读取时终止 session，避免 Host 内存无界增长。

## 错误与审计

所有错误映射为稳定 reason、retryable 和 audit ID。Internal error 对 Agent 隐藏 cause；完整 cause 仅进入脱敏 Host tracing。成功和拒绝调用都审计。

## 架构约束

- Cargo manifest 禁止 `sqlx`、Tauri shell/fs plugin 和 SQLite adapter。
- Handler 只依赖 application service traits。
- stdio codec 与 method handler 分开测试。
- Agent credential 不进入 DTO、prompt 或日志。

## 测试

- 权威协议全部方法 contract tests。
- capability/scope deny、expired/revoked session。
- validate token 篡改、过期、revision conflict 和幂等。
- malformed JSON、oversized frame、unknown method 和 rate limit。
- 架构依赖测试确保无 SQLx。

## 验收标准

1. Agent 无法从任何 API 获得数据库或 Repository 信息。
2. 没有有效 session 的请求在业务服务前被拒绝。
3. 写入只能按 validate/commit 两阶段执行。
4. 每次调用都有 request ID 和 audit ID。
