use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LedgerStreamId(String);

impl LedgerStreamId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedgerEventType {
    ConversationMessage,
    ModelRequestStarted,
    ModelResponseReceived,
    ModelRequestFailed,
    PackageReviewCompleted,
    PackageRevisionCreated,
    GeneratedFileWritten,
    CommandProposed,
    ConfirmationResolved,
    WorkspaceSnapshotRecorded,
    ProcessStarted,
    ProcessStdout,
    ProcessStderr,
    ProcessExited,
    BuildCompleted,
    BuildFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LedgerActor {
    User,
    AgentRuntime,
    ModelProvider,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewArtifact {
    pub relation: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

impl NewArtifact {
    pub fn utf8(
        relation: impl Into<String>,
        media_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            relation: relation.into(),
            media_type: media_type.into(),
            bytes: content.into().into_bytes(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLedgerEvent {
    pub stream_id: LedgerStreamId,
    pub event_type: LedgerEventType,
    pub actor: LedgerActor,
    pub causation_event_id: Option<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub relation: String,
    pub hash: String,
    pub byte_length: u64,
    pub media_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLedgerEvent {
    pub event_id: String,
    pub stream_id: LedgerStreamId,
    pub sequence: u64,
    pub recorded_at_unix_ms: u128,
    pub event_type: LedgerEventType,
    pub actor: LedgerActor,
    pub causation_event_id: Option<String>,
    pub payload: Value,
    pub artifacts: Vec<ArtifactRef>,
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerAppendReceipt {
    pub event_id: String,
    pub sequence: u64,
    pub event_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    StreamNotFound,
    Unavailable(String),
    Corrupt(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamNotFound => formatter.write_str("ledger stream not found"),
            Self::Unavailable(message) => write!(formatter, "ledger unavailable: {message}"),
            Self::Corrupt(message) => write!(formatter, "ledger corrupt: {message}"),
        }
    }
}

impl std::error::Error for LedgerError {}

#[async_trait]
pub trait AgentLedger: Send + Sync {
    async fn create_stream(&self, subject_id: &str) -> Result<LedgerStreamId, LedgerError>;

    async fn append(
        &self,
        event: NewLedgerEvent,
        artifacts: Vec<NewArtifact>,
    ) -> Result<LedgerAppendReceipt, LedgerError>;

    async fn read_stream(
        &self,
        stream_id: &LedgerStreamId,
    ) -> Result<Vec<AgentLedgerEvent>, LedgerError>;

    async fn read_artifact(&self, hash: &str) -> Result<Vec<u8>, LedgerError>;

    async fn verify_stream(&self, stream_id: &LedgerStreamId) -> Result<(), LedgerError>;
}

#[derive(Clone, Default)]
pub struct InMemoryAgentLedger {
    state: Arc<Mutex<LedgerState>>,
}

#[derive(Default)]
struct LedgerState {
    next_stream_id: u64,
    next_event_id: u64,
    streams: HashMap<LedgerStreamId, Vec<AgentLedgerEvent>>,
    artifacts: HashMap<String, Vec<u8>>,
}

#[async_trait]
impl AgentLedger for InMemoryAgentLedger {
    async fn create_stream(&self, subject_id: &str) -> Result<LedgerStreamId, LedgerError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| LedgerError::Unavailable("state lock poisoned".into()))?;
        state.next_stream_id += 1;
        let stream_id = LedgerStreamId(format!("{subject_id}-ledger-{}", state.next_stream_id));
        state.streams.insert(stream_id.clone(), Vec::new());
        Ok(stream_id)
    }

    async fn append(
        &self,
        event: NewLedgerEvent,
        artifacts: Vec<NewArtifact>,
    ) -> Result<LedgerAppendReceipt, LedgerError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| LedgerError::Unavailable("state lock poisoned".into()))?;
        let (sequence, previous_event_hash) = state
            .streams
            .get(&event.stream_id)
            .ok_or(LedgerError::StreamNotFound)?
            .last()
            .map_or((1, None), |previous| {
                (previous.sequence + 1, Some(previous.event_hash.clone()))
            });

        let artifact_refs = artifacts
            .into_iter()
            .map(|artifact| {
                let hash = sha256_hex(&artifact.bytes);
                let artifact_ref = ArtifactRef {
                    relation: artifact.relation,
                    hash: hash.clone(),
                    byte_length: artifact.bytes.len() as u64,
                    media_type: artifact.media_type,
                };
                state.artifacts.entry(hash).or_insert(artifact.bytes);
                artifact_ref
            })
            .collect::<Vec<_>>();

        state.next_event_id += 1;
        let event_id = format!("ledger-event-{}", state.next_event_id);
        let recorded_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| LedgerError::Unavailable(error.to_string()))?
            .as_millis();
        let event_hash = calculate_event_hash(
            &event_id,
            &event.stream_id,
            sequence,
            recorded_at_unix_ms,
            &event.event_type,
            &event.actor,
            event.causation_event_id.as_deref(),
            &event.payload,
            &artifact_refs,
            previous_event_hash.as_deref(),
        )?;
        let stored_event = AgentLedgerEvent {
            event_id: event_id.clone(),
            stream_id: event.stream_id.clone(),
            sequence,
            recorded_at_unix_ms,
            event_type: event.event_type,
            actor: event.actor,
            causation_event_id: event.causation_event_id,
            payload: event.payload,
            artifacts: artifact_refs,
            previous_event_hash,
            event_hash: event_hash.clone(),
        };
        state
            .streams
            .get_mut(&event.stream_id)
            .ok_or(LedgerError::StreamNotFound)?
            .push(stored_event);

        Ok(LedgerAppendReceipt {
            event_id,
            sequence,
            event_hash,
        })
    }

    async fn read_stream(
        &self,
        stream_id: &LedgerStreamId,
    ) -> Result<Vec<AgentLedgerEvent>, LedgerError> {
        self.state
            .lock()
            .map_err(|_| LedgerError::Unavailable("state lock poisoned".into()))?
            .streams
            .get(stream_id)
            .cloned()
            .ok_or(LedgerError::StreamNotFound)
    }

    async fn read_artifact(&self, hash: &str) -> Result<Vec<u8>, LedgerError> {
        self.state
            .lock()
            .map_err(|_| LedgerError::Unavailable("state lock poisoned".into()))?
            .artifacts
            .get(hash)
            .cloned()
            .ok_or_else(|| LedgerError::Corrupt(format!("missing artifact {hash}")))
    }

    async fn verify_stream(&self, stream_id: &LedgerStreamId) -> Result<(), LedgerError> {
        let state = self
            .state
            .lock()
            .map_err(|_| LedgerError::Unavailable("state lock poisoned".into()))?;
        let events = state
            .streams
            .get(stream_id)
            .ok_or(LedgerError::StreamNotFound)?;
        let mut previous_hash: Option<&str> = None;
        for (index, event) in events.iter().enumerate() {
            if event.sequence != index as u64 + 1
                || event.previous_event_hash.as_deref() != previous_hash
            {
                return Err(LedgerError::Corrupt(format!(
                    "invalid chain at sequence {}",
                    event.sequence
                )));
            }
            for artifact in &event.artifacts {
                let bytes = state.artifacts.get(&artifact.hash).ok_or_else(|| {
                    LedgerError::Corrupt(format!("missing artifact {}", artifact.hash))
                })?;
                if sha256_hex(bytes) != artifact.hash {
                    return Err(LedgerError::Corrupt(format!(
                        "artifact hash mismatch {}",
                        artifact.hash
                    )));
                }
            }
            let expected_hash = calculate_event_hash(
                &event.event_id,
                &event.stream_id,
                event.sequence,
                event.recorded_at_unix_ms,
                &event.event_type,
                &event.actor,
                event.causation_event_id.as_deref(),
                &event.payload,
                &event.artifacts,
                event.previous_event_hash.as_deref(),
            )?;
            if expected_hash != event.event_hash {
                return Err(LedgerError::Corrupt(format!(
                    "event hash mismatch at sequence {}",
                    event.sequence
                )));
            }
            previous_hash = Some(&event.event_hash);
        }
        Ok(())
    }
}

pub struct SqliteAgentLedger {
    connection: Mutex<Connection>,
}

impl SqliteAgentLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LedgerError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(unavailable)?;
        }
        let connection = Connection::open(path).map_err(unavailable)?;
        connection
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                PRAGMA journal_mode = WAL;
                PRAGMA busy_timeout = 5000;

