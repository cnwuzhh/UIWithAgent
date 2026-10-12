import { useEffect, useState, type FormEvent } from "react";
import { Bot, Boxes, ChevronRight, Code2, Command, FileCode2, Hammer, LayoutDashboard, LoaderCircle, Maximize2, Minimize2, Plus, Search, Send, ShieldCheck, X } from "lucide-react";
import type { AgentBuildResult, BuildTask, GeneratedAppPackage, ReviewedAgentPlan, RuntimeSnapshot } from "../contracts";
import { addTimePanel, buildAgentPackage, generateAgentPackage, getBuildTask, getRuntimeSnapshot, isFullscreen, openSurface, planAgent, removeElement, repositionElement, resizeElement, toggleFullscreen } from "../ipc/client";
import { SurfaceView } from "../surface/SurfaceView";
import { ToolboxDebugDialog } from "../toolbox/ToolboxDebugDialog";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; snapshot: RuntimeSnapshot }
  | { status: "failed"; message: string };

const DEFAULT_BUILD_REQUEST = "生成一个财经新闻列表应用。使用已由 Host 验证可达的 MarketWatch RSS 源 https://feeds.marketwatch.com/marketwatch/topstories/（允许跟随重定向，最终返回 application/xml），消费数据源提供的 title、description/summary、source、publishedAt 和 url 字段，按 publishedAt 倒序展示最新 10 条新闻。不要实现摘要、分类、评分或其他复杂数据加工逻辑。在线探测失败时必须在 stderr 保留 URL、异常类型和底层异常原因，不得只输出 fetch error。";

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return fallback;
}

