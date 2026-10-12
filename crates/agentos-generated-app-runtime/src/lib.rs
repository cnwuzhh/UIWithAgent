use agentos_agent_ledger::{
    AgentLedger, LedgerActor, LedgerEventType, LedgerStreamId, NewArtifact, NewLedgerEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedAppPackageDraft {
    pub manifest: GeneratedAppManifest,
    pub files: Vec<GeneratedFileDraft>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedAppManifest {
    pub app_id: String,
    pub display_name: String,
    pub working_directory: String,
    pub build_commands: Vec<ProcessCommand>,
    pub run_command: ProcessCommand,
    pub outputs: Vec<GeneratedOutputDefinition>,
    pub ui_bindings: Vec<GeneratedUiBinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessCommand {
    pub program: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub environment: Vec<EnvironmentBinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentBinding {
    pub name: String,
    pub credential_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedOutputDefinition {
    pub name: String,
    pub schema_version: u32,
    pub json_schema: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedUiBinding {
    pub output: String,
    pub type_id: String,
    pub adapter_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFileDraft {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub executable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedPackage {
    pub app_id: String,
    pub revision: u32,
    pub workspace: PathBuf,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildCommandExecution {
    pub command_index: usize,
    pub exit_code: i32,
    pub duration_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildExecution {
    pub app_id: String,
    pub revision: u32,
    pub ledger_stream_id: LedgerStreamId,
    pub commands: Vec<BuildCommandExecution>,
}

pub fn validate_package(draft: &GeneratedAppPackageDraft) -> Result<(), String> {
    validate_identifier(&draft.manifest.app_id, "appId")?;
    if draft.manifest.display_name.trim().is_empty() {
        return Err("displayName cannot be empty".into());
    }
    if draft.manifest.working_directory != "." {
        return Err("workingDirectory must be '.' in the first runtime slice".into());
    }
    if draft.manifest.run_command.program.trim().is_empty() {
        return Err("runCommand.program cannot be empty".into());
    }
    if draft.manifest.outputs.is_empty() {
        return Err("manifest must declare at least one output".into());
    }
    if draft.files.is_empty() {
        return Err("generated package must contain source files".into());
    }

    let output_names = draft
        .manifest
        .outputs
        .iter()
        .map(|output| output.name.as_str())
        .collect::<HashSet<_>>();
    if output_names.len() != draft.manifest.outputs.len()
        || output_names.iter().any(|name| name.trim().is_empty())
    {
        return Err("output names must be non-empty and unique".into());
    }
    for binding in &draft.manifest.ui_bindings {
        if !output_names.contains(binding.output.as_str()) {
            return Err(format!(
                "UI binding references unknown output: {}",
                binding.output
            ));
        }
    }

    let mut paths = HashSet::new();
    for file in &draft.files {
        validate_relative_path(&file.path)?;
        if file.path.eq_ignore_ascii_case("MANIFEST.json") {
            return Err("MANIFEST.json is owned by the Host".into());
        }
        if !paths.insert(file.path.clone()) {
            return Err(format!("duplicate generated path: {}", file.path));
        }
    }
    Ok(())
}

pub fn materialize_package(
    workspace_root: &Path,
    revision: u32,
    draft: &GeneratedAppPackageDraft,
) -> Result<MaterializedPackage, String> {
    validate_package(draft)?;
    if revision == 0 {
        return Err("package revision must be positive".into());
    }
    fs::create_dir_all(workspace_root).map_err(io_error)?;
    let app_root = workspace_root.join(&draft.manifest.app_id);
    fs::create_dir_all(&app_root).map_err(io_error)?;
    let final_path = app_root.join(format!("revision-{revision}"));
    if final_path.exists() {
        return Err(format!(
            "package revision already exists: {}",
            final_path.display()
        ));
    }
    let staging_path = app_root.join(format!(".revision-{revision}.staging"));
    if staging_path.exists() {
        fs::remove_dir_all(&staging_path).map_err(io_error)?;
    }
    fs::create_dir(&staging_path).map_err(io_error)?;

    let result = (|| {
        for file in &draft.files {
            let path = staging_path.join(&file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(io_error)?;
            }
            fs::write(&path, file.content.as_bytes()).map_err(io_error)?;
        }
        let manifest = serde_json::to_vec_pretty(&draft.manifest)
            .map_err(|error| format!("cannot serialize manifest: {error}"))?;
        fs::write(staging_path.join("MANIFEST.json"), manifest).map_err(io_error)?;
        fs::rename(&staging_path, &final_path).map_err(io_error)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_path);
    }
    result?;

    let mut files = draft
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    files.push("MANIFEST.json".into());
    files.sort();
    Ok(MaterializedPackage {
        app_id: draft.manifest.app_id.clone(),
        revision,
        workspace: final_path,
        files,
    })
}

pub async fn build_package(
    ledger: &dyn AgentLedger,
    workspace: &Path,
    revision: u32,
    manifest: &GeneratedAppManifest,
    resolved_environment: &BTreeMap<String, String>,
) -> Result<BuildExecution, String> {
    if revision == 0 {
        return Err("package revision must be positive".into());
    }
    if !workspace.is_dir() {
        return Err(format!(
            "generated workspace does not exist: {}",
            workspace.display()
        ));
    }
    let stream_id = ledger
        .create_stream(&format!("build-{}-revision-{revision}", manifest.app_id))
        .await
        .map_err(ledger_error)?;
    let workspace_files = read_workspace_snapshot(workspace)?;
    ledger
        .append(
            NewLedgerEvent {
                stream_id: stream_id.clone(),
                event_type: LedgerEventType::WorkspaceSnapshotRecorded,
                actor: LedgerActor::AgentRuntime,
                causation_event_id: None,
                payload: serde_json::json!({
                    "appId": manifest.app_id,
                    "revision": revision,
                    "fileCount": workspace_files.len(),
                }),
            },
            workspace_files
                .into_iter()
                .map(|(path, bytes)| NewArtifact {
                    relation: format!("file:{path}"),
                    media_type: "application/octet-stream".into(),
                    bytes,
                })
                .collect(),
        )
        .await
        .map_err(ledger_error)?;
    let mut executions = Vec::new();

    for (command_index, process_command) in manifest.build_commands.iter().enumerate() {
        let environment = resolve_command_environment(process_command, resolved_environment)?;
        let proposed = ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::CommandProposed,
                    actor: LedgerActor::AgentRuntime,
                    causation_event_id: None,
                    payload: serde_json::json!({
                        "appId": manifest.app_id,
                        "revision": revision,
                        "commandIndex": command_index,
                        "program": process_command.program,
                        "args": process_command.args,
                        "cwd": workspace,
                        "environmentNames": environment.keys().collect::<Vec<_>>(),
                    }),
                },
                Vec::new(),
            )
            .await
            .map_err(ledger_error)?;
        let confirmation = ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::ConfirmationResolved,
                    actor: LedgerActor::User,
                    causation_event_id: Some(proposed.event_id),
                    payload: serde_json::json!({
                        "accepted": true,
                        "scope": "build",
                        "revision": revision,
                    }),
                },
                Vec::new(),
            )
            .await
            .map_err(ledger_error)?;

        let started_at = Instant::now();
        let mut child = match Command::new(&process_command.program)
            .args(&process_command.args)
            .envs(&environment)
            .current_dir(workspace)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let message = format!("cannot start build command {command_index}: {error}");
                let exited = ledger
                    .append(
                        NewLedgerEvent {
                            stream_id: stream_id.clone(),
                            event_type: LedgerEventType::ProcessExited,
                            actor: LedgerActor::AgentRuntime,
                            causation_event_id: Some(confirmation.event_id),
                            payload: serde_json::json!({
                                "commandIndex": command_index,
                                "revision": revision,
                                "exitCode": null,
                                "stage": "spawn",
                            }),
                        },
                        vec![NewArtifact::utf8(
                            "error",
                            "text/plain; charset=utf-8",
                            &message,
                        )],
                    )
                    .await
                    .map_err(ledger_error)?;
                ledger
                    .append(
                        NewLedgerEvent {
                            stream_id: stream_id.clone(),
                            event_type: LedgerEventType::BuildFailed,
                            actor: LedgerActor::AgentRuntime,
                            causation_event_id: Some(exited.event_id),
                            payload: serde_json::json!({
                                "appId": manifest.app_id,
                                "revision": revision,
                                "failedCommandIndex": command_index,
                                "exitCode": null,
                                "stage": "spawn",
                            }),
                        },
                        Vec::new(),
                    )
                    .await
                    .map_err(ledger_error)?;
                return Err(message);
            }
        };
        let pid = child.id();
        let started = match ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::ProcessStarted,
                    actor: LedgerActor::AgentRuntime,
                    causation_event_id: Some(confirmation.event_id),
                    payload: serde_json::json!({
                        "commandIndex": command_index,
                        "revision": revision,
                        "pid": pid,
                    }),
                },
                Vec::new(),
            )
            .await
        {
            Ok(receipt) => receipt,
            Err(error) => {
                let _ = child.kill().await;
                return Err(ledger_error(error));
            }
        };
        let output = child
            .wait_with_output()
            .await
            .map_err(|error| format!("cannot wait for build command {command_index}: {error}"))?;
        let duration_ms = started_at.elapsed().as_millis();

        ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::ProcessStdout,
                    actor: LedgerActor::AgentRuntime,
                    causation_event_id: Some(started.event_id.clone()),
                    payload: serde_json::json!({ "commandIndex": command_index }),
                },
                vec![NewArtifact {
                    relation: "stdout".into(),
                    media_type: "application/octet-stream".into(),
                    bytes: output.stdout,
                }],
            )
            .await
            .map_err(ledger_error)?;
        ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::ProcessStderr,
                    actor: LedgerActor::AgentRuntime,
                    causation_event_id: Some(started.event_id.clone()),
                    payload: serde_json::json!({ "commandIndex": command_index }),
                },
                vec![NewArtifact {
                    relation: "stderr".into(),
                    media_type: "application/octet-stream".into(),
                    bytes: output.stderr,
                }],
            )
            .await
            .map_err(ledger_error)?;
        let exit_code = output.status.code().unwrap_or(-1);
        ledger
            .append(
                NewLedgerEvent {
                    stream_id: stream_id.clone(),
                    event_type: LedgerEventType::ProcessExited,
                    actor: LedgerActor::AgentRuntime,
                    causation_event_id: Some(started.event_id),
                    payload: serde_json::json!({
                        "commandIndex": command_index,
                        "revision": revision,
                        "exitCode": exit_code,
                        "durationMs": duration_ms,
                    }),
                },
                Vec::new(),
            )
            .await
            .map_err(ledger_error)?;
        executions.push(BuildCommandExecution {
            command_index,
            exit_code,
            duration_ms,
        });
        if !output.status.success() {
            ledger
                .append(
                    NewLedgerEvent {
                        stream_id: stream_id.clone(),
                        event_type: LedgerEventType::BuildFailed,
                        actor: LedgerActor::AgentRuntime,
                        causation_event_id: None,
                        payload: serde_json::json!({
                            "appId": manifest.app_id,
                            "revision": revision,
                            "failedCommandIndex": command_index,
                            "exitCode": exit_code,
                        }),
                    },
                    Vec::new(),
                )
                .await
                .map_err(ledger_error)?;
            return Err(format!(
                "build command {command_index} exited with code {exit_code}"
            ));
        }
    }

    ledger
        .append(
            NewLedgerEvent {
                stream_id: stream_id.clone(),
                event_type: LedgerEventType::BuildCompleted,
                actor: LedgerActor::AgentRuntime,
                causation_event_id: None,
                payload: serde_json::json!({
                    "appId": manifest.app_id,
                    "revision": revision,
                    "commandCount": executions.len(),
                }),
            },
            Vec::new(),
        )
        .await
        .map_err(ledger_error)?;

    Ok(BuildExecution {
        app_id: manifest.app_id.clone(),
        revision,
        ledger_stream_id: stream_id,
        commands: executions,
    })
}

