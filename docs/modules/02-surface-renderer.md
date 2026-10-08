# 02 Surface Renderer 模块设计

- 所有权：React
- 位置：`apps/desktop/src/surface`

## 目标

将 `SurfaceSnapshot` 确定性映射为 12 列 CSS Grid，并通过 Element Registry 渲染元素、选择框和编辑预览。

## 非目标

- 不校验权威几何规则；
- 不直接保存拖动结果；
- 不解析数据库 payload；
- 不根据 Element 类型调用任意动态代码。

## 依赖

`Surface Renderer -> UI Projection Store / IPC Client / Element Registry`。不得依赖 Tauri API、Repository 或 Agent API。

## 内部结构

```text
SurfaceView
├── GridCanvas
├── ElementLayer
│   ├── TimePanelRenderer
│   ├── AppIconRenderer
│   └── UnsupportedElement
├── SelectionLayer
└── InteractionLayer
```

Element Registry 是显式静态映射：

```ts
export type ElementRendererMap = {
  [K in GUIElement["type"]]: React.ComponentType<{
    element: Extract<GUIElement, { type: K }>;
    mode: "view" | "edit";
  }>;
};
```

未知类型始终进入安全 fallback，不动态 import 未签名代码。

## 布局规则

- `Rect.x/y` 为零基逻辑坐标；CSS Grid 转换时加一。
- Surface width 默认 12 列，row height 使用稳定 token。
- Element 外框尺寸不因 hover、selection 或状态文字变化。
- Renderer 使用 `Surface.icon` 解析内部 Surface 入口图标。
- view mode 不安装拖动/缩放手势；edit mode 才启用 InteractionLayer。

## 编辑预览

Pointer move 只更新本地 preview rect。结束手势时调用 `runtimeValidateTransaction`；验证成功后再 commit。Rust 返回失败时恢复 snapshot rect，并展示原因。

```ts
interface InteractionPreview {
  elementId: string;
  kind: "move" | "resize";
  origin: Rect;
  candidate: Rect;
}
```

## 性能

- TimePanel tick 局部订阅，不刷新整个 Surface。
- Projection selector 以 Surface 和 Element ID 订阅。
- 大 Surface 先限制 MVP 元素数量；达到阈值后再评估 viewport virtualization。
- resize observer 只计算显示比例，不回写领域尺寸。

## 错误与恢复

- 缺少 target：显示不可打开状态，不抛出整页错误。
- 未知 Element：显示类型、ID 和“暂不支持”。
- 图标资源失败：使用确定性 fallback。
- snapshot revision 切换时取消未提交 pointer gesture。

## 安全与可访问性

- Element 文本按纯文本渲染。
- App Icon 支持单击选择、双击或 Enter 打开。
- 拖动必须有键盘替代：方向键移动，修饰键调整步长。
- 状态不只依赖颜色。

## 测试

- Rect 到 CSS Grid property 的参数化测试。
- 每种 Element 和未知类型组件测试。
- pointer/keyboard 编辑预览与失败回滚。
- snapshot revision 更新时取消手势。
- 不同视口下无溢出和遮挡的截图测试。

## 验收标准

1. 相同 snapshot 产生相同 DOM 布局。
2. Renderer 不直接修改 Projection 或持久状态。
3. 未知元素不导致 Surface 崩溃。
4. view mode 中不存在可触发写入的布局手势。
