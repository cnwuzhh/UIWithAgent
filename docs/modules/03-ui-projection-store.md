# 03 UI Projection Store 模块设计

- 所有权：Zustand
- 位置：`apps/desktop/src/state`

## 目标

维护 Rust 权威状态在 WebView 中的只读投影，以及选择、预览、Dialog 等瞬时 UI 状态。提供细粒度 selector，隔离事件重放和重同步。

## 非目标

- 不执行领域操作或碰撞判断；
- 不持久化 `GUIDocument`；
- 不生成 revision；
- 不把 optimistic preview 当作提交成功。

## 状态模型

```ts
interface UiProjectionState {
  connection: "booting" | "ready" | "stale" | "failed";
  revision?: number;
  document?: GuiDocumentDto;
  navigation?: NavigationProjection;
  buildTasks: Record<string, BuildTaskDto>;
  pending: Record<string, PendingRequest>;
  selection?: { surfaceId: string; elementId: string };
  interactionPreview?: InteractionPreview;
  dialogs: DialogState[];
}
```

权威字段只能通过 `applySnapshot`、`applyDomainEvent` 和 `markStale` 修改。组件不能获得通用 `setState` 引用。

## 事件协议

```ts
interface ProjectionEventEnvelope<T> {
  eventId: string;
  revision: number;
  type: string;
  payload: T;
}
```

应用规则：

1. `revision == current + 1`：应用事件。
2. `revision <= current`：按 event ID 去重后忽略。
3. `revision > current + 1`：标记 stale，停止写入并请求完整 snapshot。
4. snapshot 应用时清理与新 revision 不兼容的 preview 和 selection。

## Actions

```text
applySnapshot(snapshot)
applyDomainEvent(event)
registerPending(request)
resolvePending(requestId)
markStale(reason)
setSelection(selection)
setInteractionPreview(preview)
pushDialog(dialog)
popDialog(dialogId)
resetEphemeralState()
```

## Selector 设计

- `selectCurrentSurface`
- `selectBreadcrumb`
- `selectElement(surfaceId, elementId)`
- `selectBuildTask(taskId)`
- `selectCanWrite`，仅在 connection ready 且无 revision gap 时为 true。

禁止组件订阅整个 store；Clock 等高频状态不进入全局 Projection。

## 错误与恢复

- 事件 payload 解析失败：保留现有 projection，进入 stale。
- IPC 重连：请求完整 snapshot，而不是猜测丢失事件。
- pending 超时：清理请求状态，但不回滚权威 snapshot。
- Surface 删除后 selection 自动清空。

## 安全

Projection DTO 在 IPC Client 层完成 schema 校验。Store 不接受数据库 entity、Secret、可执行路径或 Agent session credential。

## 测试

- 连续、重复、乱序和跳号事件测试。
- snapshot 替换与瞬时状态清理。
- selector 引用稳定性和组件重渲染计数。
- stale 状态下 `selectCanWrite == false`。

## 验收标准

1. WebView reload 后可由单个 snapshot 完整重建。
2. revision gap 不会被静默忽略。
3. Store 中不存在持久化 adapter 或数据库 API。
4. TimePanel tick 不导致 Surface 列表重渲染。
