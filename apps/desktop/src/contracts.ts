import { z } from "zod";

const gridRectSchema = z.object({
  x: z.number(),
  y: z.number(),
  width: z.number().positive(),
  height: z.number().positive(),
});

const timePanelSchema = z.object({
  type: z.literal("timePanel"),
  id: z.string(),
  title: z.string(),
  timezone: z.string(),
  rect: gridRectSchema,
});

const appIconSchema = z.object({
  type: z.literal("appIcon"),
  id: z.string(),
  title: z.string(),
  icon: z.string(),
  status: z.string(),
  targetSurfaceId: z.string(),
  rect: gridRectSchema,
});

const textPanelSchema = z.object({
  type: z.literal("textPanel"),
  id: z.string(),
  title: z.string(),
  runs: z.array(z.discriminatedUnion("type", [
    z.object({ type: z.literal("text"), content: z.string() }),
    z.object({ type: z.literal("surfaceLink"), label: z.string(), targetSurfaceId: z.string() }),
  ])),
  rect: gridRectSchema,
});

export const runtimeSnapshotSchema = z.object({
  revision: z.number(),
  currentSurfaceId: z.string(),
  breadcrumb: z.array(z.object({ surfaceId: z.string(), title: z.string() })),
  surface: z.object({
    id: z.string(),
    title: z.string(),
    icon: z.string(),
    columns: z.number().positive(),
    rows: z.number().positive(),
    elements: z.array(z.discriminatedUnion("type", [timePanelSchema, appIconSchema, textPanelSchema])),
  }),
});

export type RuntimeSnapshot = z.infer<typeof runtimeSnapshotSchema>;
export type SurfaceElement = RuntimeSnapshot["surface"]["elements"][number];
