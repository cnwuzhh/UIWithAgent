# UI Built by Agent

一个用于验证 Agent 构建和扩展二维 GUI 的最小原型。

MVP 以二维 `Surface` 为基础，只包含两种内容元素：

- `TimePanel`：显示当地时间或指定 IANA 时区的时间；
- `AppIcon`：链接到另一个 Surface，或表示一个构建任务。

系统 Shell 还提供 Agent 规划入口。用户提交需求后，真实 Agent 只返回结构化方案；规划阶段仅获得当前 Surface 的只读快照，不创建 BuildTask、不添加占位图标，也不修改 GUI。

整体进程、模块、数据、安全与交付架构见 [AgentOS 总体架构](docs/AgentOS-总体架构.md)。产品与交互细节见 [AgentOS MVP 设计文档](docs/AgentOS-MVP设计文档.md)。

17 个模块的职责、接口、状态、错误、安全与测试设计见 [模块设计索引](docs/modules/README.md)。

Agent 只能通过受 capability 限制的系统 API 使用 Surface、构建任务和本地能力，不能直接访问 SQLite、Repository、文件系统或任意 Shell。完整协议见 [Agent System API](docs/Agent-System-API.md)。

## 产品原型

- [完整 UI 原型总览](docs/prototypes/UI原型总览.md)
- [完整 UI 长图](docs/prototypes/all-ui-prototypes.png)
- [MVP 总览原型](docs/prototypes/agentos-ui-prototype.png)
- [App Move / Copy 交互原型](docs/prototypes/app-move-copy-prototype.png)
- [App Move / Copy 可编辑 SVG](docs/prototypes/app-move-copy-prototype.svg)
- [本地 App 启动与返回原型](docs/prototypes/local-app-launch-prototype.png)
- [本地 App 启动与返回可编辑 SVG](docs/prototypes/local-app-launch-prototype.svg)
- [Surface Icon 设置原型](docs/prototypes/surface-icon-prototype.png)
- [Surface Icon 设置可编辑 SVG](docs/prototypes/surface-icon-prototype.svg)
- [Wiki 路径与内嵌文本导航原型](docs/prototypes/wiki-navigation-prototype.png)
- [Wiki 路径与内嵌文本导航可编辑 SVG](docs/prototypes/wiki-navigation-prototype.svg)

## 实现阶段

1. 协议与静态 Surface 渲染；
2. 二维添加、移除、移动和缩放；
3. Agent 只读规划入口与结构化方案；
4. 本地持久化、校验与验收测试。

## 工程基线

工程已经建立 React 19、Tauri 2 与 Rust workspace，并打通 Rust authoritative Runtime IPC：

```text
apps/desktop                 React Shell 与 Tauri bootstrap
crates/agentos-contracts     跨 IPC DTO
crates/agentos-application   确定性 Runtime snapshot
```

Ubuntu 24.04 开发命令：

```bash
just bootstrap
just build
just test
just dev
```

启动前配置 OpenAI-compatible 模型服务：

```bash
export AGENTOS_MODEL_BASE_URL="https://api.openai.com/v1"
export AGENTOS_MODEL_NAME="your-model-name"
export AGENTOS_MODEL_API_KEY="your-api-key"
pnpm dev
```

也可以使用 `OPENAI_API_KEY` 代替 `AGENTOS_MODEL_API_KEY`。不要把 API Key 写入仓库；当前环境变量方式是 MVP 配置，后续将迁移到 GNOME Secret Service。

当前窗口展示桌面 breadcrumb、时区面板、Surface 入口与 Agent Planner。规划结果包含目标、步骤、预期影响和待确认事项，并明确显示 `surface.read` 权限与所依据的 revision。模型输出只能形成 `planOnly` DTO，不能提交 Operation；已有 BuildTask Stub 仍保留在后端用于后续迁移，但当前 UI 不再触发构建。Agent Worker、Secret Service、任务取消和 SQLite 将按后续小切片接入。
