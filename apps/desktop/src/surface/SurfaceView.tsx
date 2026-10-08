import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, BriefcaseBusiness, Check, Clock3, Grip, Hammer, Minus, Move, Plus, Search, Trash2 } from "lucide-react";
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
  const ready = element.status === "ready" && element.targetSurfaceId;
  return (
    <button
      type="button"
      className={`app-icon${ready ? "" : " building"}`}
      aria-disabled={!ready}
      onDoubleClick={() => !editing && ready && onOpenSurface(ready)}
      onKeyDown={(event) => !editing && ready && event.key === "Enter" && onOpenSurface(ready)}
    >
      <span>{ready ? <BriefcaseBusiness size={25} /> : <Hammer size={24} />}</span>
      <strong>{element.title}</strong>
      {!ready && <small>Agent 正在构建</small>}
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

function SurfaceElementView({ element, editing, selected, previewPosition, onOpenSurface, onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onReposition, onResize, onRemove }: {
  element: SurfaceElement;
  editing: boolean;
  selected: boolean;
  previewPosition?: { x: number; y: number };
  onOpenSurface: (surfaceId: string) => void;
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>, element: SurfaceElement) => void;
  onPointerMove: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerUp: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerCancel: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onReposition: (element: SurfaceElement, deltaX: number, deltaY: number) => void;
  onResize: (element: SurfaceElement, deltaWidth: number, deltaHeight: number) => void;
  onRemove: (elementId: string) => void;
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
      className={`element-frame${editing ? " editing" : ""}${selected ? " selected" : ""}${previewPosition ? " dragging" : ""}`}
      style={{
        gridColumn: `${(previewPosition?.x ?? element.rect.x) + 1} / span ${element.rect.width}`,
        gridRow: `${(previewPosition?.y ?? element.rect.y) + 1} / span ${element.rect.height}`,
      }}
      onPointerDown={(event) => editing && onPointerDown(event, element)}
      onPointerMove={(event) => editing && onPointerMove(event)}
      onPointerUp={(event) => editing && onPointerUp(event)}
      onPointerCancel={(event) => editing && onPointerCancel(event)}
    >
      {content}
      {selected && (
        <>
          <div className="drag-indicator" aria-hidden="true"><Grip size={14} /></div>
          <div className="move-controls element-controls" aria-label={`移动${element.title}`}>
            <button title="向左移动" aria-label="向左移动" onClick={() => onReposition(element, -1, 0)}><ArrowLeft size={14} /></button>
            <button title="向上移动" aria-label="向上移动" onClick={() => onReposition(element, 0, -1)}><ArrowUp size={14} /></button>
            <button title="向下移动" aria-label="向下移动" onClick={() => onReposition(element, 0, 1)}><ArrowDown size={14} /></button>
            <button title="向右移动" aria-label="向右移动" onClick={() => onReposition(element, 1, 0)}><ArrowRight size={14} /></button>
          </div>
          <div className="resize-controls element-controls" aria-label={`调整${element.title}尺寸`}>
            <span>宽</span>
            <button title="减少宽度" aria-label="减少宽度" onClick={() => onResize(element, -1, 0)}><Minus size={13} /></button>
            <button title="增加宽度" aria-label="增加宽度" onClick={() => onResize(element, 1, 0)}><Plus size={13} /></button>
            <span>高</span>
            <button title="减少高度" aria-label="减少高度" onClick={() => onResize(element, 0, -1)}><Minus size={13} /></button>
            <button title="增加高度" aria-label="增加高度" onClick={() => onResize(element, 0, 1)}><Plus size={13} /></button>
          </div>
          <button
            className="remove-element element-controls"
            title={`删除${element.title}`}
            aria-label={`删除${element.title}`}
            onClick={() => onRemove(element.id)}
          ><Trash2 size={14} /></button>
        </>
      )}
    </div>
  );
}

type DragState = {
  element: SurfaceElement;
  pointerId: number;
  startClientX: number;
  startClientY: number;
  x: number;
  y: number;
};

