import { z } from "zod";

const timePanelSchema = z.object({
  type: z.literal("timePanel"),
  id: z.string(),
  title: z.string(),
  timezone: z.string(),
  column: z.number(),
});

const appIconSchema = z.object({
  type: z.literal("appIcon"),
  id: z.string(),
  title: z.string(),
  icon: z.string(),
  status: z.string(),
  targetSurfaceId: z.string(),
  column: z.number(),
});

export const runtimeSnapshotSchema = z.object({
  revision: z.number(),
  currentSurfaceId: z.string(),
  breadcrumb: z.array(z.object({ surfaceId: z.string(), title: z.string() })),
  surface: z.object({
    id: z.string(),
    title: z.string(),
    icon: z.string(),
    elements: z.array(z.discriminatedUnion("type", [timePanelSchema, appIconSchema])),
  }),
});

export type RuntimeSnapshot = z.infer<typeof runtimeSnapshotSchema>;
export type SurfaceElement = RuntimeSnapshot["surface"]["elements"][number];
