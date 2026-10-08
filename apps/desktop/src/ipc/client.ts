import { invoke } from "@tauri-apps/api/core";
import { runtimeSnapshotSchema, type RuntimeSnapshot } from "../contracts";

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  const response = await invoke<unknown>("get_runtime_snapshot");
  return runtimeSnapshotSchema.parse(response);
}