fn resolve_command_environment(
    command: &ProcessCommand,
    resolved_environment: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    command
        .environment
        .iter()
        .map(|binding| {
            resolved_environment
                .get(&binding.name)
                .cloned()
                .map(|value| (binding.name.clone(), value))
                .ok_or_else(|| format!("missing environment binding: {}", binding.name))
        })
        .collect()
}

fn read_workspace_snapshot(workspace: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    fn visit(
        root: &Path,
        directory: &Path,
        files: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        let mut entries = fs::read_dir(directory)
            .map_err(io_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(io_error)?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().map_err(io_error)?;
            if file_type.is_symlink() {
                return Err(format!(
                    "generated workspace contains unsupported symlink: {}",
                    path.display()
                ));
            }
            if file_type.is_dir() {
                visit(root, &path, files)?;
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| "workspace snapshot path escaped root".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, fs::read(&path).map_err(io_error)?));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(workspace, workspace, &mut files)?;
    Ok(files)
}

fn ledger_error(error: impl std::fmt::Display) -> String {
    format!("Agent Ledger write failed; build stopped: {error}")
}

fn validate_identifier(value: &str, name: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || value.starts_with('-')
        || value.ends_with('-')
    {
        return Err(format!(
            "{name} must use lowercase ASCII letters, digits, and internal hyphens"
        ));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return Err(format!("generated path must be relative: {value}"));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!("generated path escapes workspace: {value}"));
    }
    Ok(())
}

