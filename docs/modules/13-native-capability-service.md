# 13 Native Capability Service 模块设计

- 所有权：Rust
- crate：`crates/agentos-native`

## 目标

封装 Ubuntu 桌面能力：本地应用注册/启动/跟踪、xdg-desktop-portal、Secret Service、通知和窗口恢复，为应用层提供窄 trait。

## 非目标

不暴露任意 Shell、通用文件系统、不做 Agent capability 决策、不保存领域状态、不承诺 Wayland 不保证的窗口控制。

## Ports

```rust
#[async_trait]
pub trait LocalAppService {
    async fn register_via_picker(&self, actor: UserId) -> Result<LocalAppCandidate, NativeError>;
    async fn launch(&self, definition: AuthorizedLocalApp, context: ReturnContext) -> Result<ExternalAppSession, NativeError>;
    async fn revoke(&self, local_app_id: LocalAppId) -> Result<(), NativeError>;
}

#[async_trait]
pub trait SecretService {
    async fn store(&self, label: SecretLabel, secret: SecretBytes) -> Result<SecretRef, NativeError>;
    async fn resolve(&self, reference: &SecretRef) -> Result<SecretBytes, NativeError>;
    async fn delete(&self, reference: &SecretRef) -> Result<(), NativeError>;
}
```

Notification、Portal 和 WindowActivation 采用独立小 trait，便于测试和降级。

## 本地应用注册

1. 必须由用户操作触发系统 picker。
2. 解析 `.desktop` 或 executable candidate。
3. 显示名称、路径摘要、参数和权限供用户确认。
4. 应用层生成 `LocalAppDefinition` 并经 Repository 保存。
5. Agent 永远不能注册、修改路径或参数。

真实 executable path 只在 Rust Native/Repository 边界内使用；Agent API 只返回 localAppId 和显示信息。

## 启动与跟踪

使用 `tokio::process::Command` 和参数数组，不经过 Shell。记录 PID、return Surface、launcher Element 和 startedAt。若 launcher 很快退出并把请求交给已有进程，返回 `Untrackable`，不得伪造 exited event。

## Wayland 行为

优先请求正常窗口激活；compositor 拒绝时发送桌面/应用内通知，不循环抢焦点。不依赖全局窗口枚举、窗口嵌入或 X11-only API。

## Secret 生命周期

Secret 使用 zeroizing buffer，禁止 Clone/Debug/Serialize。SQLite 仅保存 SecretRef。keyring 不可用时禁用需要 Secret 的能力，不能退化为明文文件。

## 错误与降级

`PortalUnavailable`、`PermissionDenied`、`ExecutableMissing`、`LaunchFailed`、`Untrackable`、`SecretServiceUnavailable`、`WindowActivationDenied`。错误含安全摘要和 audit ID，不向 Agent 暴露完整路径。

## 测试

- Fake ports 测注册确认、撤销和启动流程。
- 参数不经过 Shell 的 contract test。
- child exit、launcher transfer 和取消 race。
- Secret 不可 Debug/Serialize 的编译/单元测试。
- Ubuntu Wayland/X11 手工与 E2E 矩阵。

## 验收标准

1. 无任何 API 接受 Shell 字符串。
2. Agent 不能注册或改变 LocalAppDefinition。
3. Secret Service 不可用时不保存明文替代品。
4. 无法跟踪进程时 UI 收到明确降级状态。
