import { useEffect, useState } from "react";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, BriefcaseBusiness, Check, Clock3, Move, Search } from "lucide-react";
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
    <article className="time-panel">
      <div className="panel-heading"><Clock3 size={16} /> {element.title}</div>
      <time>{displayTime}</time>
      <small>{element.timezone === "local" ? "系统时区" : element.timezone}</small>
    </article>
  );
}

function AppIcon({ element, editing, onOpenSurface }: {
  element: Extract<SurfaceElement, { type: "appIcon" }>;
  editing: boolean;
  onOpenSurface: (surfaceId: string) => void;
}) {
  return (
    <button
      className="app-icon"
      onDoubleClick={() => !editing && onOpenSurface(element.targetSurfaceId)}
      onKeyDown={(event) => !editing && event.key === "Enter" && onOpenSurface(element.targetSurfaceId)}
    >
      <span><BriefcaseBusiness size={25} /></span>
      <strong>{element.title}</strong>
    </button>
  );
}

function TextPanel({ element, editing, onOpenSurface }: {
  element: Extract<SurfaceElement, { type: "textPanel" }>;
  editing: boolean;
  onOpenSurface: (surfaceId: string) => void;
}) {
  return (
    <article className="text-panel">
      <h2>{element.title}</h2>
      <p>{element.runs.map((run, index) => run.type === "text"
        ? <span key={index}>{run.content}</span>
        : <button type="button" disabled={editing} key={index} onClick={() => onOpenSurface(run.targetSurfaceId)}>{run.label}</button>)}</p>
    </article>
  );
}

function SurfaceElementView({ element, editing, onOpenSurface, onReposition }: {
  element: SurfaceElement;
  editing: boolean;
  onOpenSurface: (surfaceId: string) => void;
  onReposition: (element: SurfaceElement, deltaX: number, deltaY: number) => void;
}) {
  const content = (() => {
    switch (element.type) {
      case "timePanel": return <TimePanel element={element} />;
      case "appIcon": return <AppIcon element={element} editing={editing} onOpenSurface={onOpenSurface} />;
      case "textPanel": return <TextPanel element={element} editing={editing} onOpenSurface={onOpenSurface} />;
    }
  })();

  return (
    <div
      className={`element-frame${editing ? " editing" : ""}`}
      style={{
        gridColumn: `${element.rect.x + 1} / span ${element.rect.width}`,
        gridRow: `${element.rect.y + 1} / span ${element.rect.height}`,
      }}
    >
      {content}
      {editing && (
        <div className="move-controls" aria-label={`移动${element.title}`}>
          <button title="向左移动" aria-label="向左移动" onClick={() => onReposition(element, -1, 0)}><ArrowLeft size={14} /></button>
          <button title="向上移动" aria-label="向上移动" onClick={() => onReposition(element, 0, -1)}><ArrowUp size={14} /></button>
          <button title="向下移动" aria-label="向下移动" onClick={() => onReposition(element, 0, 1)}><ArrowDown size={14} /></button>
          <button title="向右移动" aria-label="向右移动" onClick={() => onReposition(element, 1, 0)}><ArrowRight size={14} /></button>
        </div>
      )}
    </div>
  );
}

export function SurfaceView({ surface, onOpenSurface, onRepositionElement }: {
  surface: RuntimeSnapshot["surface"];
  onOpenSurface: (surfaceId: string) => void;
  onRepositionElement: (elementId: string, x: number, y: number) => Promise<string | undefined>;
}) {
  const [editing, setEditing] = useState(false);
  const [layoutError, setLayoutError] = useState<string>();

  async function moveElement(element: SurfaceElement, deltaX: number, deltaY: number) {
    const x = element.rect.x + deltaX;
    const y = element.rect.y + deltaY;
    if (x < 0 || y < 0) {
      setLayoutError("元素不能移出 Surface 边界");
      return;
    }
    setLayoutError(await onRepositionElement(element.id, x, y));
  }

  return (
    <section className="surface">
      <div className="surface-heading">
        <div><span className="eyebrow">CURRENT SURFACE</span><h1>{surface.title}</h1></div>
        <div className="surface-actions">
          <span className="surface-id">{surface.id} · {surface.columns} × {surface.rows}</span>
          <button className="layout-toggle" onClick={() => { setEditing(!editing); setLayoutError(undefined); }}>
            {editing ? <Check size={16} /> : <Move size={16} />}{editing ? "完成" : "编辑布局"}
          </button>
        </div>
      </div>
      {layoutError && <div className="layout-error" role="status">{layoutError}</div>}
      <div
        className="surface-grid"
        style={{
          gridTemplateColumns: `repeat(${surface.columns}, 1fr)`,
          gridTemplateRows: `repeat(${surface.rows}, 112px)`,
        }}
      >
        {surface.elements.map((element) => (
          <SurfaceElementView
            element={element}
            editing={editing}
            key={element.id}
            onOpenSurface={onOpenSurface}
            onReposition={moveElement}
          />
        ))}
      </div>
      <div className="navigator"><Search size={17} /><span>跳转到 Surface 或告诉 Agent 你要去哪里...</span><kbd>Enter</kbd></div>
    </section>
  );
}