                CREATE TABLE IF NOT EXISTS agent_ledger_streams (
                    stream_id TEXT PRIMARY KEY,
                    subject_id TEXT NOT NULL,
                    created_at_unix_ms INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS agent_ledger_events (
                    event_id TEXT PRIMARY KEY,
                    stream_id TEXT NOT NULL REFERENCES agent_ledger_streams(stream_id),
                    sequence INTEGER NOT NULL,
                    recorded_at_unix_ms INTEGER NOT NULL,
                    event_type_json TEXT NOT NULL,
                    actor_json TEXT NOT NULL,
                    causation_event_id TEXT,
                    payload_json TEXT NOT NULL,
                    previous_event_hash TEXT,
                    event_hash TEXT NOT NULL,
                    UNIQUE(stream_id, sequence)
                );

                CREATE TABLE IF NOT EXISTS agent_ledger_artifacts (
                    hash TEXT PRIMARY KEY,
                    byte_length INTEGER NOT NULL,
                    media_type TEXT NOT NULL,
                    bytes BLOB NOT NULL
                );

                CREATE TABLE IF NOT EXISTS agent_ledger_event_artifacts (
                    event_id TEXT NOT NULL REFERENCES agent_ledger_events(event_id),
                    artifact_hash TEXT NOT NULL REFERENCES agent_ledger_artifacts(hash),
                    relation TEXT NOT NULL,
                    media_type TEXT NOT NULL,
                    byte_length INTEGER NOT NULL,
                    ordinal INTEGER NOT NULL,
                    PRIMARY KEY(event_id, ordinal)
                );
                "#,
            )
            .map_err(unavailable)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

#[async_trait]
impl AgentLedger for SqliteAgentLedger {
    async fn create_stream(&self, subject_id: &str) -> Result<LedgerStreamId, LedgerError> {
        let stream_id = LedgerStreamId(unique_id(&format!("{subject_id}-ledger")));
        let recorded_at = unix_time_ms()?;
        self.connection
            .lock()
            .map_err(|_| LedgerError::Unavailable("database lock poisoned".into()))?
            .execute(
                "INSERT INTO agent_ledger_streams (stream_id, subject_id, created_at_unix_ms) VALUES (?1, ?2, ?3)",
                params![stream_id.as_str(), subject_id, recorded_at],
            )
            .map_err(unavailable)?;
        Ok(stream_id)
    }

