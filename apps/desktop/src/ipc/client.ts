import { invoke } from "@tauri-apps/api/core";
import { runtimeSnapshotSchema, type RuntimeSnapshot } from "../contracts";

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("get_runtime_snapshot");
  return runtimeSnapshotSchema.parse(response);
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

export async function toggleFullscreen(): Promise<boolean> {
  return invoke<boolean>("toggle_fullscreen");
}

export async function isFullscreen(): Promise<boolean> {
  return invoke<boolean>("is_fullscreen");
}