fn io_error(error: std::io::Error) -> String {
    format!("generated workspace IO failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentos_agent_ledger::{
        AgentLedger, AgentLedgerEvent, InMemoryAgentLedger, LedgerAppendReceipt, LedgerError,
        LedgerEventType,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingLedger {
        inner: InMemoryAgentLedger,
        stream_id: Mutex<Option<LedgerStreamId>>,
    }

    #[async_trait::async_trait]
    impl AgentLedger for CapturingLedger {
        async fn create_stream(&self, subject_id: &str) -> Result<LedgerStreamId, LedgerError> {
            let stream_id = self.inner.create_stream(subject_id).await?;
            *self.stream_id.lock().unwrap() = Some(stream_id.clone());
            Ok(stream_id)
        }

        async fn append(
            &self,
            event: NewLedgerEvent,
            artifacts: Vec<NewArtifact>,
        ) -> Result<LedgerAppendReceipt, LedgerError> {
            self.inner.append(event, artifacts).await
        }

        async fn read_stream(
            &self,
            stream_id: &LedgerStreamId,
        ) -> Result<Vec<AgentLedgerEvent>, LedgerError> {
            self.inner.read_stream(stream_id).await
        }

        async fn read_artifact(&self, hash: &str) -> Result<Vec<u8>, LedgerError> {
            self.inner.read_artifact(hash).await
        }

        async fn verify_stream(&self, stream_id: &LedgerStreamId) -> Result<(), LedgerError> {
            self.inner.verify_stream(stream_id).await
        }
    }

    fn draft(path: &str) -> GeneratedAppPackageDraft {
        GeneratedAppPackageDraft {
            manifest: GeneratedAppManifest {
                app_id: "finance-news".into(),
                display_name: "Finance News".into(),
                working_directory: ".".into(),
                build_commands: vec![],
                run_command: ProcessCommand {
                    program: "python".into(),
                    args: vec!["main.py".into()],
                    environment: vec![],
                },
                outputs: vec![GeneratedOutputDefinition {
                    name: "articles".into(),
                    schema_version: 1,
                    json_schema: serde_json::json!({"type": "array"}),
                }],
                ui_bindings: vec![GeneratedUiBinding {
                    output: "articles".into(),
                    type_id: "text.panel".into(),
                    adapter_id: Some("articles-to-text-panel".into()),
                }],
            },
            files: vec![GeneratedFileDraft {
                path: path.into(),
                content: "print('ok')\n".into(),
                executable: false,
            }],
        }
    }

    #[test]
    fn materializes_manifest_and_source_in_an_immutable_revision() {
        let root = tempfile::tempdir().unwrap();
        let package = materialize_package(root.path(), 1, &draft("src/main.py")).unwrap();

        assert_eq!(package.files, ["MANIFEST.json", "src/main.py"]);
        assert_eq!(
            fs::read_to_string(package.workspace.join("src/main.py")).unwrap(),
            "print('ok')\n"
        );
        assert!(package.workspace.join("MANIFEST.json").is_file());
        assert!(materialize_package(root.path(), 1, &draft("src/main.py")).is_err());
    }

    #[test]
    fn rejects_workspace_escape_and_host_manifest_overwrite() {
        assert!(validate_package(&draft("../AgentOS/Cargo.toml")).is_err());
        assert!(validate_package(&draft("MANIFEST.json")).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn build_records_exact_stdout_stderr_and_exit() {
        let root = tempfile::tempdir().unwrap();
        let mut package = draft("src/main.py");
        package.manifest.build_commands = vec![ProcessCommand {
            program: "sh".into(),
            args: vec!["-c".into(), "printf out; printf err >&2".into()],
            environment: vec![],
        }];
        let materialized = materialize_package(root.path(), 1, &package).unwrap();
        let ledger = InMemoryAgentLedger::default();

        let build = build_package(
            &ledger,
            &materialized.workspace,
            1,
            &package.manifest,
            &BTreeMap::new(),
        )
        .await
        .unwrap();

        assert_eq!(build.commands[0].exit_code, 0);
        let events = ledger.read_stream(&build.ledger_stream_id).await.unwrap();
        assert_eq!(
            events[0].event_type,
            LedgerEventType::WorkspaceSnapshotRecorded
        );
        let source = events[0]
            .artifacts
            .iter()
            .find(|artifact| artifact.relation == "file:src/main.py")
            .unwrap();
        assert_eq!(
            ledger.read_artifact(&source.hash).await.unwrap(),
            b"print('ok')\n"
        );
        assert!(
            events[0]
                .artifacts
                .iter()
                .any(|artifact| artifact.relation == "file:MANIFEST.json")
        );
        let stdout = events
            .iter()
            .find(|event| event.event_type == LedgerEventType::ProcessStdout)
            .unwrap();
        let stderr = events
            .iter()
            .find(|event| event.event_type == LedgerEventType::ProcessStderr)
            .unwrap();
        assert_eq!(
            ledger
                .read_artifact(&stdout.artifacts[0].hash)
                .await
                .unwrap(),
            b"out"
        );
        assert_eq!(
            ledger
                .read_artifact(&stderr.artifacts[0].hash)
                .await
                .unwrap(),
            b"err"
        );
        assert_eq!(
            events.last().unwrap().event_type,
            LedgerEventType::BuildCompleted
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn build_stops_after_first_nonzero_command() {
        let root = tempfile::tempdir().unwrap();
        let mut package = draft("src/main.py");
        package.manifest.build_commands = vec![
            ProcessCommand {
                program: "sh".into(),
                args: vec!["-c".into(), "exit 7".into()],
                environment: vec![],
            },
            ProcessCommand {
                program: "sh".into(),
                args: vec!["-c".into(), "touch should-not-exist".into()],
                environment: vec![],
            },
        ];
        let materialized = materialize_package(root.path(), 1, &package).unwrap();
        let ledger = InMemoryAgentLedger::default();

        let error = build_package(
            &ledger,
            &materialized.workspace,
            1,
            &package.manifest,
            &BTreeMap::new(),
        )
        .await
        .unwrap_err();

        assert_eq!(error, "build command 0 exited with code 7");
        assert!(!materialized.workspace.join("should-not-exist").exists());
    }

    #[tokio::test]
    async fn build_records_terminal_failure_when_program_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let mut package = draft("src/main.py");
        package.manifest.build_commands = vec![ProcessCommand {
            program: "agentos-program-that-does-not-exist".into(),
            args: vec![],
            environment: vec![],
        }];
        let materialized = materialize_package(root.path(), 1, &package).unwrap();
        let ledger = CapturingLedger::default();

        let error = build_package(
            &ledger,
            &materialized.workspace,
            1,
            &package.manifest,
            &BTreeMap::new(),
        )
        .await
        .unwrap_err();

        assert!(error.starts_with("cannot start build command 0:"));
        let stream_id = ledger.stream_id.lock().unwrap().clone().unwrap();
        let events = ledger.read_stream(&stream_id).await.unwrap();
        assert_eq!(
            events.last().unwrap().event_type,
            LedgerEventType::BuildFailed
        );
        assert_eq!(events.last().unwrap().payload["stage"], "spawn");
        assert_eq!(
            events.last().unwrap().causation_event_id,
            Some(events[events.len() - 2].event_id.clone())
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn build_rejects_workspace_symlink_before_starting_process() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let mut package = draft("src/main.py");
        package.manifest.build_commands = vec![ProcessCommand {
            program: "sh".into(),
            args: vec!["-c".into(), "touch should-not-exist".into()],
            environment: vec![],
        }];
        let materialized = materialize_package(root.path(), 1, &package).unwrap();
        symlink(
            materialized.workspace.join("src/main.py"),
            materialized.workspace.join("linked.py"),
        )
        .unwrap();
        let ledger = InMemoryAgentLedger::default();

        let error = build_package(
            &ledger,
            &materialized.workspace,
            1,
            &package.manifest,
            &BTreeMap::new(),
        )
        .await
        .unwrap_err();

        assert!(error.contains("unsupported symlink"));
        assert!(!materialized.workspace.join("should-not-exist").exists());
    }
}
