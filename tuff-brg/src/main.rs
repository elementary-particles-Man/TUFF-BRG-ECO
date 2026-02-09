use async_trait::async_trait;
use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use chrono::Utc;
use dotenv::dotenv;
use futures_util::StreamExt;
use tokio::sync::{mpsc, watch};
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::signal::unix::{signal as unix_signal, SignalKind};
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use transformer_neo::db::{OpKind, TuffDb, TuffEngine};
use transformer_neo::lightweight::{
    LightweightCheckStatus, LightweightVerifier, MeaningDb, MeaningMatchMode,
};
use transformer_neo::models::{AgentIdentity, Id, IsoDateTime, ManualOverride, VerificationStatus};
use transformer_neo::pipeline::{
    AbstractGenerator, ClaimVerifier, DummyAbstractGenerator, DummySplitter, DummyVerifier,
    IngestPipeline, LlmAbstractor, LlmGapResolver, LlmVerifier,
    WebFetcher,
};

mod api;
use api::message::{
    ApproveFactPayload, ControlCommand, ControlCommandPayload, ControlTrigger, JudgeResultPayload,
    Message, ProposeFactPayload, StreamFragmentPayload, VerificationStatus as ProtoStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingFact {
    id: String,
    tag: String,
    value: String,
    created_at: String,
}

enum Verifier {
    Dummy(DummyVerifier),
    Llm(LlmVerifier),
}

#[async_trait]
impl ClaimVerifier for Verifier {
    async fn verify(
        &self,
        fragment: &str,
        facts: &[transformer_neo::models::RequiredFact],
    ) -> anyhow::Result<transformer_neo::pipeline::traits::VerificationResult> {
        match self {
            Verifier::Dummy(v) => v.verify(fragment, facts).await,
            Verifier::Llm(v) => v.verify(fragment, facts).await,
        }
    }
}

enum Abstractor {
    Dummy(DummyAbstractGenerator),
    Llm(LlmAbstractor),
}

#[async_trait]
impl AbstractGenerator for Abstractor {
    async fn generate(
        &self,
        fragment: &str,
        facts: &[transformer_neo::models::RequiredFact],
        status: VerificationStatus,
    ) -> anyhow::Result<transformer_neo::models::Abstract> {
        match self {
            Abstractor::Dummy(a) => a.generate(fragment, facts, status).await,
            Abstractor::Llm(a) => a.generate(fragment, facts, status).await,
        }
    }
}

fn valid_api_key(key: &str) -> bool {
    let trimmed = key.trim();
    !trimmed.is_empty() && !trimmed.contains("...")
}

fn to_proto_status(status: VerificationStatus) -> ProtoStatus {
    match status {
        VerificationStatus::Smoke => ProtoStatus::Smoke,
        VerificationStatus::GrayBlack => ProtoStatus::GrayBlack,
        VerificationStatus::GrayMid => ProtoStatus::GrayMid,
        VerificationStatus::GrayWhite => ProtoStatus::GrayWhite,
        VerificationStatus::White => ProtoStatus::White,
    }
}

#[derive(Clone)]
struct AppState {
    pipeline: Arc<
        IngestPipeline<
            DummySplitter,
            WebFetcher,
            Verifier,
            Abstractor,
            TuffEngine,
        >,
    >,
    lightweight_verifier: Option<Arc<RwLock<LightweightVerifier>>>,
    gap_resolver: Option<Arc<LlmGapResolver>>,
    stop_threshold: f32,
    history_dir: PathBuf,
    history_html: Arc<String>,
    pending_path: PathBuf,
    meaning_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let _identity = AgentIdentity::current();
    log_line("TUFF-BRG boot: main() start");

    let wal_dir = PathBuf::from("_tuffdb");
    fs::create_dir_all(&wal_dir)?;
    let wal_path = wal_dir.join("tuff.wal");

    let engine = TuffEngine::new(
        wal_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid wal path"))?,
    ).await?;

    let api_key = env::var("OPENAI_API_KEY").ok();
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

    let verifier = match api_key.as_deref() {
        Some(key) if valid_api_key(key) => Verifier::Llm(LlmVerifier::new(key, &model)),
        _ => Verifier::Dummy(DummyVerifier),
    };

    let abstractor = match api_key.as_deref() {
        Some(key) if valid_api_key(key) => Abstractor::Llm(LlmAbstractor::new(key, &model)),
        _ => Abstractor::Dummy(DummyAbstractGenerator),
    };

    let gap_resolver = match api_key.as_deref() {
        Some(key) if valid_api_key(key) => Some(Arc::new(LlmGapResolver::new(key, &model))),
        _ => None,
    };

    let pipeline = IngestPipeline {
        splitter: DummySplitter,
        fetcher: WebFetcher::new(),
        verifier,
        generator: abstractor,
        db: engine,
    };

    let lightweight_verifier = init_lightweight_verifier(&wal_dir);

    let stop_threshold = env::var("TUFF_STOP_CONFIDENCE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.35);

    let history_dir = PathBuf::from(env::var("TUFF_HISTORY_OUT").unwrap_or_else(|_| "history_out".to_string()));
    let history_html = include_str!("../assets/history_viewer.html").to_string();
    let meaning_path = env::var("TUFF_LIGHTWEIGHT_MEANING_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| wal_dir.join("lightweight").join("meaning.db"));
    let pending_path = env::var("TUFF_PENDING_FACT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| wal_dir.join("lightweight").join("meaning.pending"));

    let state = AppState {
        pipeline: Arc::new(pipeline),
        lightweight_verifier,
        gap_resolver,
        stop_threshold,
        history_dir,
        history_html: Arc::new(history_html),
        pending_path,
        meaning_path,
    };

    let app = Router::new()
        .route("/", get(ws_handler))
        .route("/history", get(history_page))
        .route("/history/api/latest", get(history_latest))
        .route("/history/api/timeline", get(history_timeline))
        .route("/facts/pending", get(facts_pending))
        .with_state(state);

    let addr: SocketAddr = "127.0.0.1:8787".parse()?;
    let listener = TcpListener::bind(addr).await?;
    log_line(&format!("TUFF-BRG listening on {}", addr));
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn parse_meaning_env_pairs() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(raw) = env::var("TUFF_MEANING_DB") {
        for item in raw.split(';') {
            let mut parts = item.splitn(2, '=');
            if let (Some(tag), Some(meaning)) = (parts.next(), parts.next()) {
                if !tag.trim().is_empty() && !meaning.trim().is_empty() {
                    map.insert(tag.trim().to_string(), meaning.trim().to_string());
                }
            }
        }
    }
    map
}

fn init_lightweight_verifier(wal_dir: &PathBuf) -> Option<Arc<RwLock<LightweightVerifier>>> {
    let enabled = env::var("TUFF_FAST_PATH")
        .map(|v| v.trim() != "0")
        .unwrap_or(true);
    if !enabled {
        return None;
    }

    let default_path = wal_dir.join("lightweight").join("meaning.db");
    let path = env::var("TUFF_LIGHTWEIGHT_MEANING_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or(default_path);

    let mut merged = if path.exists() {
        MeaningDb::from_path(&path).unwrap_or_else(|_| MeaningDb::new(std::collections::HashMap::new()))
    } else {
        MeaningDb::new(std::collections::HashMap::new())
    };
    merged.merge(parse_meaning_env_pairs());
    let verifier = LightweightVerifier::new(merged);
    Some(Arc::new(RwLock::new(verifier)))
}

async fn shutdown_signal() {
    let mut sigint = unix_signal(SignalKind::interrupt()).ok();
    let mut sigterm = unix_signal(SignalKind::terminate()).ok();

    tokio::select! {
        _ = async {
            if let Err(err) = signal::ctrl_c().await {
                eprintln!("Failed to listen for shutdown signal: {}", err);
            }
        } => {
            log_line("SIGINT received. Shutting down...");
        }
        _ = async {
            if let Some(sig) = sigint.as_mut() { sig.recv().await; }
        } => {
            log_line("SIGINT (unix) received. Shutting down...");
        }
        _ = async {
            if let Some(sig) = sigterm.as_mut() { sig.recv().await; }
        } => {
            log_line("SIGTERM received. Shutting down...");
        }
    }

    // force exit if runtime is wedged
    std::process::exit(0);
}

fn log_line(msg: &str) {
    println!("{}", msg);
    let _ = io::stdout().flush();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    log_line("WS: client connected");
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::channel::<WsMessage>(256);
    let (frag_tx, mut frag_rx) = watch::channel(String::new());

    // outbound pump
    let tx_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // ingest worker (decouple from WS receive loop)
    let state_for_worker = state.clone();
    let tx_for_worker = tx.clone();
    let ingest_task = tokio::spawn(async move {
        while frag_rx.changed().await.is_ok() {
            let fragment = frag_rx.borrow().clone();
            if fragment.is_empty() {
                continue;
            }
            log_line("INGEST: start");

            if let Some(lightweight) = state_for_worker.lightweight_verifier.as_ref() {
                let lw = lightweight.read().await;
                match lw.check_fragment(&fragment) {
                    LightweightCheckStatus::Hit => {
                        if let Some(hit) = lw.verify_fragment(&fragment) {
                            let mode = match hit.mode {
                                MeaningMatchMode::Exact => "exact",
                                MeaningMatchMode::Contains => "contains",
                            };
                            let judge = Message::JudgeResult {
                                id: Id::new().to_string(),
                                ts: Utc::now().to_rfc3339(),
                                payload: JudgeResultPayload {
                                    status: ProtoStatus::White,
                                    reason: format!("source=Cache tag={} mode={}", hit.tag, mode),
                                    confidence: 1.0,
                                    claim: fragment.clone(),
                                    evidence_count: 0,
                                    abstract_id: None,
                                },
                            };
                            let _ = tx_for_worker
                                .send(WsMessage::Text(serde_json::to_string(&judge).unwrap_or_default()))
                                .await;
                            log_line("INGEST: cache hit");
                            continue;
                        }
                    }
                    LightweightCheckStatus::Mismatch => {
                        let judge = Message::JudgeResult {
                            id: Id::new().to_string(),
                            ts: Utc::now().to_rfc3339(),
                            payload: JudgeResultPayload {
                                status: ProtoStatus::Smoke,
                                reason: "source=Cache mismatch".to_string(),
                                confidence: 0.0,
                                claim: fragment.clone(),
                                evidence_count: 0,
                                abstract_id: None,
                            },
                        };
                        let _ = tx_for_worker
                            .send(WsMessage::Text(serde_json::to_string(&judge).unwrap_or_default()))
                            .await;
                        let stop = Message::ControlCommand {
                            id: "system".to_string(),
                            ts: Utc::now().to_rfc3339(),
                            payload: ControlCommandPayload {
                                command: ControlCommand::Stop,
                                trigger: ControlTrigger::SmokeDetected,
                                detail: "FastPath mismatch detected".to_string(),
                                manual_override: None,
                            },
                        };
                        let _ = tx_for_worker
                            .send(WsMessage::Text(serde_json::to_string(&stop).unwrap_or_default()))
                            .await;
                        log_line("INGEST: cache mismatch -> stop");
                        continue;
                    }
                    LightweightCheckStatus::Unknown => {}
                }
            }

            let ingest_result = timeout(
                Duration::from_secs(3),
                state_for_worker.pipeline.ingest(&fragment),
            )
            .await;

            let ops = match ingest_result {
                Ok(Ok(v)) => v,
                Ok(Err(_)) => {
                    log_line("INGEST: error");
                    continue;
                }
                Err(_) => {
                    log_line("INGEST: timeout");
                    continue;
                }
            };
            log_line("INGEST: end");

            let mut status = VerificationStatus::GrayMid;
            let mut confidence = 0.4_f32;
            let mut evidence_count = 0usize;
            let mut reason = "ok".to_string();
            let mut abstract_id: Option<String> = None;
            if let Some(outcome) = ops.first() {
                status = outcome.status;
                confidence = outcome.confidence;
                evidence_count = outcome.evidence_count;
                reason = format!("source=LLM {}", outcome.reason);
                if let OpKind::InsertAbstract { abstract_ } = &outcome.op.kind {
                    abstract_id = Some(abstract_.id.to_string());
                }
            } else {
                reason = "source=LLM ok".to_string();
            }

            let judge = Message::JudgeResult {
                id: Id::new().to_string(),
                ts: Utc::now().to_rfc3339(),
                payload: JudgeResultPayload {
                    status: to_proto_status(status),
                    reason,
                    confidence,
                    claim: fragment.clone(),
                    evidence_count: evidence_count as u32,
                    abstract_id,
                },
            };
            let _ = tx_for_worker
                .send(WsMessage::Text(serde_json::to_string(&judge).unwrap_or_default()))
                .await;
            log_line("WS: JudgeResult sent");

            if status == VerificationStatus::Smoke || confidence < state_for_worker.stop_threshold {
                let trigger = if status == VerificationStatus::Smoke {
                    ControlTrigger::SmokeDetected
                } else {
                    ControlTrigger::LowConfidence
                };
                let stop = Message::ControlCommand {
                    id: "system".to_string(),
                    ts: Utc::now().to_rfc3339(),
                    payload: ControlCommandPayload {
                        command: ControlCommand::Stop,
                        trigger,
                        detail: format!("status={:?} confidence={:.3}", status, confidence),
                        manual_override: None,
                    },
                };
                let _ = tx_for_worker
                    .send(WsMessage::Text(serde_json::to_string(&stop).unwrap_or_default()))
                    .await;
                log_line("WS: ControlCommand STOP sent");
            }
        }
    });

    // inbound loop
    while let Some(msg) = ws_rx.next().await {
        let Ok(msg) = msg else { break };
        let text = match msg {
            WsMessage::Text(t) => t,
            _ => continue,
        };
        log_line("WS: received text frame");
        let parsed: Message = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                log_line("WS: JSON parse error");
                let stop = Message::ControlCommand {
                    id: "system".to_string(),
                    ts: Utc::now().to_rfc3339(),
                    payload: ControlCommandPayload {
                        command: ControlCommand::Stop,
                        trigger: ControlTrigger::ManualOverride,
                        detail: "JSON parse error".to_string(),
                        manual_override: None,
                    },
                };
                let _ = tx
                    .send(WsMessage::Text(serde_json::to_string(&stop).unwrap_or_default()))
                    .await;
                continue;
            }
        };

        // handle control command inline
        if let Message::ControlCommand { payload, .. } = parsed {
            if payload.command == ControlCommand::Continue
                && payload.trigger == ControlTrigger::ManualOverride
            {
                let meta = payload.manual_override;
                let note = meta
                    .as_ref()
                    .and_then(|m| m.note.clone())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "No reason provided".to_string());
                let conversation_id = meta.as_ref().and_then(|m| m.conversation_id.clone());
                let abstract_id = meta
                    .as_ref()
                    .and_then(|m| m.abstract_id.as_ref())
                    .and_then(|s| s.parse::<Id>().ok());
                let override_ = ManualOverride {
                    override_id: Id::new(),
                    observed_at: IsoDateTime::now(),
                    agent: AgentIdentity::current(),
                    conversation_id,
                    abstract_id,
                    note: Some(note),
                };
                let _ = state.pipeline.db.append_override(override_).await;
            }
            continue;
        }

        if let Message::ProposeFact { payload, .. } = parsed {
            let _ = handle_proposal(&state, payload).await;
            continue;
        }

        if let Message::ApproveFact { payload, .. } = parsed {
            let _ = handle_approve(&state, payload).await;
            continue;
        }

        if let Message::StreamFragment { payload, .. } = parsed {
            log_line("WS: StreamFragment received");
            let StreamFragmentPayload { fragment, .. } = payload;
            let _ = frag_tx.send(fragment);
        }
    }

    tx_task.abort();
    ingest_task.abort();
}

async fn history_page(State(state): State<AppState>) -> Html<String> {
    Html(state.history_html.as_ref().to_string())
}

async fn history_latest(State(state): State<AppState>) -> Response {
    let mut value = read_json_or_default(
        &state.history_dir.join("latest_facts.json"),
        json!({
            "last_updated": Utc::now().to_rfc3339(),
            "facts": []
        }),
    );
    merge_lightweight_latest(&mut value, &lightweight_wal_path());
    (StatusCode::OK, value.to_string()).into_response()
}

async fn history_timeline(State(state): State<AppState>) -> Response {
    let mut value = read_json_or_default(&state.history_dir.join("timeline.json"), json!([]));
    merge_lightweight_timeline(&mut value, &lightweight_wal_path());
    (StatusCode::OK, value.to_string()).into_response()
}

async fn facts_pending(State(state): State<AppState>) -> Response {
    let items = load_pending_facts(&state.pending_path).await;
    (
        StatusCode::OK,
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string()),
    )
        .into_response()
}

fn read_json_or_default(path: &Path, default_value: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or(default_value),
        Err(_) => default_value,
    }
}

