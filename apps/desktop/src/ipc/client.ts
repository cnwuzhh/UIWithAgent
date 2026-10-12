import { invoke } from "@tauri-apps/api/core";
import { agentBuildResultSchema, buildSubmissionSchema, buildTaskSchema, generatedAppPackageSchema, reviewedAgentPlanSchema, runtimeSnapshotSchema, toolboxDescriptorSchema, type AgentBuildResult, type BuildSubmission, type BuildTask, type GeneratedAppPackage, type ReviewedAgentPlan, type RuntimeSnapshot, type ToolboxDescriptor } from "../contracts";

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("get_runtime_snapshot");
  return runtimeSnapshotSchema.parse(response);
}

export async function getToolboxDescriptor(): Promise<ToolboxDescriptor> {
  const response = await invoke<unknown>("get_toolbox_descriptor");
  return toolboxDescriptorSchema.parse(response);
}

export async function openSurface(surfaceId: string): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("open_surface", { surfaceId });
  return runtimeSnapshotSchema.parse(response);
}

export async function repositionElement(
  surfaceId: string,
  elementId: string,
  x: number,
  y: number,
): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("reposition_element", { surfaceId, elementId, x, y });
  return runtimeSnapshotSchema.parse(response);
}

export async function resizeElement(
  surfaceId: string,
  elementId: string,
  width: number,
  height: number,
): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("resize_element", { surfaceId, elementId, width, height });
  return runtimeSnapshotSchema.parse(response);
}

export async function addTimePanel(
  surfaceId: string,
  title: string,
  timezone: string,
): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("add_time_panel", { surfaceId, title, timezone });
  return runtimeSnapshotSchema.parse(response);
}

export async function removeElement(surfaceId: string, elementId: string): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("remove_element", { surfaceId, elementId });
  return runtimeSnapshotSchema.parse(response);
}

export async function submitBuild(surfaceId: string, request: string): Promise<BuildSubmission> {
  const response = await invoke<unknown>("submit_build", { surfaceId, request });
  return buildSubmissionSchema.parse(response);
}

export async function getBuildTask(buildTaskId: string): Promise<BuildTask> {
  const response = await invoke<unknown>("get_build_task", { buildTaskId });
  return buildTaskSchema.parse(response);
}

export async function planAgent(surfaceId: string, request: string): Promise<ReviewedAgentPlan> {
  const response = await invoke<unknown>("plan_agent", { surfaceId, request });
  return reviewedAgentPlanSchema.parse(response);
}

export async function generateAgentPackage(planId: string): Promise<GeneratedAppPackage> {
  const response = await invoke<unknown>("generate_agent_package", { planId });
  return generatedAppPackageSchema.parse(response);
}

export async function buildAgentPackage(planId: string): Promise<AgentBuildResult> {
  const response = await invoke<unknown>("build_agent_package", { planId });
  return agentBuildResultSchema.parse(response);
}

export async function toggleFullscreen(): Promise<boolean> {
  return invoke<boolean>("toggle_fullscreen");
}

export async function isFullscreen(): Promise<boolean> {
  return invoke<boolean>("is_fullscreen");
}