export function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [fullscreen, setFullscreen] = useState(true);
  const [builderOpen, setBuilderOpen] = useState(false);
  const [buildRequest, setBuildRequest] = useState(DEFAULT_BUILD_REQUEST);
  const [reviewedPlan, setReviewedPlan] = useState<ReviewedAgentPlan>();
  const agentPlan = reviewedPlan?.plan;
  const [generatedPackage, setGeneratedPackage] = useState<GeneratedAppPackage>();
  const [generatingPackage, setGeneratingPackage] = useState(false);
  const [packageError, setPackageError] = useState<string>();
  const [buildingPackage, setBuildingPackage] = useState(false);
  const [agentBuildResult, setAgentBuildResult] = useState<AgentBuildResult>();
  const [agentBuildError, setAgentBuildError] = useState<string>();
  const [buildError, setBuildError] = useState<string>();
  const [submittingBuild, setSubmittingBuild] = useState(false);
  const [planningStartedAt, setPlanningStartedAt] = useState<number>();
  const [planningElapsedSeconds, setPlanningElapsedSeconds] = useState(0);
  const [buildTask, setBuildTask] = useState<BuildTask>();
  const [taskError, setTaskError] = useState<string>();
  const [toolboxOpen, setToolboxOpen] = useState(false);

  useEffect(() => {
    getRuntimeSnapshot()
      .then((snapshot) => setState({ status: "ready", snapshot }))
      .catch((error: unknown) => {
        const message = error instanceof Error ? error.message : "Runtime 暂时不可用";
        setState({ status: "failed", message });
      });
    isFullscreen().then(setFullscreen).catch((error: unknown) => {
      console.error("Unable to read fullscreen state", error);
    });
  }, []);

  useEffect(() => {
    if (!planningStartedAt) return;
    const updateElapsed = () => {
      setPlanningElapsedSeconds(Math.floor((Date.now() - planningStartedAt) / 1000));
    };
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 1000);
    return () => window.clearInterval(timer);
  }, [planningStartedAt]);

  async function handleOpenSurface(surfaceId: string) {
    if (state.status !== "ready") return;
    try {
      setState({ status: "ready", snapshot: await openSurface(surfaceId) });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "无法打开 Surface";
      setState({ status: "failed", message });
    }
  }

  async function handleRepositionElement(elementId: string, x: number, y: number) {
    if (state.status !== "ready") return "Runtime 暂时不可写";
    try {
      const snapshot = await repositionElement(state.snapshot.currentSurfaceId, elementId, x, y);
      setState({ status: "ready", snapshot });
      return undefined;
    } catch (error: unknown) {
      return error instanceof Error ? error.message : "无法移动元素";
    }
  }

  async function handleResizeElement(elementId: string, width: number, height: number) {
    if (state.status !== "ready") return "Runtime 暂时不可写";
    try {
      const snapshot = await resizeElement(state.snapshot.currentSurfaceId, elementId, width, height);
      setState({ status: "ready", snapshot });
      return undefined;
    } catch (error: unknown) {
      return error instanceof Error ? error.message : "无法调整元素尺寸";
    }
  }

  async function handleAddTimePanel() {
    if (state.status !== "ready") return "Runtime 暂时不可写";
    try {
      const snapshot = await addTimePanel(state.snapshot.currentSurfaceId, "新时钟", "local");
      setState({ status: "ready", snapshot });
      return undefined;
    } catch (error: unknown) {
      return error instanceof Error ? error.message : "无法添加时钟";
    }
  }

  async function handleRemoveElement(elementId: string) {
    if (state.status !== "ready") return "Runtime 暂时不可写";
    try {
      const snapshot = await removeElement(state.snapshot.currentSurfaceId, elementId);
      setState({ status: "ready", snapshot });
      return undefined;
    } catch (error: unknown) {
      return error instanceof Error ? error.message : "无法删除元素";
    }
  }

  async function handleToggleFullscreen() {
    try {
      setFullscreen(await toggleFullscreen());
    } catch (error) {
      console.error("Unable to toggle fullscreen", error);
    }
  }

  async function handlePlanRequest(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (state.status !== "ready" || !buildRequest.trim() || submittingBuild) return;
    setSubmittingBuild(true);
    setPlanningStartedAt(Date.now());
    setPlanningElapsedSeconds(0);
    setBuildError(undefined);
    setReviewedPlan(undefined);
    setGeneratedPackage(undefined);
    setPackageError(undefined);
    setAgentBuildResult(undefined);
    setAgentBuildError(undefined);
    try {
      setReviewedPlan(await planAgent(state.snapshot.currentSurfaceId, buildRequest));
    } catch (error: unknown) {
      setBuildError(errorMessage(error, "无法生成规划方案"));
    } finally {
      setSubmittingBuild(false);
      setPlanningStartedAt(undefined);
    }
  }

  async function handleGeneratePackage() {
    if (!reviewedPlan || generatingPackage) return;
    setGeneratingPackage(true);
    setPackageError(undefined);
    setGeneratedPackage(undefined);
    setAgentBuildResult(undefined);
    setAgentBuildError(undefined);
    try {
      setGeneratedPackage(await generateAgentPackage(reviewedPlan.planId));
    } catch (error: unknown) {
      setPackageError(errorMessage(error, "无法生成代码包"));
    } finally {
      setGeneratingPackage(false);
    }
  }

  async function handleBuildPackage() {
    if (!reviewedPlan || !generatedPackage || buildingPackage) return;
    setBuildingPackage(true);
    setAgentBuildError(undefined);
    setAgentBuildResult(undefined);
    try {
      setAgentBuildResult(await buildAgentPackage(reviewedPlan.planId));
    } catch (error: unknown) {
      setAgentBuildError(errorMessage(error, "代码包构建失败"));
    } finally {
      setBuildingPackage(false);
    }
  }

  async function handleOpenBuildTask(buildTaskId: string) {
    setTaskError(undefined);
    try {
      setBuildTask(await getBuildTask(buildTaskId));
    } catch (error: unknown) {
      setTaskError(error instanceof Error ? error.message : "无法读取构建任务");
    }
  }

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand"><Command size={18} /> UI With Agent</div>
        <div className="titlebar-actions">
          <div className="system-state"><span /> Runtime connected</div>
          <button
            className="titlebar-icon-button"
            title="查看 Toolbox"
            aria-label="查看 Toolbox 调试信息"
            onClick={() => setToolboxOpen(true)}
          >
            <Boxes size={16} />
          </button>
          <button
            className="titlebar-icon-button"
            title={fullscreen ? "退出全屏" : "进入全屏"}
            aria-label={fullscreen ? "退出全屏" : "进入全屏"}
            onClick={handleToggleFullscreen}
          >
            {fullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
          </button>
        </div>
      </header>

      {toolboxOpen && <ToolboxDebugDialog onClose={() => setToolboxOpen(false)} />}

      <nav className="breadcrumb" aria-label="当前位置">
        <LayoutDashboard size={17} />
        {state.status === "ready" ? state.snapshot.breadcrumb.map((item) => (
          <button
            className="breadcrumb-item"
            key={item.surfaceId}
            onClick={() => handleOpenSurface(item.surfaceId)}
          >
            <ChevronRight size={15} />{item.title}
          </button>
        )) : <span className="breadcrumb-item"><ChevronRight size={15} />正在载入</span>}
        <button className="icon-button" title="搜索 Surface" aria-label="搜索 Surface"><Search size={18} /></button>
      </nav>

      {state.status === "loading" && <section className="status-page">正在连接 Runtime...</section>}
      {state.status === "failed" && <section className="status-page error">{state.message}</section>}
      {state.status === "ready" && (
        <SurfaceView
          revision={state.snapshot.revision}
          surface={state.snapshot.surface}
          onOpenSurface={handleOpenSurface}
          onOpenBuildTask={handleOpenBuildTask}
          onRepositionElement={handleRepositionElement}
          onResizeElement={handleResizeElement}
          onAddTimePanel={handleAddTimePanel}
          onRemoveElement={handleRemoveElement}
        />
      )}

      {(buildTask || taskError) && (
        <div className="task-dialog-backdrop" role="presentation" onMouseDown={() => { setBuildTask(undefined); setTaskError(undefined); }}>
          <section
            className="task-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="task-dialog-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <header>
              <div><span className="eyebrow">BUILD TASK</span><h2 id="task-dialog-title">构建任务详情</h2></div>
              <button className="task-dialog-close" aria-label="关闭任务详情" onClick={() => { setBuildTask(undefined); setTaskError(undefined); }}><X size={18} /></button>
            </header>
            {taskError ? <p className="task-dialog-error">{taskError}</p> : buildTask && (
              <div className="task-dialog-content">
                <div className="task-status"><Hammer size={16} /><strong>构建中</strong><span>{buildTask.id}</span></div>
                <div><span>原始需求</span><p>{buildTask.request}</p></div>
                <div><span>Agent 回复</span><p>{buildTask.agentMessage}</p></div>
                <dl><dt>Surface</dt><dd>{buildTask.surfaceId}</dd><dt>占位元素</dt><dd>{buildTask.placeholderElementId}</dd></dl>
              </div>
            )}
          </section>
        </div>
      )}

      <aside className={`agent-dock${builderOpen ? " expanded" : ""}`} aria-label="Agent 规划入口">
        <div className="agent-mark"><Bot size={22} /></div>
        <div className="agent-copy">
          <strong>Agent Planner</strong>
          <span>{builderOpen ? "Agent 只生成方案，不会修改当前 Surface" : "先审查方案，再决定是否构建"}</span>
          {builderOpen && (
            <form className="agent-builder-form" onSubmit={handlePlanRequest}>
              <label htmlFor="build-request">规划需求</label>
              <textarea
                id="build-request"
                value={buildRequest}
                onChange={(event) => setBuildRequest(event.target.value)}
                placeholder="例如：在当地时间上增加当地天气"
                rows={2}
                autoFocus
              />
              <div className="agent-builder-status" aria-live="polite">
                {buildError && <span className="agent-build-error">{buildError}</span>}
                <button type="submit" disabled={!buildRequest.trim() || submittingBuild}>
                  {submittingBuild ? <LoaderCircle className="spin" size={15} /> : <Send size={15} />}
                  {submittingBuild ? "规划中" : "生成方案"}
                </button>
              </div>
              {submittingBuild && (
                <div className="agent-request-progress" role="status" aria-live="polite">
                  <LoaderCircle className="spin" size={17} />
                  <div>
                    <strong>模型正在生成并审核方案</strong>
                    <span>已等待 {planningElapsedSeconds} 秒。复杂方案可能需要数分钟，请保持窗口开启。</span>
                  </div>
                </div>
              )}
              {agentPlan && (
                <section className="agent-plan" aria-live="polite">
                  <header><div><span className="eyebrow">PLAN ONLY</span><h2>{agentPlan.goal}</h2></div><span><ShieldCheck size={14} />只读</span></header>
                  <div className="agent-plan-notes"><strong>实现方法</strong><p>{agentPlan.approach}</p></div>
                  <div className="agent-plan-notes"><strong>架构与职责</strong><ul>{agentPlan.architecture.map((item) => <li key={item}>{item}</li>)}</ul></div>
                  <div className="agent-plan-notes"><strong>数据协议</strong><p>{agentPlan.dataContract}</p></div>
                  <div className="agent-plan-notes agent-plan-commands">
                    <strong>构建与运行</strong>
                    {agentPlan.buildCommands.map((command) => <code key={command}>{command}</code>)}
                    <code>{agentPlan.runCommand}</code>
                  </div>
                  <ol className="agent-plan-steps">
                    {agentPlan.steps.map((step, index) => (
                      <li key={`${index}-${step.title}`}>
                        <h3>{step.title}</h3>
                        <p>{step.action}</p>
                        <dl>
                          <dt>产物</dt>
                          <dd><ul>{step.deliverables.map((item) => <li key={item}>{item}</li>)}</ul></dd>
                          <dt>验证</dt>
                          <dd>{step.verification}</dd>
                        </dl>
                      </li>
                    ))}
                  </ol>
                  <div className="agent-plan-notes">
                    <strong>UI 绑定 · {agentPlan.uiBinding.status === "ready" ? "可提交" : "阻塞"}</strong>
                    <p><code>{agentPlan.uiBinding.typeId}</code> · {agentPlan.uiBinding.notes}</p>
                    <code>{JSON.stringify(agentPlan.uiBinding.config)}</code>
                    {agentPlan.uiBinding.adapterId && <p>Adapter：<code>{agentPlan.uiBinding.adapterId}</code></p>}
                  </div>
                  <div className="agent-plan-notes"><strong>预期影响</strong><p>{agentPlan.impact}</p></div>
                  {agentPlan.assumptions.length > 0 && <div className="agent-plan-notes"><strong>前提与待确认</strong><ul>{agentPlan.assumptions.map((item) => <li key={item}>{item}</li>)}</ul></div>}
                  <div className="agent-package-action">
                    <button type="button" onClick={handleGeneratePackage} disabled={generatingPackage}>
                      <Code2 size={15} />{generatingPackage ? "生成并审核中" : "生成代码包"}
                    </button>
                    <span>只生成并静态审核，不写入 workspace、不构建、不运行</span>
                  </div>
                  {packageError && <span className="agent-build-error">{packageError}</span>}
                  {generatedPackage && (
                    <section className="agent-package" aria-live="polite">
                      <header>
                        <div><span className="eyebrow">REVIEW READY</span><h3>{generatedPackage.manifest.displayName}</h3></div>
                        <span><ShieldCheck size={14} />静态审核通过</span>
                      </header>
                      <dl>
                        <dt>App ID</dt><dd><code>{generatedPackage.manifest.appId}</code></dd>
                        <dt>输出</dt><dd>{generatedPackage.manifest.outputs.map((output) => `${output.name}@${output.schemaVersion}`).join(", ")}</dd>
                        <dt>UI 绑定</dt><dd>{generatedPackage.manifest.uiBindings.map((binding) => `${binding.output} → ${binding.typeId}`).join(", ") || "无"}</dd>
                      </dl>
                      <div className="agent-package-files">
                        {generatedPackage.files.map((file) => <span key={file.path}><FileCode2 size={14} />{file.path}</span>)}
                      </div>
                      <div className="agent-plan-notes agent-plan-commands">
                        <strong>将执行的精确构建命令</strong>
                        {generatedPackage.manifest.buildCommands.map((command, index) => (
                          <code key={`${index}-${command.program}`}>{[command.program, ...command.args].join(" ")}</code>
                        ))}
                      </div>
                      <div className="agent-build-action">
                        <button type="button" onClick={handleBuildPackage} disabled={buildingPackage || Boolean(agentBuildResult)}>
                          {buildingPackage ? <LoaderCircle className="spin" size={15} /> : <Hammer size={15} />}
                          {buildingPackage ? "构建中" : agentBuildResult ? "构建完成" : "确认并开始构建"}
                        </button>
                        <span>将物化 revision 1 并执行以上命令；不会运行应用或修改 Surface</span>
                      </div>
                      {agentBuildError && <p className="agent-build-result error">{agentBuildError}</p>}
                      {agentBuildResult && (
                        <div className="agent-build-result">
                          <strong>BUILD PASSED · revision {agentBuildResult.revision}</strong>
                          <span>{agentBuildResult.commands.length} 条命令全部成功</span>
                          <code>{agentBuildResult.workspace}</code>
                        </div>
                      )}
                    </section>
                  )}
                  <footer>基于 revision {agentPlan.basedOnRevision} · 权限 {agentPlan.capabilities.join(", ")} · 未执行任何变更</footer>
                </section>
              )}
            </form>
          )}
        </div>
        <button
          className="builder-toggle"
          onClick={() => {
            setBuilderOpen(!builderOpen);
            setBuildError(undefined);
            setReviewedPlan(undefined);
            setGeneratedPackage(undefined);
            setPackageError(undefined);
            setAgentBuildResult(undefined);
            setAgentBuildError(undefined);
          }}
        >
          {builderOpen ? <X size={17} /> : <Plus size={17} />}{builderOpen ? "关闭" : "新建规划"}
        </button>
      </aside>
    </main>
  );
}