#[derive(Debug, Clone)]
struct LightweightWalRecord {
    tag: String,
    meaning: String,
}

fn lightweight_wal_path() -> PathBuf {
    env::var("TUFF_LIGHTWEIGHT_WAL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("tuff-db-lightweight.wal"))
}

fn load_lightweight_records(path: &Path) -> Vec<LightweightWalRecord> {
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let tag = parts.next()?.trim();
            let meaning = parts.next()?.trim();
            let _sha = parts.next()?.trim();
            if tag.is_empty() || meaning.is_empty() {
                return None;
            }
            Some(LightweightWalRecord {
                tag: tag.to_string(),
                meaning: meaning.to_string(),
            })
        })
        .collect()
}

fn merge_lightweight_latest(root: &mut Value, wal_path: &Path) {
    let records = load_lightweight_records(wal_path);
    if records.is_empty() {
        return;
    }
    if root.get("last_updated").is_none() {
        root["last_updated"] = json!(Utc::now().to_rfc3339());
    }
    let facts = root["facts"].as_array_mut();
    let Some(facts) = facts else { return };

    for (idx, rec) in records.into_iter().enumerate() {
        facts.push(json!({
            "topic_id": format!("lw:{}", rec.tag),
            "subject": "LIGHTWEIGHT",
            "current_value": rec.meaning,
            "status": "VERIFIED",
            "confidence": 1.0,
            "confidence_kind": "FAST_PATH",
            "agent_origin": "LIGHTWEIGHT_DB",
            "source_op_id": format!("lw_{idx:08}"),
            "last_event_ts": Utc::now().to_rfc3339(),
            "is_human_overridden": false
        }));
    }
}

