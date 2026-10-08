# 12 Model Gateway 模块设计

- 所有权：Rust trait + provider adapters
- crate：port 位于 `agentos-application`，adapter 位于独立 infrastructure crate

## 目标

隔离模型供应商 SDK、认证、流式响应、重试、用量和错误，使 Agent Worker 面向统一推理接口。

## 非目标

不提供 AgentOS 系统能力、不提交 Operation、不保存 BuildTask、不把模型输出视为可信领域对象。

## Port

```rust
#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn complete(
        &self,
        request: ModelRequest,
        sink: Box<dyn ModelEventSink>,
        cancellation: CancellationToken,
    ) -> Result<ModelCompletion, ModelError>;
}
```

```rust
pub struct ModelRequest {
    pub model_profile: ModelProfileId,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolSchema>,
    pub limits: ModelLimits,
    pub correlation: ModelCorrelation,
}
```

Tool schema 只描述 Agent API client methods；Gateway 不执行 tool。

## Adapter

MVP 提供：

- `StubModelGateway`：确定性脚本，用于闭环和测试；
- 一个远程 provider adapter，具体供应商通过 ADR 确定；
- 本地模型 adapter 延后，但保持同一 port。

## Secret

Provider credential 以 Secret Service reference 配置。Gateway 在调用时从 Native Secret port 获取，明文只在最小作用域内存在，不进入 SQLite、prompt、error 或 tracing。

## 流式与工具调用

adapter 将供应商事件规范化为 `TextDelta`、`ToolCallStarted`、`ToolArgumentsDelta`、`Usage` 和 `Completed`。Worker 汇总并调用 Agent API；Gateway 不知道 Surface 或 OperationTransaction。

## 超时与重试

- connect/request/idle 分开超时。
- 仅在尚未产生可见输出/工具副作用时自动重试 transient failure。
- respect provider retry-after，并受 BuildTask 总 deadline 限制。
- cancellation 必须中止网络请求和流读取。

## 内容与限制

限制 message 数、总字节、tool schema 大小、输出 token 和工具轮数。日志只记录 provider、model profile、时延和 token usage，不记录用户内容，除非显式诊断模式且经过脱敏。

## 错误

统一为 `Authentication`、`RateLimited`、`Unavailable`、`Timeout`、`InvalidResponse`、`ContextTooLarge`、`Cancelled`。供应商 body 不直接返回 UI/Agent。

## 测试

- Stub 的固定脚本和工具调用序列。
- provider fixture 的 streaming parser。
- timeout、retry-after、取消和断流。
- Secret 不进入 Debug/Display 序列化。
- adapter contract suite 确保一致语义。

## 验收标准

1. 更换 provider 不改变 Build Orchestrator 或 Agent API。
2. Gateway 无 Repository 和 Operation Engine 依赖。
3. 模型输出必须经过 Worker 与 Agent API 校验后才能影响系统。
4. cancellation 在限定时间内终止网络活动。