    async fn append(
        &self,
        event: NewLedgerEvent,
        artifacts: Vec<NewArtifact>,
    ) -> Result<LedgerAppendReceipt, LedgerError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::Unavailable("database lock poisoned".into()))?;
        let transaction = connection.transaction().map_err(unavailable)?;
        let stream_exists = transaction
            .query_row(
                "SELECT 1 FROM agent_ledger_streams WHERE stream_id = ?1",
                [event.stream_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(unavailable)?
            .is_some();
        if !stream_exists {
            return Err(LedgerError::StreamNotFound);
        }
        let previous = transaction
            .query_row(
                "SELECT sequence, event_hash FROM agent_ledger_events WHERE stream_id = ?1 ORDER BY sequence DESC LIMIT 1",
                [event.stream_id.as_str()],
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(unavailable)?;
        let (sequence, previous_event_hash) = previous
            .map(|(sequence, hash)| (sequence + 1, Some(hash)))
            .unwrap_or((1, None));
        let artifact_refs = artifacts
            .iter()
            .map(|artifact| ArtifactRef {
                relation: artifact.relation.clone(),
                hash: sha256_hex(&artifact.bytes),
                byte_length: artifact.bytes.len() as u64,
                media_type: artifact.media_type.clone(),
            })
            .collect::<Vec<_>>();
        let event_id = unique_id("ledger-event");
        let recorded_at_unix_ms = unix_time_ms()? as u128;
        let event_hash = calculate_event_hash(
            &event_id,
            &event.stream_id,
            sequence,
            recorded_at_unix_ms,
            &event.event_type,
            &event.actor,
            event.causation_event_id.as_deref(),
            &event.payload,
            &artifact_refs,
            previous_event_hash.as_deref(),
        )?;
        transaction
            .execute(
                "INSERT INTO agent_ledger_events (event_id, stream_id, sequence, recorded_at_unix_ms, event_type_json, actor_json, causation_event_id, payload_json, previous_event_hash, event_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event_id,
                    event.stream_id.as_str(),
                    sequence,
                    recorded_at_unix_ms as u64,
                    serialize(&event.event_type)?,
                    serialize(&event.actor)?,
                    event.causation_event_id,
                    serialize(&event.payload)?,
                    previous_event_hash,
                    event_hash,
                ],
            )
            .map_err(unavailable)?;
        for (ordinal, (artifact, artifact_ref)) in artifacts.iter().zip(&artifact_refs).enumerate()
        {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO agent_ledger_artifacts (hash, byte_length, media_type, bytes) VALUES (?1, ?2, ?3, ?4)",
                    params![artifact_ref.hash, artifact_ref.byte_length, artifact_ref.media_type, artifact.bytes],
                )
                .map_err(unavailable)?;
            transaction
                .execute(
                    "INSERT INTO agent_ledger_event_artifacts (event_id, artifact_hash, relation, media_type, byte_length, ordinal) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![event_id, artifact_ref.hash, artifact_ref.relation, artifact_ref.media_type, artifact_ref.byte_length, ordinal],
                )
                .map_err(unavailable)?;
        }
        transaction.commit().map_err(unavailable)?;
        Ok(LedgerAppendReceipt {
            event_id,
            sequence,
            event_hash,
        })
    }

