import { useEffect, useState } from "react";
import { Bot, ChevronRight, Command, LayoutDashboard, Maximize2, Minimize2, Plus, Search } from "lucide-react";
import type { RuntimeSnapshot } from "../contracts";
import { getRuntimeSnapshot, isFullscreen, openSurface, repositionElement, toggleFullscreen } from "../ipc/client";
import { SurfaceView } from "../surface/SurfaceView";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; snapshot: RuntimeSnapshot }
  | { status: "failed"; message: string };

export function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [fullscreen, setFullscreen] = useState(true);

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

  async function handleToggleFullscreen() {
    try {
      setFullscreen(await toggleFullscreen());
    } catch (error) {
      console.error("Unable to toggle fullscreen", error);
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
          surface={state.snapshot.surface}
          onOpenSurface={handleOpenSurface}
          onRepositionElement={handleRepositionElement}
        />
      )}

      <aside className="agent-dock" aria-label="Agent 构建入口">
        <div className="agent-mark"><Bot size={22} /></div>
        <div><strong>Agent Builder</strong><span>描述你想创建的应用</span></div>
        <button><Plus size={17} /> 新建构建</button>
      </aside>
    </main>
  );
}
