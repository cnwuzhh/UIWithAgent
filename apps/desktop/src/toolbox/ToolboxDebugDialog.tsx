import { useEffect, useMemo, useState } from "react";
import { Box, Boxes, RefreshCw, ShieldCheck, X } from "lucide-react";
import type { ToolboxDescriptor } from "../contracts";
import { getToolboxDescriptor } from "../ipc/client";

type AvailabilityFilter = "all" | "available" | "preview";
type ToolboxElement = ToolboxDescriptor["elements"][number];

interface ToolboxDebugDialogProps {
  onClose: () => void;
}

const categoryLabels: Record<ToolboxElement["category"], string> = {
  basic: "基础",
  weather: "天气",
  media: "媒体",
  calculator: "计算器",
  finance: "财经",
};

const valueKindLabels: Record<ToolboxElement["configFields"][number]["valueKind"], string> = {
  string: "string",
  stringList: "string[]",
  boolean: "boolean",
  integer: "integer",
  reference: "reference",
};

export function ToolboxDebugDialog({ onClose }: ToolboxDebugDialogProps) {
  const [descriptor, setDescriptor] = useState<ToolboxDescriptor>();
  const [selectedTypeId, setSelectedTypeId] = useState<string>();
  const [filter, setFilter] = useState<AvailabilityFilter>("all");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  async function loadToolbox() {
    setLoading(true);
    setError(undefined);
    try {
      const next = await getToolboxDescriptor();
      setDescriptor(next);
      setSelectedTypeId((current) => current && next.elements.some((item) => item.typeId === current)
        ? current
        : next.elements[0]?.typeId);
    } catch (loadError: unknown) {
      setError(loadError instanceof Error ? loadError.message : "无法读取 Toolbox Catalog");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadToolbox();
  }, []);

  const elements = useMemo(() => descriptor?.elements.filter((element) => (
    filter === "all" || element.availability === filter
  )) ?? [], [descriptor, filter]);
  const selected = descriptor?.elements.find((element) => element.typeId === selectedTypeId);
  const availableCount = descriptor?.elements.filter((element) => element.availability === "available").length ?? 0;
  const previewCount = descriptor?.elements.filter((element) => element.availability === "preview").length ?? 0;

  return (
    <div className="toolbox-dialog-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="toolbox-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="toolbox-dialog-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="toolbox-dialog-header">
          <div className="toolbox-dialog-heading">
            <span className="toolbox-dialog-mark"><Boxes size={19} /></span>
            <div><span className="eyebrow">RUNTIME CATALOG</span><h2 id="toolbox-dialog-title">Toolbox 调试器</h2></div>
          </div>
          <div className="toolbox-dialog-actions">
            <button title="重新读取 Catalog" aria-label="重新读取 Catalog" onClick={() => void loadToolbox()} disabled={loading}><RefreshCw size={17} /></button>
            <button title="关闭" aria-label="关闭 Toolbox 调试器" onClick={onClose}><X size={18} /></button>
          </div>
        </header>

        {loading && !descriptor && <div className="toolbox-dialog-state">正在读取 Runtime Catalog...</div>}
        {error && !descriptor && <div className="toolbox-dialog-state error"><strong>Catalog 不可用</strong><span>{error}</span></div>}
        {descriptor && (
          <div className="toolbox-debug-layout">
            <section className="toolbox-summary" aria-label="Catalog 摘要">
              <div><span>Catalog version</span><strong>{descriptor.version}</strong></div>
              <div><span>Available</span><strong>{availableCount}</strong></div>
              <div><span>Preview</span><strong>{previewCount}</strong></div>
              <div><span>Recipes</span><strong>{descriptor.recipes.length}</strong></div>
              {error && <p>{error}</p>}
            </section>

            <aside className="toolbox-element-index">
              <div className="toolbox-filter" aria-label="Element availability 筛选">
                {(["all", "available", "preview"] as const).map((value) => (
                  <button key={value} className={filter === value ? "active" : ""} onClick={() => setFilter(value)}>
                    {value === "all" ? "全部" : value === "available" ? "Available" : "Preview"}
                  </button>
                ))}
              </div>
              <div className="toolbox-element-list">
                {elements.map((element) => (
                  <button
                    key={element.typeId}
                    className={selectedTypeId === element.typeId ? "selected" : ""}
                    onClick={() => setSelectedTypeId(element.typeId)}
                  >
                    <Box size={15} />
                    <span><strong>{element.title}</strong><code>{element.typeId}</code></span>
                    <i className={element.availability}>{element.availability}</i>
                  </button>
                ))}
              </div>
            </aside>

            <section className="toolbox-element-detail" aria-live="polite">
              {selected ? (
                <>
                  <header>
                    <div><span className="eyebrow">{categoryLabels[selected.category]} · VERSION {selected.version}</span><h3>{selected.title}</h3><code>{selected.typeId}</code></div>
                    <span className={`toolbox-availability ${selected.availability}`}><ShieldCheck size={14} />{selected.availability}</span>
                  </header>
                  <p className="toolbox-description">{selected.description}</p>
                  <dl className="toolbox-facts">
                    <dt>默认尺寸</dt><dd>{selected.defaultSize.width} × {selected.defaultSize.height} grid units</dd>
                    <dt>Category</dt><dd>{selected.category}</dd>
                    <dt>Capabilities</dt><dd>{selected.requiredCapabilities.length ? selected.requiredCapabilities.join(", ") : "none"}</dd>
                  </dl>
                  <div className="toolbox-config">
                    <h4>Config schema <span>{selected.configFields.length} fields</span></h4>
                    {selected.configFields.length ? (
                      <table><thead><tr><th>字段</th><th>类型</th><th>约束</th></tr></thead><tbody>
                        {selected.configFields.map((field) => (
                          <tr key={field.name}>
                            <td><code>{field.name}</code></td>
                            <td>{valueKindLabels[field.valueKind]}</td>
                            <td>{field.required ? "required" : "optional"}{field.allowedValues.length ? ` · ${field.allowedValues.join(" | ")}` : ""}</td>
                          </tr>
                        ))}
                      </tbody></table>
                    ) : <p>当前 Preview 尚未发布配置字段。</p>}
                  </div>
                </>
              ) : <div className="toolbox-empty">从左侧选择一个 Element</div>}
            </section>

            <section className="toolbox-recipes">
              <header><div><span className="eyebrow">COMPOSITIONS</span><h3>App Recipes</h3></div><span>{descriptor.recipes.length} registered</span></header>
              <div className="toolbox-recipe-grid">
                {descriptor.recipes.map((recipe) => (
                  <article key={recipe.recipeId}>
                    <strong>{recipe.title}</strong><code>{recipe.recipeId}</code>
                    <p>required: {recipe.requiredElements.join(", ") || "none"}</p>
                    <p>optional: {recipe.optionalElements.join(", ") || "none"}</p>
                  </article>
                ))}
              </div>
            </section>
          </div>
        )}
      </section>
    </div>
  );
}