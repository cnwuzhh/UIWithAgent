import { useEffect, useState } from "react";
import { Bot, ChevronRight, Command, LayoutDashboard, Plus, Search } from "lucide-react";
import type { RuntimeSnapshot } from "../contracts";
import { getRuntimeSnapshot } from "../ipc/client";
import { SurfaceView } from "../surface/SurfaceView";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; snapshot: RuntimeSnapshot }
  | { status: "failed"; message: string };

export function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });

  useEffect(() => {
    getRuntimeSnapshot()
      .then((snapshot) => setState({ status: "ready", snapshot }))
      .catch((error: unknown) => {
        const message = error instanceof Error ? error.message : "Runtime 暂时不可用";
        setState({ status: "failed", message });
      });
  }, []);

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand"><Command size={18} /> UI With Agent</div>
        <div className="system-state"><span /> Runtime connected</div>
      </header>

      <nav className="breadcrumb" aria-label="当前位置">
        <LayoutDashboard size={17} />
        {state.status === "ready" ? state.snapshot.breadcrumb.map((item) => (
          <span className="breadcrumb-item" key={item.surfaceId}>
            <ChevronRight size={15} />{item.title}
          </span>
        )) : <span className="breadcrumb-item"><ChevronRight size={15} />正在载入</span>}
        <button className="icon-button" title="搜索 Surface" aria-label="搜索 Surface"><Search size={18} /></button>
      </nav>

      {state.status === "loading" && <section className="status-page">正在连接 Runtime...</section>}
      {state.status === "failed" && <section className="status-page error">{state.message}</section>}
      {state.status === "ready" && <SurfaceView surface={state.snapshot.surface} />}

      <aside className="agent-dock" aria-label="Agent 构建入口">
        <div className="agent-mark"><Bot size={22} /></div>
        <div><strong>Agent Builder</strong><span>描述你想创建的应用</span></div>
        <button><Plus size={17} /> 新建构建</button>
      </aside>
    </main>
  );
}
