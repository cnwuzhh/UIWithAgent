# 04 IPC Client 模块设计

- 所有权：TypeScript
- 位置：`apps/desktop/src/ipc`

## 目标

封装 WebView 到 Rust Host 的全部 Tauri IPC，提供生成类型、运行时响应校验、request ID、超时、取消和事件订阅。

## 非目标

- 不包含业务规则或权限判断；
- 不暴露通用 `invoke(command: string)` 给业务组件；
- 不接受 SQL、Shell、任意文件路径或 Repository 参数；
- 不自动重试非幂等提交。

## 公共接口

```ts
export interface AgentOsIpcClient {
  getRuntimeSnapshot(signal?: AbortSignal): Promise<RuntimeSnapshotDto>;
  validateTransaction(request: ValidateTransactionDto): Promise<ValidationDto>;
  commitTransaction(request: CommitTransactionDto): Promise<CommitReceiptDto>;
  openSurface(surfaceId: string): Promise<NavigationDto>;
  resolvePath(query: string): Promise<PathResolutionDto>;
  submitBuild(request: BuildRequestDto): Promise<BuildTaskDto>;
  cancelBuild(taskId: string): Promise<void>;
  resolveConfirmation(request: ConfirmationDecisionDto): Promise<void>;
  registerLocalApp(): Promise<LocalAppDto>;
  launchLocalApp(request: LaunchLocalAppDto): Promise<ExternalSessionDto>;
  subscribe(listener: (event: HostEventDto) => void): Unsubscribe;
}
```

具体 Tauri command 名称只存在于 adapter 内部。

## Envelope

```ts
interface IpcResponse<T> {
  requestId: string;
  revision?: number;
  data?: T;
  error?: {
    reason: string;
    message: string;
    retryable: boolean;
    auditId?: string;
  };
}
```

每个请求生成 UUID request ID，并传入 tracing span。响应通过生成的 Zod schema 校验；未知字段允许向前兼容，缺少必填字段视为协议错误。

## 超时与重试

- 查询默认 10 秒；模型或构建命令使用调用方提供的更长超时。
- 只读请求在 transport failure 时最多自动重试一次。
- commit 只有显式 idempotency key 才允许查询结果，不能盲目重发。
- 页面卸载时取消未完成查询，不假定 Rust 操作也被取消。

## 事件

单一 Host event channel 承载 domain、build、local app 和 notification 事件。IPC Client 校验 envelope 后交给 Projection Store。取消订阅必须幂等。

## 错误映射

Transport、Protocol、Domain、Policy 和 Internal 错误映射为可判别联合。UI 只能展示安全 message 和 audit ID，不能展示 Rust backtrace。

## 安全

- adapter 维护 command allowlist；业务代码不能传任意 command 名。
- 禁止 Tauri shell/fs 通用插件 API 进入该包。
- 日志对请求参数执行字段级脱敏。
- Secret 和 Agent credential 不进入 WebView。

## 测试

- 每个 command 的请求/响应 contract test。
- malformed response、超时、取消和重复事件。
- commit transport timeout 后不自动重放。
- 架构测试扫描业务模块不得直接 import Tauri invoke/listen。

## 验收标准

1. 所有 WebView 系统调用均可在此接口中枚举。
2. 业务组件不依赖 `@tauri-apps/api/core`。
3. 响应未经 schema 校验不能进入 Projection Store。
4. 日志中不出现 Secret、SQL 或完整可执行路径。
