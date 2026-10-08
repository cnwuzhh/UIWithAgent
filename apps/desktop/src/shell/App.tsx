import { useEffect, useState, type FormEvent } from "react";
import { Bot, ChevronRight, Command, LayoutDashboard, Maximize2, Minimize2, Plus, Search, Send, X } from "lucide-react";
import type { RuntimeSnapshot } from "../contracts";
import { addTimePanel, getRuntimeSnapshot, isFullscreen, openSurface, removeElement, repositionElement, resizeElement, submitBuild, toggleFullscreen } from "../ipc/client";
import { SurfaceView } from "../surface/SurfaceView";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; snapshot: RuntimeSnapshot }
  | { status: "failed"; message: string };

export function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [fullscreen, setFullscreen] = useState(true);
  const [builderOpen, setBuilderOpen] = useState(false);
  const [buildRequest, setBuildRequest] = useState("");
  const [buildMessage, setBuildMessage] = useState<string>();
  const [buildError, setBuildError] = useState<string>();
  const [submittingBuild, setSubmittingBuild] = useState(false);

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

  async function handleSubmitBuild(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (state.status !== "ready" || !buildRequest.trim() || submittingBuild) return;
    setSubmittingBuild(true);
    setBuildError(undefined);
    setBuildMessage(undefined);
    try {
      const submission = await submitBuild(state.snapshot.currentSurfaceId, buildRequest);
      setState({ status: "ready", snapshot: submission.snapshot });
      setBuildMessage(submission.message);
      setBuildRequest("");
    } catch (error: unknown) {
      setBuildError(error instanceof Error ? error.message : "无法提交构建需求");
    } finally {
      setSubmittingBuild(false);
    }
  }

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand"><Command size={18} /> UI With Agent</div>
        <div className="titlebar-actions">
          <div className="system-state"><span /> Runtime connected</div>
          <button
            className="fullscreen-toggle"
            title={fullscreen ? "退出全屏" : "进入全屏"}
            aria-label={fullscreen ? "退出全屏" : "进入全屏"}
            onClick={handleToggleFullscreen}
          >
            {fullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
          </button>
        </div>
      </header>

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
          onRepositionElement={handleRepositionElement}
          onResizeElement={handleResizeElement}
          onAddTimePanel={handleAddTimePanel}
          onRemoveElement={handleRemoveElement}
        />
      )}

      <aside className={`agent-dock${builderOpen ? " expanded" : ""}`} aria-label="Agent 构建入口">
        <div className="agent-mark"><Bot size={22} /></div>
        <div className="agent-copy">
          <strong>Agent Builder</strong>
          <span>{builderOpen ? "告诉 Agent 你希望当前 Surface 增加什么" : "描述你想创建的应用"}</span>
          {builderOpen && (
            <form className="agent-builder-form" onSubmit={handleSubmitBuild}>
              <label htmlFor="build-request">构建需求</label>
              <textarea
                id="build-request"
                value={buildRequest}
                onChange={(event) => setBuildRequest(event.target.value)}
                placeholder="例如：在当地时间上增加当地天气"
                rows={2}
                autoFocus
              />
              <div className="agent-builder-status" aria-live="polite">
                {buildMessage && <span className="agent-reply"><Bot size={14} />Agent：{buildMessage}</span>}
                {buildError && <span className="agent-build-error">{buildError}</span>}
                <button type="submit" disabled={!buildRequest.trim() || submittingBuild}>
                  <Send size={15} />{submittingBuild ? "提交中" : "提交需求"}
                </button>
              </div>
            </form>
          )}
        </div>
        <button
          className="builder-toggle"
          onClick={() => {
            setBuilderOpen(!builderOpen);
            setBuildError(undefined);
            setBuildMessage(undefined);
          }}
        >
          {builderOpen ? <X size={17} /> : <Plus size={17} />}{builderOpen ? "关闭" : "新建构建"}
        </button>
      </aside>
    </main>
  );
}