fn merge_lightweight_timeline(root: &mut Value, wal_path: &Path) {
    let records = load_lightweight_records(wal_path);
    if records.is_empty() {
        return;
    }
    let timelines = root.as_array_mut();
    let Some(timelines) = timelines else { return };

    for (idx, rec) in records.into_iter().enumerate() {
        timelines.push(json!({
            "topic_id": format!("lw:{}", rec.tag),
            "events": [{
                "op_id": format!("lw_{idx:08}"),
                "timestamp": Utc::now().to_rfc3339(),
                "type": "INGEST",
                "agent_origin": "LIGHTWEIGHT_DB",
                "status_after": "VERIFIED",
                "reason": rec.meaning
            }]
        }));
    }
}

async fn handle_proposal(state: &AppState, payload: ProposeFactPayload) -> anyhow::Result<()> {
    let tag = payload.tag.trim().to_string();
    let value = payload.value.trim().to_string();
    if tag.is_empty() || value.is_empty() {
        return Ok(());
    }
    let item = PendingFact {
        id: Id::new().to_string(),
        tag,
        value,
        created_at: Utc::now().to_rfc3339(),
    };
    append_pending_fact(&state.pending_path, &item).await?;
    Ok(())
}

async fn handle_approve(state: &AppState, payload: ApproveFactPayload) -> anyhow::Result<()> {
    let mut items = load_pending_facts(&state.pending_path).await;
    if let Some(pos) = items.iter().position(|v| v.id == payload.id) {
        let item = items.remove(pos);
        save_pending_facts(&state.pending_path, &items).await?;

        if let Some(lw) = state.lightweight_verifier.as_ref() {
            let mut guard = lw.write().await;
            let _ = guard.insert_meaning(&state.meaning_path, &item.tag, &item.value);
            let _ = guard.reload(&state.meaning_path);
        } else {
            append_meaning_line(&state.meaning_path, &item.tag, &item.value).await?;
        }
    }
    Ok(())
}

async fn append_pending_fact(path: &Path, item: &PendingFact) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let line = serde_json::to_string(item)? + "\n";
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(line.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

async fn load_pending_facts(path: &Path) -> Vec<PendingFact> {
    let text = match tokio::fs::read_to_string(path).await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<PendingFact>(line).ok())
        .collect()
}

async fn save_pending_facts(path: &Path, items: &[PendingFact]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut out = String::new();
    for item in items {
        out.push_str(&serde_json::to_string(item)?);
        out.push('\n');
    }
    tokio::fs::write(path, out).await?;
    Ok(())
}

async fn append_meaning_line(path: &Path, tag: &str, value: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(format!("{}={}\n", tag, value).as_bytes()).await?;
    file.flush().await?;
    Ok(())
}