export function SurfaceView({ revision, surface, onOpenSurface, onRepositionElement, onResizeElement, onAddTimePanel, onRemoveElement }: {
  revision: number;
  surface: RuntimeSnapshot["surface"];
  onOpenSurface: (surfaceId: string) => void;
  onRepositionElement: (elementId: string, x: number, y: number) => Promise<string | undefined>;
  onResizeElement: (elementId: string, width: number, height: number) => Promise<string | undefined>;
  onAddTimePanel: () => Promise<string | undefined>;
  onRemoveElement: (elementId: string) => Promise<string | undefined>;
}) {
  const [editing, setEditing] = useState(false);
  const [layoutError, setLayoutError] = useState<string>();
  const [drag, setDrag] = useState<DragState>();
  const [selectedElementId, setSelectedElementId] = useState<string>();
  const dragRef = useRef<DragState | undefined>(undefined);
  const gridRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dragRef.current = undefined;
    setDrag(undefined);
  }, [editing, revision, surface.id]);
  useEffect(() => setSelectedElementId(undefined), [surface.id]);

  async function moveElement(element: SurfaceElement, deltaX: number, deltaY: number) {
    const x = element.rect.x + deltaX;
    const y = element.rect.y + deltaY;
    if (x < 0 || y < 0) {
      setLayoutError("元素不能移出 Surface 边界");
      return;
    }
    setLayoutError(await onRepositionElement(element.id, x, y));
  }

  async function resizeElement(element: SurfaceElement, deltaWidth: number, deltaHeight: number) {
    const width = element.rect.width + deltaWidth;
    const height = element.rect.height + deltaHeight;
    if (width < 1 || height < 1) {
      setLayoutError("元素宽度和高度至少为一个网格单位");
      return;
    }
    setLayoutError(await onResizeElement(element.id, width, height));
  }

  function beginDrag(event: ReactPointerEvent<HTMLDivElement>, element: SurfaceElement) {
    setSelectedElementId(element.id);
    if ((event.target as HTMLElement).closest(".element-controls")) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setLayoutError(undefined);
    const nextDrag = {
      element,
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      x: element.rect.x,
      y: element.rect.y,
    };
    dragRef.current = nextDrag;
    setDrag(nextDrag);
  }

  function previewDrag(event: ReactPointerEvent<HTMLDivElement>) {
    const currentDrag = dragRef.current;
    if (!currentDrag || event.pointerId !== currentDrag.pointerId || !gridRef.current) return;
    const style = window.getComputedStyle(gridRef.current);
    const columnGap = Number.parseFloat(style.columnGap) || 0;
    const rowGap = Number.parseFloat(style.rowGap) || 0;
    const contentWidth = gridRef.current.clientWidth
      - Number.parseFloat(style.paddingLeft)
      - Number.parseFloat(style.paddingRight);
    const columnWidth = (contentWidth - columnGap * (surface.columns - 1)) / surface.columns;
    const rowHeight = Number.parseFloat(style.gridTemplateRows.split(" ")[0]) || 112;
    const deltaX = Math.round((event.clientX - currentDrag.startClientX) / (columnWidth + columnGap));
    const deltaY = Math.round((event.clientY - currentDrag.startClientY) / (rowHeight + rowGap));
    const nextDrag = {
      ...currentDrag,
      x: Math.max(0, Math.min(surface.columns - currentDrag.element.rect.width, currentDrag.element.rect.x + deltaX)),
      y: Math.max(0, Math.min(surface.rows - currentDrag.element.rect.height, currentDrag.element.rect.y + deltaY)),
    };
    dragRef.current = nextDrag;
    setDrag(nextDrag);
  }

  async function finishDrag(event: ReactPointerEvent<HTMLDivElement>) {
    const completed = dragRef.current;
    if (!completed || event.pointerId !== completed.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    dragRef.current = undefined;
    setDrag(undefined);
    if (completed.x !== completed.element.rect.x || completed.y !== completed.element.rect.y) {
      setLayoutError(await onRepositionElement(completed.element.id, completed.x, completed.y));
    }
  }

  function cancelDrag(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.pointerId !== dragRef.current?.pointerId) return;
    dragRef.current = undefined;
    setDrag(undefined);
  }

  async function removeElement(elementId: string) {
    setLayoutError(await onRemoveElement(elementId));
    setSelectedElementId(undefined);
  }

  return (
    <section className="surface">
      <div className="surface-heading">
        <div><span className="eyebrow">CURRENT SURFACE</span><h1>{surface.title}</h1></div>
        <div className="surface-actions">
          <span className="surface-id">{surface.id} · {surface.columns} × {surface.rows}</span>
          {editing && <button className="add-element" onClick={async () => setLayoutError(await onAddTimePanel())}><Plus size={16} />添加时钟</button>}
          <button className="layout-toggle" onClick={() => { setEditing(!editing); setSelectedElementId(undefined); setLayoutError(undefined); }}>
            {editing ? <Check size={16} /> : <Move size={16} />}{editing ? "完成" : "编辑布局"}
          </button>
        </div>
      </div>
      {layoutError && <div className="layout-error" role="status">{layoutError}</div>}
      <div
        ref={gridRef}
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
            selected={editing && selectedElementId === element.id}
            previewPosition={drag?.element.id === element.id ? { x: drag.x, y: drag.y } : undefined}
            onOpenSurface={onOpenSurface}
            onPointerDown={beginDrag}
            onPointerMove={previewDrag}
            onPointerUp={finishDrag}
            onPointerCancel={cancelDrag}
            onReposition={moveElement}
            onResize={resizeElement}
            onRemove={removeElement}
          />
        ))}
      </div>
      <div className="navigator"><Search size={17} /><span>跳转到 Surface 或告诉 Agent 你要去哪里...</span><kbd>Enter</kbd></div>
    </section>
  );
}