    async fn read_stream(
        &self,
        stream_id: &LedgerStreamId,
    ) -> Result<Vec<AgentLedgerEvent>, LedgerError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LedgerError::Unavailable("database lock poisoned".into()))?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM agent_ledger_streams WHERE stream_id = ?1",
                [stream_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(unavailable)?
            .is_some();
        if !exists {
            return Err(LedgerError::StreamNotFound);
        }
        let mut statement = connection
            .prepare(
                "SELECT event_id, sequence, recorded_at_unix_ms, event_type_json, actor_json, causation_event_id, payload_json, previous_event_hash, event_hash FROM agent_ledger_events WHERE stream_id = ?1 ORDER BY sequence",
            )
            .map_err(unavailable)?;
        let rows = statement
            .query_map([stream_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .map_err(unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(unavailable)?;
        drop(statement);
        rows.into_iter()
            .map(
                |(
                    event_id,
                    sequence,
                    recorded_at_unix_ms,
                    event_type,
                    actor,
                    causation_event_id,
                    payload,
                    previous_event_hash,
                    event_hash,
                )| {
                    let mut artifact_statement = connection
                        .prepare("SELECT relation, artifact_hash, byte_length, media_type FROM agent_ledger_event_artifacts WHERE event_id = ?1 ORDER BY ordinal")
                        .map_err(unavailable)?;
                    let artifacts = artifact_statement
                        .query_map([event_id.as_str()], |row| {
                            Ok(ArtifactRef {
                                relation: row.get(0)?,
                                hash: row.get(1)?,
                                byte_length: row.get(2)?,
                                media_type: row.get(3)?,
                            })
                        })
                        .map_err(unavailable)?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(unavailable)?;
                    Ok(AgentLedgerEvent {
                        event_id,
                        stream_id: stream_id.clone(),
                        sequence,
                        recorded_at_unix_ms: recorded_at_unix_ms as u128,
                        event_type: deserialize(&event_type)?,
                        actor: deserialize(&actor)?,
                        causation_event_id,
                        payload: deserialize(&payload)?,
                        artifacts,
                        previous_event_hash,
                        event_hash,
                    })
                },
            )
            .collect()
    }

    async fn read_artifact(&self, hash: &str) -> Result<Vec<u8>, LedgerError> {
        self.connection
            .lock()
            .map_err(|_| LedgerError::Unavailable("database lock poisoned".into()))?
            .query_row(
                "SELECT bytes FROM agent_ledger_artifacts WHERE hash = ?1",
                [hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(unavailable)?
            .ok_or_else(|| LedgerError::Corrupt(format!("missing artifact {hash}")))
    }

    async fn verify_stream(&self, stream_id: &LedgerStreamId) -> Result<(), LedgerError> {
        let events = self.read_stream(stream_id).await?;
        let mut previous_hash: Option<&str> = None;
        for (index, event) in events.iter().enumerate() {
            if event.sequence != index as u64 + 1
                || event.previous_event_hash.as_deref() != previous_hash
            {
                return Err(LedgerError::Corrupt(format!(
                    "invalid chain at sequence {}",
                    event.sequence
                )));
            }
            for artifact in &event.artifacts {
                let bytes = self.read_artifact(&artifact.hash).await?;
                if bytes.len() as u64 != artifact.byte_length || sha256_hex(&bytes) != artifact.hash
                {
                    return Err(LedgerError::Corrupt(format!(
                        "artifact mismatch {}",
                        artifact.hash
                    )));
                }
            }
            let expected_hash = calculate_event_hash(
                &event.event_id,
                &event.stream_id,
                event.sequence,
                event.recorded_at_unix_ms,
                &event.event_type,
                &event.actor,
                event.causation_event_id.as_deref(),
                &event.payload,
                &event.artifacts,
                event.previous_event_hash.as_deref(),
            )?;
            if expected_hash != event.event_hash {
                return Err(LedgerError::Corrupt(format!(
                    "event hash mismatch at sequence {}",
                    event.sequence
                )));
            }
            previous_hash = Some(&event.event_hash);
        }
        Ok(())
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn unique_id(prefix: &str) -> String {
    let counter = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{prefix}-{timestamp:x}-{counter:x}")
}

fn unix_time_ms() -> Result<u64, LedgerError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(unavailable)
}

fn serialize(value: &impl Serialize) -> Result<String, LedgerError> {
    serde_json::to_string(value).map_err(unavailable)
}

fn deserialize<'a, T: Deserialize<'a>>(value: &'a str) -> Result<T, LedgerError> {
    serde_json::from_str(value).map_err(|error| LedgerError::Corrupt(error.to_string()))
}

fn unavailable(error: impl std::fmt::Display) -> LedgerError {
    LedgerError::Unavailable(error.to_string())
}

#[allow(clippy::too_many_arguments)]
fn calculate_event_hash(
    event_id: &str,
    stream_id: &LedgerStreamId,
    sequence: u64,
    recorded_at_unix_ms: u128,
    event_type: &LedgerEventType,
    actor: &LedgerActor,
    causation_event_id: Option<&str>,
    payload: &Value,
    artifacts: &[ArtifactRef],
    previous_event_hash: Option<&str>,
) -> Result<String, LedgerError> {
    let canonical = serde_json::to_vec(&(
        event_id,
        stream_id,
        sequence,
        recorded_at_unix_ms,
        event_type,
        actor,
        causation_event_id,
        payload,
        artifacts,
        previous_event_hash,
    ))
    .map_err(|error| LedgerError::Unavailable(error.to_string()))?;
    Ok(sha256_hex(&canonical))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn records_content_addressed_artifacts_in_a_verifiable_chain() {
        futures_lite::future::block_on(async {
            let ledger = InMemoryAgentLedger::default();
            let stream_id = ledger.create_stream("build-7").await.unwrap();
            ledger
                .append(
                    NewLedgerEvent {
                        stream_id: stream_id.clone(),
                        event_type: LedgerEventType::ConversationMessage,
                        actor: LedgerActor::User,
                        causation_event_id: None,
                        payload: json!({ "role": "user" }),
                    },
                    vec![NewArtifact::utf8(
                        "content",
                        "text/plain; charset=utf-8",
                        "build a news reader",
                    )],
                )
                .await
                .unwrap();
            ledger
                .append(
                    NewLedgerEvent {
                        stream_id: stream_id.clone(),
                        event_type: LedgerEventType::ModelRequestStarted,
                        actor: LedgerActor::AgentRuntime,
                        causation_event_id: None,
                        payload: json!({ "provider": "openai-compatible" }),
                    },
                    vec![NewArtifact::utf8(
                        "request",
                        "application/json",
                        "{\"model\":\"test\"}",
                    )],
                )
                .await
                .unwrap();

            let events = ledger.read_stream(&stream_id).await.unwrap();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].sequence, 1);
            assert_eq!(events[1].sequence, 2);
            assert_eq!(
                events[1].previous_event_hash.as_deref(),
                Some(events[0].event_hash.as_str())
            );
            assert_eq!(events[0].artifacts[0].byte_length, 19);
            assert_eq!(
                ledger
                    .read_artifact(&events[0].artifacts[0].hash)
                    .await
                    .unwrap(),
                b"build a news reader"
            );
            ledger.verify_stream(&stream_id).await.unwrap();
        });
    }

    #[test]
    fn rejects_events_for_unknown_streams() {
        futures_lite::future::block_on(async {
            let ledger = InMemoryAgentLedger::default();
            let error = ledger
                .append(
                    NewLedgerEvent {
                        stream_id: LedgerStreamId("missing".into()),
                        event_type: LedgerEventType::ConversationMessage,
                        actor: LedgerActor::User,
                        causation_event_id: None,
                        payload: json!({}),
                    },
                    Vec::new(),
                )
                .await
                .unwrap_err();
            assert_eq!(error, LedgerError::StreamNotFound);
        });
    }

    #[test]
    fn sqlite_ledger_survives_reopen_with_exact_artifact_bytes() {
        futures_lite::future::block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("agent-ledger.sqlite3");
            let stream_id = {
                let ledger = SqliteAgentLedger::open(&path).unwrap();
                let stream_id = ledger.create_stream("plan-surface-desktop").await.unwrap();
                ledger
                    .append(
                        NewLedgerEvent {
                            stream_id: stream_id.clone(),
                            event_type: LedgerEventType::ConversationMessage,
                            actor: LedgerActor::User,
                            causation_event_id: None,
                            payload: json!({ "role": "user" }),
                        },
                        vec![NewArtifact::utf8(
                            "content",
                            "text/plain; charset=utf-8",
                            "规划财经新闻应用",
                        )],
                    )
                    .await
                    .unwrap();
                stream_id
            };

            let reopened = SqliteAgentLedger::open(&path).unwrap();
            let events = reopened.read_stream(&stream_id).await.unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].sequence, 1);
            assert_eq!(
                reopened
                    .read_artifact(&events[0].artifacts[0].hash)
                    .await
                    .unwrap(),
                "规划财经新闻应用".as_bytes()
            );
            reopened.verify_stream(&stream_id).await.unwrap();
        });
    }
}
