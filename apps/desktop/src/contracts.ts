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
  status: z.enum(["ready", "building"]),
  targetSurfaceId: z.string().optional(),
  buildTaskId: z.string().optional(),
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

export const buildSubmissionSchema = z.object({
  message: z.string(),
  snapshot: runtimeSnapshotSchema,
});

export const buildTaskSchema = z.object({
  id: z.string(),
  surfaceId: z.string(),
  request: z.string(),
  status: z.literal("building"),
  agentMessage: z.string(),
  placeholderElementId: z.string(),
});

export const agentPlanSchema = z.object({
  mode: z.literal("planOnly"),
  request: z.string(),
  goal: z.string(),
  approach: z.string().min(1),
  architecture: z.array(z.string().min(1)).min(1),
  dataContract: z.string().min(1),
  buildCommands: z.array(z.string().min(1)).min(1),
  runCommand: z.string().min(1),
  steps: z.array(z.object({
    title: z.string().min(1),
    action: z.string().min(1),
    deliverables: z.array(z.string().min(1)).min(1),
    verification: z.string().min(1),
  }).strict()).min(4).max(8),
  uiBinding: z.object({
    typeId: z.string().min(1),
    config: z.record(z.string(), z.union([
      z.string(),
      z.array(z.string()),
      z.boolean(),
      z.number().int(),
    ])),
    adapterId: z.string().min(1).nullable(),
    status: z.enum(["ready", "blocked"]),
    notes: z.string().min(1),
  }).strict(),
  impact: z.string(),
  assumptions: z.array(z.string()),
  capabilities: z.array(z.enum(["surface.read", "toolbox.read"])),
  basedOnRevision: z.number(),
});

export const reviewedAgentPlanSchema = z.object({
  planId: z.string().min(1),
  plan: agentPlanSchema,
}).strict();

const processCommandSchema = z.object({
  program: z.string().min(1),
  args: z.array(z.string()),
  environment: z.array(z.object({
    name: z.string().min(1),
    credentialRef: z.string().min(1).nullable(),
  }).strict()),
}).strict();

export const generatedAppPackageSchema = z.object({
  manifest: z.object({
    appId: z.string().min(1),
    displayName: z.string().min(1),
    workingDirectory: z.literal("."),
    buildCommands: z.array(processCommandSchema),
    runCommand: processCommandSchema,
    outputs: z.array(z.object({
      name: z.string().min(1),
      schemaVersion: z.number().int().positive(),
      jsonSchema: z.record(z.string(), z.unknown()),
    }).strict()).min(1),
    uiBindings: z.array(z.object({
      output: z.string().min(1),
      typeId: z.string().min(1),
      adapterId: z.string().min(1).nullable(),
    }).strict()),
  }).strict(),
  files: z.array(z.object({
    path: z.string().min(1),
    content: z.string(),
    executable: z.boolean(),
  }).strict()).min(1),
}).strict();

export const agentBuildResultSchema = z.object({
  appId: z.string().min(1),
  revision: z.number().int().positive(),
  workspace: z.string().min(1),
  files: z.array(z.string().min(1)).min(1),
  commands: z.array(z.object({
    commandIndex: z.number().int().nonnegative(),
    exitCode: z.number().int(),
    durationMs: z.number().nonnegative(),
  }).strict()),
}).strict();

const toolboxConfigFieldSchema = z.object({
  name: z.string().min(1),
  valueKind: z.enum(["string", "stringList", "boolean", "integer", "reference"]),
  required: z.boolean(),
  allowedValues: z.array(z.string()),
}).strict();

const toolboxElementSchema = z.object({
  typeId: z.string().min(1),
  version: z.number().int().positive(),
  title: z.string().min(1),
  description: z.string().min(1),
  category: z.enum(["basic", "weather", "media", "calculator", "finance"]),
  availability: z.enum(["available", "preview"]),
  defaultSize: z.object({
    width: z.number().int().positive(),
    height: z.number().int().positive(),
  }).strict(),
  configFields: z.array(toolboxConfigFieldSchema),
  requiredCapabilities: z.array(z.string()),
}).strict();

const toolboxRecipeSchema = z.object({
  recipeId: z.string().min(1),
  title: z.string().min(1),
  requiredElements: z.array(z.string()),
  optionalElements: z.array(z.string()),
}).strict();

export const toolboxDescriptorSchema = z.object({
  version: z.string().min(1),
  elements: z.array(toolboxElementSchema),
  recipes: z.array(toolboxRecipeSchema),
}).strict();

export type RuntimeSnapshot = z.infer<typeof runtimeSnapshotSchema>;
export type SurfaceElement = RuntimeSnapshot["surface"]["elements"][number];
export type BuildSubmission = z.infer<typeof buildSubmissionSchema>;
export type BuildTask = z.infer<typeof buildTaskSchema>;
export type AgentPlan = z.infer<typeof agentPlanSchema>;
export type ReviewedAgentPlan = z.infer<typeof reviewedAgentPlanSchema>;
export type GeneratedAppPackage = z.infer<typeof generatedAppPackageSchema>;
export type AgentBuildResult = z.infer<typeof agentBuildResultSchema>;
export type ToolboxDescriptor = z.infer<typeof toolboxDescriptorSchema>;
