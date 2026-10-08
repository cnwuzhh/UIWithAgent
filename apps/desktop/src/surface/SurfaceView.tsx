import { useEffect, useState } from "react";
import { BriefcaseBusiness, Clock3, Search } from "lucide-react";
import type { RuntimeSnapshot, SurfaceElement } from "../contracts";

function TimePanel({ element }: { element: Extract<SurfaceElement, { type: "timePanel" }> }) {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  const displayTime = new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    ...(element.timezone === "local" ? {} : { timeZone: element.timezone }),
  }).format(now);

  return (
    <article className="time-panel" style={{ gridColumn: `${element.column} / span 4` }}>
      <div className="panel-heading"><Clock3 size={16} /> {element.title}</div>
      <time>{displayTime}</time>
      <small>{element.timezone === "local" ? "系统时区" : element.timezone}</small>
    </article>
  );
}

function AppIcon({ element, onOpenSurface }: {
  element: Extract<SurfaceElement, { type: "appIcon" }>;
  onOpenSurface: (surfaceId: string) => void;
}) {
  return (
    <button
      className="app-icon"
      onDoubleClick={() => onOpenSurface(element.targetSurfaceId)}
      onKeyDown={(event) => event.key === "Enter" && onOpenSurface(element.targetSurfaceId)}
      style={{ gridColumn: `${element.column} / span 2` }}
    >
      <span><BriefcaseBusiness size={25} /></span>
      <strong>{element.title}</strong>
    </button>
  );
}

export function SurfaceView({ surface, onOpenSurface }: {
  surface: RuntimeSnapshot["surface"];
  onOpenSurface: (surfaceId: string) => void;
}) {
  return (
    <section className="surface">
      <div className="surface-heading">
        <div><span className="eyebrow">CURRENT SURFACE</span><h1>{surface.title}</h1></div>
        <span className="surface-id">{surface.id}</span>
      </div>
      <div className="surface-grid">
        {surface.elements.map((element) => element.type === "timePanel"
          ? <TimePanel element={element} key={element.id} />
          : <AppIcon element={element} key={element.id} onOpenSurface={onOpenSurface} />)}
      </div>
      <div className="navigator"><Search size={17} /><span>跳转到 Surface 或告诉 Agent 你要去哪里...</span><kbd>Enter</kbd></div>
    </section>
  );
}
