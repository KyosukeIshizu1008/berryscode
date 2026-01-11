//! BerryFlow - n8n-style Agent Pipeline Engine
//!
//! ノードをつなげて、結果を次の工程に渡し、条件分岐でループさせる仕組み

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::berrycode::llm::{LLMClient, Message};
use crate::berrycode::conversation_engine::{ConversationEngine, ToolCallback};
use crate::berrycode::models::Model;
use crate::berrycode::berryflow_config::BerryFlowConfig;
use crate::berrycode::notifications::Notifier;

/// ワークフロー進捗メッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgressMessage {
    pub node_id: String,
    pub node_name: String,
    pub status: String, // "running", "success", "failure"
    pub message: String,
    pub loop_count: usize,
}

/// 進捗送信用のチャネル
pub type ProgressSender = tokio::sync::mpsc::UnboundedSender<WorkflowProgressMessage>;

/// スナップショット保存用のコールバック
pub type SnapshotSaver = Arc<
    dyn Fn(String, String, String, String, String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

/// 一時停止フラグマップ
pub type PauseFlags = Arc<Mutex<std::collections::HashMap<String, bool>>>;

/// 再開シグナルマップ
pub type ResumeSignals = Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<()>>>>;

/// キャンセルフラグマップ
pub type CancelFlags = Arc<Mutex<std::collections::HashMap<String, bool>>>;

/// ノードのアクション種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeAction {
    /// 設計フェーズ
    Design,
    /// 実装フェーズ
    Implement,
    /// テスト実行
    Test,
    /// エラー修正
    Fix,
    /// リファクタリング
    Refactor,
    /// カスタムプロンプト実行
    Custom { prompt: String },
    /// HTTP APIコール
    HttpRequest {
        url: String,
        method: String,
        headers: Option<std::collections::HashMap<String, String>>,
        body: Option<String>,
    },
    /// データ変換（JavaScript式を使用）
    DataTransform {
        script: String,
    },
    /// ファイル操作（読み込み/書き込み）
    FileOperation {
        operation: String, // "read" or "write"
        file_path: String,
        content: Option<String>,
    },
    /// スクリプト実行（任意のシェルコマンド）
    ScriptExecution {
        language: String, // "bash", "python", "node", etc.
        script: String,
    },
}

/// 条件チェック種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConditionCheck {
    /// カバレッジが閾値を下回る
    CoverageLessThan { threshold: f64 },
    /// カバレッジが閾値以上
    CoverageGreaterOrEqual { threshold: f64 },
    /// 実行時間が閾値を超える
    ExecutionTimeGreaterThan { threshold_ms: u64 },
    /// テスト失敗数が閾値を超える
    FailedTestsGreaterThan { threshold: usize },
    /// カスタム条件（シェルコマンドの終了コードで判定）
    CustomCommand { command: String, expect_success: bool },
    /// 複数条件のAND結合（すべて真の場合に真）
    And { conditions: Vec<ConditionCheck> },
    /// 複数条件のOR結合（いずれか真の場合に真）
    Or { conditions: Vec<ConditionCheck> },
    /// 条件の否定
    Not { condition: Box<ConditionCheck> },
}

/// 条件付き遷移
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalTransition {
    /// 条件
    pub condition: ConditionCheck,
    /// 条件を満たした場合の次のノードID
    pub next_node_id: String,
}

/// ワークフローの1つのノード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    /// ノードID
    pub id: String,
    /// ノード名（表示用）
    pub name: String,
    /// 実行するアクション
    pub action: NodeAction,
    /// 成功時の次のノードID
    pub next_on_success: Option<String>,
    /// 失敗時の次のノードID（これがループを作る）
    pub next_on_failure: Option<String>,
    /// 条件付き遷移（成功時に追加でチェック）
    pub conditional_transitions: Option<Vec<ConditionalTransition>>,
    /// 並列実行するノードIDリスト（このノードと並行して実行）
    pub parallel_nodes: Option<Vec<String>>,
}

/// パイプライン定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// パイプラインID
    pub id: String,
    /// パイプライン名
    pub name: String,
    /// ノード一覧（ID -> Node）
    pub nodes: HashMap<String, FlowNode>,
    /// 開始ノードID
    pub start_node_id: String,
    /// 最大ループ回数（無限ループ防止）
    pub max_loops: usize,
    /// タイムアウト（秒）None = 無制限
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// パイプライン実行コンテキスト拡張
pub struct PipelineExecutor {
    pub llm_client: Option<LLMClient>,
    pub model: Option<Model>,
}

/// パイプライン状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    Running,
    Paused { at_node: String },
    Completed,
    Failed { error: String },
}

/// パイプライン実行コンテキスト
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// 現在のノードID
    pub current_node_id: String,
    /// 前のノードからの出力（設計書、コード、エラーログなど）
    pub context_data: String,
    /// ループカウンター
    pub loop_count: usize,
    /// 実行履歴
    pub execution_history: Vec<ExecutionRecord>,
    /// パイプライン状態
    pub state: PipelineState,
}

/// 実行履歴の1レコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub node_id: String,
    pub node_name: String,
    pub action: NodeAction,
    pub success: bool,
    pub output: String,
    pub timestamp: String,
    /// ノード実行時間（ミリ秒）
    pub duration_ms: u64,
    /// 実行時のメトリクス
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ExecutionMetrics>,
}

/// 実行メトリクス
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// コードカバレッジ（0.0 - 100.0）
    pub coverage: Option<f64>,
    /// 実行時間（ミリ秒）
    pub execution_time_ms: Option<u64>,
    /// 失敗したテスト数
    pub failed_tests: Option<usize>,
    /// 合計テスト数
    pub total_tests: Option<usize>,
}

/// ノード実行結果
#[derive(Debug)]
pub enum NodeResult {
    Success(String, ExecutionMetrics),
    Failure(String),
}

/// ファイルスナップショット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub file_path: String,
    pub content: String,
    pub timestamp: String,
}

/// ワークフロースナップショット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub snapshot_id: String,
    pub execution_id: String,
    pub node_id: String,
    pub node_name: String,
    pub files: Vec<FileSnapshot>,
    pub timestamp: String,
}

impl Pipeline {
    /// 新しいパイプラインを作成
    pub fn new(id: String, name: String, start_node_id: String) -> Self {
        Self {
            id,
            name,
            nodes: HashMap::new(),
            start_node_id,
            max_loops: 5,
            timeout_seconds: None,
        }
    }

    /// ノードを追加
    pub fn add_node(&mut self, node: FlowNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// 条件を評価
    fn evaluate_condition<'a>(
        &'a self,
        condition: &'a ConditionCheck,
        metrics: &'a ExecutionMetrics,
        project_root: &'a PathBuf,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
        match condition {
            ConditionCheck::CoverageLessThan { threshold } => {
                if let Some(coverage) = metrics.coverage {
                    Ok(coverage < *threshold)
                } else {
                    Ok(false) // メトリクスがない場合は条件を満たさない
                }
            }
            ConditionCheck::CoverageGreaterOrEqual { threshold } => {
                if let Some(coverage) = metrics.coverage {
                    Ok(coverage >= *threshold)
                } else {
                    Ok(false)
                }
            }
            ConditionCheck::ExecutionTimeGreaterThan { threshold_ms } => {
                if let Some(exec_time) = metrics.execution_time_ms {
                    Ok(exec_time > *threshold_ms)
                } else {
                    Ok(false)
                }
            }
            ConditionCheck::FailedTestsGreaterThan { threshold } => {
                if let Some(failed) = metrics.failed_tests {
                    Ok(failed > *threshold)
                } else {
                    Ok(false)
                }
            }
            ConditionCheck::CustomCommand { command, expect_success } => {
                // カスタムコマンドを実行
                let output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(project_root)
                    .output()
                    .await?;

                Ok(output.status.success() == *expect_success)
            }
            ConditionCheck::And { conditions } => {
                // すべての条件が真の場合のみ真
                for condition in conditions {
                    if !self.evaluate_condition(condition, metrics, project_root).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConditionCheck::Or { conditions } => {
                // いずれかの条件が真の場合に真
                for condition in conditions {
                    if self.evaluate_condition(condition, metrics, project_root).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ConditionCheck::Not { condition } => {
                // 条件の否定
                let result = self.evaluate_condition(condition, metrics, project_root).await?;
                Ok(!result)
            }
        }
        })
    }

    /// パイプラインを実行
    pub async fn run(
        &self,
        project_root: &PathBuf,
        initial_prompt: String,
        progress_tx: Option<ProgressSender>,
        execution_id: Option<String>,
        snapshot_saver: Option<SnapshotSaver>,
        pause_flags: Option<PauseFlags>,
        resume_signals: Option<ResumeSignals>,
        cancel_flags: Option<CancelFlags>,
    ) -> Result<PipelineContext> {
        info!("Starting pipeline execution: {}", self.name);

        // タイムアウトが設定されている場合は、タイムアウト付きで実行
        if let Some(timeout_secs) = self.timeout_seconds {
            let timeout_duration = std::time::Duration::from_secs(timeout_secs);
            match tokio::time::timeout(
                timeout_duration,
                self.run_internal(
                    project_root,
                    initial_prompt,
                    progress_tx,
                    execution_id,
                    snapshot_saver,
                    pause_flags,
                    resume_signals,
                    cancel_flags,
                ),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(anyhow!(
                    "ワークフローがタイムアウトしました（{}秒）",
                    timeout_secs
                )),
            }
        } else {
            // タイムアウトなしで実行
            self.run_internal(
                project_root,
                initial_prompt,
                progress_tx,
                execution_id,
                snapshot_saver,
                pause_flags,
                resume_signals,
                cancel_flags,
            )
            .await
        }
    }

    /// パイプラインの内部実行ロジック
    async fn run_internal(
        &self,
        project_root: &PathBuf,
        initial_prompt: String,
        progress_tx: Option<ProgressSender>,
        execution_id: Option<String>,
        snapshot_saver: Option<SnapshotSaver>,
        pause_flags: Option<PauseFlags>,
        resume_signals: Option<ResumeSignals>,
        cancel_flags: Option<CancelFlags>,
    ) -> Result<PipelineContext> {
        let mut context = PipelineContext {
            current_node_id: self.start_node_id.clone(),
            context_data: initial_prompt,
            loop_count: 0,
            execution_history: Vec::new(),
            state: PipelineState::Running,
        };

        while let Some(node) = self.nodes.get(&context.current_node_id) {
            info!("Executing node: {} ({})", node.name, node.id);

            // 一時停止チェック（execution_idがある場合のみ）
            if let (Some(ref exec_id), Some(ref flags)) = (&execution_id, &pause_flags) {
                let mut paused_message_sent = false;

                loop {
                    let is_paused = {
                        let pause_map = flags.lock().await;
                        pause_map.get(exec_id).copied().unwrap_or(false)
                    };

                    if is_paused {
                        if !paused_message_sent {
                            info!("Pipeline {} is paused, waiting for resume...", exec_id);

                            // 進捗送信: 一時停止中
                            if let Some(ref tx) = progress_tx {
                                let _ = tx.send(WorkflowProgressMessage {
                                    node_id: node.id.clone(),
                                    node_name: node.name.clone(),
                                    status: "paused".to_string(),
                                    message: format!("{}で一時停止中...", node.name),
                                    loop_count: context.loop_count,
                                });
                            }

                            paused_message_sent = true;
                        }

                        // 1秒待ってから再チェック
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    } else {
                        if paused_message_sent {
                            info!("Pipeline {} resumed", exec_id);
                        }
                        break;
                    }
                }
            }

            // キャンセルチェック（execution_idがある場合のみ）
            if let (Some(ref exec_id), Some(ref flags)) = (&execution_id, &cancel_flags) {
                let is_cancelled = {
                    let cancel_map = flags.lock().await;
                    cancel_map.get(exec_id).copied().unwrap_or(false)
                };

                if is_cancelled {
                    warn!("Pipeline {} was cancelled", exec_id);

                    // 進捗送信: キャンセル
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(WorkflowProgressMessage {
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            status: "cancelled".to_string(),
                            message: format!("{}でキャンセルされました", node.name),
                            loop_count: context.loop_count,
                        });
                    }

                    return Err(anyhow!("ワークフローがキャンセルされました"));
                }
            }

            // 進捗送信: ノード開始
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(WorkflowProgressMessage {
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                    status: "running".to_string(),
                    message: format!("{}を実行中...", node.name),
                    loop_count: context.loop_count,
                });
            }

            // ループ回数チェック
            if context.loop_count >= self.max_loops {
                warn!("Max loop limit reached: {}", self.max_loops);

                // 進捗送信: 失敗
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(WorkflowProgressMessage {
                        node_id: node.id.clone(),
                        node_name: node.name.clone(),
                        status: "failure".to_string(),
                        message: format!("最大ループ回数({})に達しました", self.max_loops),
                        loop_count: context.loop_count,
                    });
                }

                return Err(anyhow!("最大ループ回数に達しました: {} 回", self.max_loops));
            }

            // ノードを実行（時間計測）
            let node_start = std::time::Instant::now();
            let result = self.execute_node(node, &context.context_data, project_root).await?;
            let node_duration_ms = node_start.elapsed().as_millis() as u64;

            // 並列ノードがあれば実行
            let mut parallel_results = Vec::new();
            if let Some(ref parallel_node_ids) = node.parallel_nodes {
                info!("Executing {} parallel nodes", parallel_node_ids.len());

                let mut parallel_tasks = Vec::new();

                for parallel_node_id in parallel_node_ids {
                    if let Some(parallel_node) = self.nodes.get(parallel_node_id) {
                        // 進捗送信: 並列ノード開始
                        if let Some(ref tx) = progress_tx {
                            let _ = tx.send(WorkflowProgressMessage {
                                node_id: parallel_node.id.clone(),
                                node_name: parallel_node.name.clone(),
                                status: "running".to_string(),
                                message: format!("{}を並列実行中...", parallel_node.name),
                                loop_count: context.loop_count,
                            });
                        }

                        let parallel_node_clone = parallel_node.clone();
                        let context_data = context.context_data.clone();
                        let project_root_clone = project_root.clone();

                        // 並列タスクを生成
                        let task = tokio::spawn(async move {
                            let start = std::time::Instant::now();
                            let result = Self::execute_node_static(&parallel_node_clone, &context_data, &project_root_clone).await;
                            let duration_ms = start.elapsed().as_millis() as u64;
                            (parallel_node_clone.id.clone(),
                             parallel_node_clone.name.clone(),
                             result,
                             duration_ms)
                        });

                        parallel_tasks.push(task);
                    } else {
                        warn!("Parallel node not found: {}", parallel_node_id);
                    }
                }

                // すべての並列タスクの完了を待つ
                for task in parallel_tasks {
                    match task.await {
                        Ok((node_id, node_name, task_result, duration_ms)) => {
                            match task_result {
                                Ok(NodeResult::Success(output, metrics)) => {
                                    info!("Parallel node {} completed successfully", node_id);

                                    // 進捗送信: 並列ノード成功
                                    if let Some(ref tx) = progress_tx {
                                        let _ = tx.send(WorkflowProgressMessage {
                                            node_id: node_id.clone(),
                                            node_name: node_name.clone(),
                                            status: "success".to_string(),
                                            message: format!("{}が正常に完了しました（並列）", node_name),
                                            loop_count: context.loop_count,
                                        });
                                    }

                                    // 並列ノードの結果を収集
                                    parallel_results.push(format!("【{}】\n{}", node_name, output));

                                    // 並列ノードの結果を履歴に記録
                                    context.execution_history.push(ExecutionRecord {
                                        node_id: node_id.clone(),
                                        node_name: node_name.clone(),
                                        action: NodeAction::Custom { prompt: "Parallel execution".to_string() },
                                        success: true,
                                        output,
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                        duration_ms,
                                        metrics: Some(metrics),
                                    });
                                }
                                Ok(NodeResult::Failure(error_log)) => {
                                    warn!("Parallel node {} failed: {}", node_id, error_log);

                                    // 進捗送信: 並列ノード失敗
                                    if let Some(ref tx) = progress_tx {
                                        let _ = tx.send(WorkflowProgressMessage {
                                            node_id: node_id.clone(),
                                            node_name: node_name.clone(),
                                            status: "failure".to_string(),
                                            message: format!("{}が失敗しました（並列）", node_name),
                                            loop_count: context.loop_count,
                                        });
                                    }

                                    // 並列ノードの失敗結果も収集
                                    parallel_results.push(format!("【{}（失敗）】\n{}", node_name, error_log));

                                    // 並列ノードの失敗を履歴に記録
                                    context.execution_history.push(ExecutionRecord {
                                        node_id: node_id.clone(),
                                        node_name: node_name.clone(),
                                        action: NodeAction::Custom { prompt: "Parallel execution".to_string() },
                                        success: false,
                                        output: error_log,
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                        duration_ms,
                                        metrics: None,
                                    });
                                }
                                Err(e) => {
                                    warn!("Parallel node {} execution error: {}", node_id, e);
                                    parallel_results.push(format!("【{}（エラー）】\n{}", node_name, e));
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Parallel task join error: {}", e);
                        }
                    }
                }
            }

            // 実行履歴に記録
            let record = ExecutionRecord {
                node_id: node.id.clone(),
                node_name: node.name.clone(),
                action: node.action.clone(),
                success: matches!(result, NodeResult::Success(_, _)),
                output: match &result {
                    NodeResult::Success(s, _) => s.clone(),
                    NodeResult::Failure(s) => s.clone(),
                },
                timestamp: chrono::Utc::now().to_rfc3339(),
                duration_ms: node_duration_ms,
                metrics: match &result {
                    NodeResult::Success(_, m) => Some(m.clone()),
                    NodeResult::Failure(_) => None,
                },
            };
            context.execution_history.push(record);

            // 結果に応じて次のノードへ遷移
            match result {
                NodeResult::Success(output, metrics) => {
                    // 進捗送信: 成功
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(WorkflowProgressMessage {
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            status: "success".to_string(),
                            message: format!("{}が正常に完了しました", node.name),
                            loop_count: context.loop_count,
                        });
                    }

                    context.context_data = output;

                    // 並列ノードの結果を統合
                    if !parallel_results.is_empty() {
                        context.context_data.push_str("\n\n--- 並列実行結果 ---\n");
                        context.context_data.push_str(&parallel_results.join("\n\n"));
                    }

                    // スナップショットを自動作成（execution_idとsnapshot_saverが両方ある場合のみ）
                    if let (Some(ref exec_id), Some(ref saver)) = (&execution_id, &snapshot_saver) {
                        info!("Creating snapshot for node {} in execution {}", node.id, exec_id);

                        // 追跡対象のファイルを取得
                        match Self::get_tracked_files(project_root).await {
                            Ok(tracked_files) => {
                                // スナップショットを作成
                                match Self::create_snapshot(
                                    exec_id,
                                    &node.id,
                                    &node.name,
                                    project_root,
                                    &tracked_files,
                                ).await {
                                    Ok(snapshot) => {
                                        // スナップショットをDBに保存
                                        let snapshot_data = serde_json::to_string(&snapshot).unwrap_or_default();
                                        let snapshot_id = snapshot.snapshot_id.clone();
                                        let node_id = node.id.clone();
                                        let node_name = node.name.clone();
                                        let exec_id_clone = exec_id.clone();

                                        match saver(
                                            snapshot_id.clone(),
                                            exec_id_clone,
                                            node_id,
                                            node_name,
                                            snapshot_data,
                                        ).await {
                                            Ok(_) => {
                                                info!("Snapshot {} saved successfully with {} files",
                                                      snapshot_id, snapshot.files.len());
                                            }
                                            Err(e) => {
                                                warn!("Failed to save snapshot to database: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to create snapshot: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to get tracked files for snapshot: {}", e);
                            }
                        }
                    }

                    // 条件付き遷移をチェック
                    let mut next_node_determined = false;
                    if let Some(ref transitions) = node.conditional_transitions {
                        for transition in transitions {
                            if self.evaluate_condition(&transition.condition, &metrics, project_root).await? {
                                info!(
                                    "Conditional transition triggered: {:?} -> {}",
                                    transition.condition, transition.next_node_id
                                );
                                context.current_node_id = transition.next_node_id.clone();
                                next_node_determined = true;
                                break;
                            }
                        }
                    }

                    // 条件付き遷移がなければ通常の成功時遷移
                    if !next_node_determined {
                        if let Some(next_id) = &node.next_on_success {
                            context.current_node_id = next_id.clone();
                        } else {
                            info!("Pipeline completed successfully");
                            break;
                        }
                    }
                }
                NodeResult::Failure(error_log) => {
                    // 進捗送信: 失敗
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(WorkflowProgressMessage {
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            status: "failure".to_string(),
                            message: format!("{}が失敗しました", node.name),
                            loop_count: context.loop_count,
                        });
                    }

                    context.context_data = error_log.clone();
                    if let Some(fix_node_id) = &node.next_on_failure {
                        warn!("Node failed, moving to fix node: {}", fix_node_id);
                        context.current_node_id = fix_node_id.clone();
                        context.loop_count += 1;
                    } else {
                        return Err(anyhow!("パイプラインが失敗しました"));
                    }
                }
            }
        }

        Ok(context)
    }

    /// 個別のノードを実行
    async fn execute_node(
        &self,
        node: &FlowNode,
        input_context: &str,
        project_root: &PathBuf,
    ) -> Result<NodeResult> {
        Self::execute_node_static(node, input_context, project_root).await
    }

    /// 個別のノードを実行（静的バージョン - 並列実行用）
    async fn execute_node_static(
        node: &FlowNode,
        input_context: &str,
        project_root: &PathBuf,
    ) -> Result<NodeResult> {
        match &node.action {
            NodeAction::Test => {
                // テスト実行
                Self::execute_test_static(project_root, input_context).await
            }
            NodeAction::Fix => {
                // エラー修正（AIにエラーログを渡して修正させる）
                Self::execute_fix_static(project_root, input_context).await
            }
            NodeAction::Design => {
                Self::execute_design_static(project_root, input_context).await
            }
            NodeAction::Implement => {
                Self::execute_implement_static(project_root, input_context).await
            }
            NodeAction::Refactor => {
                Self::execute_refactor_static(project_root, input_context).await
            }
            NodeAction::Custom { prompt } => {
                Self::execute_custom_static(project_root, input_context, prompt).await
            }
            NodeAction::HttpRequest { url, method, headers, body } => {
                Self::execute_http_request_static(url, method, headers.as_ref(), body.as_ref()).await
            }
            NodeAction::DataTransform { script } => {
                Self::execute_data_transform_static(input_context, script).await
            }
            NodeAction::FileOperation { operation, file_path, content } => {
                Self::execute_file_operation_static(project_root, operation, file_path, content.as_ref()).await
            }
            NodeAction::ScriptExecution { language, script } => {
                Self::execute_script_static(project_root, language, script).await
            }
        }
    }

    /// テスト実行ノード
    async fn execute_test(&self, project_root: &PathBuf, context: &str) -> Result<NodeResult> {
        Self::execute_test_static(project_root, context).await
    }

    /// テスト実行ノード（静的バージョン）
    async fn execute_test_static(project_root: &PathBuf, _context: &str) -> Result<NodeResult> {
        info!("Running tests in {:?}", project_root);

        // プロジェクトの種類を判定してカバレッジ付きテストコマンドを実行
        let (test_command, project_type) = if project_root.join("Cargo.toml").exists() {
            // Rustプロジェクト: cargo-tarpaulinを試す、なければ通常のテスト
            let has_tarpaulin = tokio::process::Command::new("cargo")
                .arg("tarpaulin")
                .arg("--version")
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if has_tarpaulin {
                ("cargo tarpaulin --out Stdout --output-dir target/coverage", "rust_coverage")
            } else {
                ("cargo test --no-fail-fast", "rust")
            }
        } else if project_root.join("package.json").exists() {
            // Node.jsプロジェクト: package.jsonのtest scriptでカバレッジを確認
            // npm test -- --coverage または jest --coverage を試す
            ("npm test -- --coverage 2>&1 || npm test", "node")
        } else if project_root.join("pytest.ini").exists() || project_root.join("setup.py").exists() {
            // Pythonプロジェクト: pytest-covを試す
            ("pytest --cov --cov-report=term 2>&1 || pytest", "python")
        } else {
            return Ok(NodeResult::Failure("テストコマンドが見つかりません".to_string()));
        };

        // 実行時間を計測
        let start = std::time::Instant::now();

        // テストを実行
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(test_command)
            .current_dir(project_root)
            .output()
            .await?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        // メトリクスを抽出
        let mut metrics = ExecutionMetrics {
            execution_time_ms: Some(execution_time_ms),
            ..Default::default()
        };

        // プロジェクトタイプに応じてメトリクスを抽出
        match project_type {
            "rust_coverage" => {
                // cargo-tarpaulinの出力からカバレッジを抽出
                // 例: "47.37% coverage, 18/38 lines covered"
                if let Some(captures) = regex::Regex::new(r"(\d+\.\d+)% coverage")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    if let Some(cov) = captures.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                        metrics.coverage = Some(cov);
                    }
                }

                // テスト数を抽出
                if let Some(captures) = regex::Regex::new(r"test result: \w+\. (\d+) passed; (\d+) failed")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    let passed: usize = captures.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    let failed: usize = captures.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    metrics.total_tests = Some(passed + failed);
                    metrics.failed_tests = Some(failed);
                }
            }
            "rust" => {
                // 通常のcargo testの出力からテスト数のみ抽出
                if let Some(captures) = regex::Regex::new(r"test result: \w+\. (\d+) passed; (\d+) failed")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    let passed: usize = captures.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    let failed: usize = captures.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    metrics.total_tests = Some(passed + failed);
                    metrics.failed_tests = Some(failed);
                }
            }
            "node" => {
                // Jestのカバレッジ出力を抽出
                // 例: "All files | 85.71 | 66.67 | 100 | 85.71"
                if let Some(captures) = regex::Regex::new(r"All files\s+\|\s+(\d+\.?\d*)")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    if let Some(cov) = captures.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                        metrics.coverage = Some(cov);
                    }
                }

                // テスト数を抽出
                // 例: "Tests: 5 passed, 5 total"
                if let Some(captures) = regex::Regex::new(r"Tests:\s+(\d+)\s+passed.*?(\d+)\s+total")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    let total: usize = captures.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    let passed: usize = captures.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                    metrics.total_tests = Some(total);
                    metrics.failed_tests = Some(total - passed);
                }
            }
            "python" => {
                // pytest-covのカバレッジ出力を抽出
                // 例: "TOTAL 156 39 75%"
                if let Some(captures) = regex::Regex::new(r"TOTAL\s+\d+\s+\d+\s+(\d+)%")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    if let Some(cov) = captures.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                        metrics.coverage = Some(cov);
                    }
                }

                // テスト数を抽出
                // 例: "5 passed in 0.12s"
                if let Some(captures) = regex::Regex::new(r"(\d+)\s+passed")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    if let Some(passed) = captures.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                        metrics.total_tests = Some(passed);
                        metrics.failed_tests = Some(0);
                    }
                }

                // 失敗したテストも抽出
                if let Some(captures) = regex::Regex::new(r"(\d+)\s+failed")
                    .ok()
                    .and_then(|re| re.captures(&combined))
                {
                    if let Some(failed) = captures.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                        metrics.failed_tests = Some(failed);
                        if let Some(total) = metrics.total_tests {
                            metrics.total_tests = Some(total + failed);
                        }
                    }
                }
            }
            _ => {}
        }

        if output.status.success() {
            let summary = if let Some(cov) = metrics.coverage {
                format!("✅ テスト成功 (カバレッジ: {:.2}%)\n\n{}", cov, combined)
            } else {
                format!("✅ テスト成功\n\n{}", combined)
            };

            Ok(NodeResult::Success(summary, metrics))
        } else {
            Ok(NodeResult::Failure(format!("❌ テスト失敗\n\n{}", combined)))
        }
    }

    /// エラー修正ノード（AIを呼び出す）
    async fn execute_fix(&self, project_root: &PathBuf, error_log: &str) -> Result<NodeResult> {
        Self::execute_fix_static(project_root, error_log).await
    }

    /// エラー修正ノード（静的バージョン）
    async fn execute_fix_static(project_root: &PathBuf, error_log: &str) -> Result<NodeResult> {
        info!("Executing fix node with error log");

        // エラーログを解析して、問題のあるファイルとエラー内容を特定
        let fix_prompt = Self::build_fix_prompt_static(error_log, project_root)?;

        // LLMを使って実際に修正を実行
        match Self::execute_with_llm_static(project_root, &fix_prompt).await {
            Ok(result) => {
                info!("Fix node completed successfully");
                Ok(NodeResult::Success(
                    format!("🔧 修正完了\n\n{}", result),
                    ExecutionMetrics::default(),
                ))
            }
            Err(e) => {
                warn!("Fix node LLM execution failed: {}", e);
                // LLM統合が失敗した場合でも、プロンプトは返す
                Ok(NodeResult::Success(
                    format!(
                        "🔧 修正プロンプトを生成しました（LLM実行失敗: {}）\n\n{}",
                        e, fix_prompt
                    ),
                    ExecutionMetrics::default(),
                ))
            }
        }
    }

    /// 修正用のプロンプトを構築
    fn build_fix_prompt(&self, error_log: &str, project_root: &PathBuf) -> Result<String> {
        Self::build_fix_prompt_static(error_log, project_root)
    }

    /// 修正用のプロンプトを構築（静的バージョン）
    fn build_fix_prompt_static(error_log: &str, project_root: &PathBuf) -> Result<String> {
        let prompt = format!(
            r#"あなたはエキスパートプログラマーです。以下のテストエラーを修正してください。

プロジェクトルート: {}

テストエラー:
```
{}
```

指示:
1. エラーログを分析して、問題の原因を特定してください
2. 該当するファイルを特定してください
3. write_file または edit_file ツールを使って修正を適用してください
4. 修正内容を簡潔に説明してください

注意事項:
- テストが通るように確実に修正してください
- コードの品質を維持してください
- 不要な変更は避けてください
"#,
            project_root.display(),
            error_log
        );

        Ok(prompt)
    }

    /// 設計ノード
    async fn execute_design(&self, project_root: &PathBuf, requirement: &str) -> Result<NodeResult> {
        Self::execute_design_static(project_root, requirement).await
    }

    /// 設計ノード（静的バージョン）
    async fn execute_design_static(project_root: &PathBuf, requirement: &str) -> Result<NodeResult> {
        info!("Executing design node");

        let design_prompt = format!(
            r#"あなたはソフトウェアアーキテクトです。以下の要件について詳細な設計を行ってください。

プロジェクトルート: {}

要件:
{}

指示:
1. 要件を分析して、実装すべき機能を明確化してください
2. 必要なファイルやモジュール構成を設計してください
3. データ構造とインターフェースを定義してください
4. 実装手順を段階的に示してください

設計には以下を含めてください:
- アーキテクチャ概要
- ファイル構成
- 主要な関数/クラスのシグネチャ
- データフロー
"#,
            project_root.display(),
            requirement
        );

        // LLMを使って設計を実行
        match Self::execute_with_llm_static(project_root, &design_prompt).await {
            Ok(result) => {
                info!("Design node completed successfully");
                Ok(NodeResult::Success(
                    format!("📐 設計完了\n\n{}", result),
                    ExecutionMetrics::default(),
                ))
            }
            Err(e) => {
                warn!("Design node LLM execution failed: {}", e);
                Ok(NodeResult::Success(
                    format!("📐 設計プロンプトを生成しました（LLM実行失敗: {}）\n\n{}", e, design_prompt),
                    ExecutionMetrics::default(),
                ))
            }
        }
    }

    /// 実装ノード
    async fn execute_implement(&self, project_root: &PathBuf, design: &str) -> Result<NodeResult> {
        Self::execute_implement_static(project_root, design).await
    }

    /// 実装ノード（静的バージョン）
    async fn execute_implement_static(project_root: &PathBuf, design: &str) -> Result<NodeResult> {
        info!("Executing implement node");

        let impl_prompt = format!(
            r#"あなたはエキスパート開発者です。以下の設計に基づいて実装してください。

プロジェクトルート: {}

設計:
{}

指示:
1. 設計に従って、必要なファイルを作成してください
2. write_file ツールを使ってコードを実装してください
3. エラーハンドリングとエッジケースを考慮してください
4. コードにコメントを追加して、実装の意図を明確にしてください
"#,
            project_root.display(),
            design
        );

        // LLMを使って実装を実行
        match Self::execute_with_llm_static(project_root, &impl_prompt).await {
            Ok(result) => {
                info!("Implement node completed successfully");
                Ok(NodeResult::Success(
                    format!("💻 実装完了\n\n{}", result),
                    ExecutionMetrics::default(),
                ))
            }
            Err(e) => {
                warn!("Implement node LLM execution failed: {}", e);
                Ok(NodeResult::Success(
                    format!("💻 実装プロンプトを生成しました（LLM実行失敗: {}）\n\n{}", e, impl_prompt),
                    ExecutionMetrics::default(),
                ))
            }
        }
    }

    /// リファクタリングノード
    async fn execute_refactor(&self, project_root: &PathBuf, code: &str) -> Result<NodeResult> {
        Self::execute_refactor_static(project_root, code).await
    }

    /// リファクタリングノード（静的バージョン）
    async fn execute_refactor_static(project_root: &PathBuf, code: &str) -> Result<NodeResult> {
        info!("Executing refactor node");

        let refactor_prompt = format!(
            r#"あなたはコード品質のエキスパートです。以下のコードをリファクタリングしてください。

プロジェクトルート: {}

コンテキスト:
{}

指示:
1. コードの可読性を向上させてください
2. 重複を排除してください
3. パフォーマンスを改善できる箇所を特定してください
4. edit_file ツールを使って改善を適用してください
"#,
            project_root.display(),
            code
        );

        // LLMを使ってリファクタリングを実行
        match Self::execute_with_llm_static(project_root, &refactor_prompt).await {
            Ok(result) => {
                info!("Refactor node completed successfully");
                Ok(NodeResult::Success(
                    format!("🧹 リファクタリング完了\n\n{}", result),
                    ExecutionMetrics::default(),
                ))
            }
            Err(e) => {
                warn!("Refactor node LLM execution failed: {}", e);
                Ok(NodeResult::Success(
                    format!("🧹 リファクタリングプロンプトを生成しました（LLM実行失敗: {}）\n\n{}", e, refactor_prompt),
                    ExecutionMetrics::default(),
                ))
            }
        }
    }

    /// カスタムプロンプトノード
    async fn execute_custom(
        &self,
        project_root: &PathBuf,
        context: &str,
        prompt: &str,
    ) -> Result<NodeResult> {
        Self::execute_custom_static(project_root, context, prompt).await
    }

    /// カスタムプロンプトノード（静的バージョン）
    async fn execute_custom_static(
        project_root: &PathBuf,
        context: &str,
        prompt: &str,
    ) -> Result<NodeResult> {
        info!("Executing custom node");

        let full_prompt = format!(
            r#"プロジェクトルート: {}

カスタムタスク:
{}

コンテキスト:
{}

⚠️  完全なLLM統合は開発中です
"#,
            project_root.display(),
            prompt,
            context
        );

        Ok(NodeResult::Success(
            format!("🎯 カスタムプロンプトを生成しました\n\n{}", full_prompt),
            ExecutionMetrics::default(),
        ))
    }

    /// LLMを使ってプロンプトを実行
    async fn execute_with_llm(&self, project_root: &PathBuf, prompt: &str) -> Result<String> {
        Self::execute_with_llm_static(project_root, prompt).await
    }

    /// LLMを使ってプロンプトを実行（静的バージョン）
    async fn execute_with_llm_static(project_root: &PathBuf, prompt: &str) -> Result<String> {
        info!("Executing with LLM");

        // 環境変数からAPIキーを取得
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY or OPENAI_API_KEY not set"))?;

        // デフォルトモデル名を決定
        let model_name = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "claude-3-5-sonnet-20240620".to_string()
        } else {
            "gpt-4o".to_string()
        };

        info!("Using model: {}", model_name);

        // Modelを作成
        let model = Model::new(
            model_name,
            None, // weak_model
            None, // editor_model
            None, // editor_edit_format
            false, // verbose
        )?;

        // LLMClientを初期化
        let llm_client = LLMClient::new(&model, api_key)?;

        // ConversationEngineを初期化
        let engine = ConversationEngine::with_project_root(project_root);

        // メッセージを作成
        let messages = vec![Message::user(prompt.to_string())];

        // シンプルなコールバック（何もしない）
        struct NoOpCallback;

        #[async_trait]
        impl ToolCallback for NoOpCallback {
            async fn on_tool_start(&mut self, _tool_name: &str, _args: &str) {}
            async fn on_tool_complete(&mut self, _tool_name: &str, _result: &str) {}
            async fn on_response(&mut self, _text: &str) {}
            async fn on_response_chunk(&mut self, _chunk: &str) {}
        }

        let mut callback = NoOpCallback;

        info!("Sending prompt to LLM");

        // ConversationEngineを使って実行
        let response = engine.execute(&llm_client, messages, project_root, &mut callback).await?;

        info!("LLM execution completed");

        Ok(response)
    }

    /// スナップショットを作成
    pub async fn create_snapshot(
        execution_id: &str,
        node_id: &str,
        node_name: &str,
        project_root: &PathBuf,
        tracked_files: &[String],
    ) -> Result<WorkflowSnapshot> {
        info!("Creating snapshot for node {} in {}", node_id, execution_id);

        let mut file_snapshots = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // 追跡対象のファイルのスナップショットを作成
        for file_path in tracked_files {
            let full_path = project_root.join(file_path);

            if full_path.exists() && full_path.is_file() {
                match tokio::fs::read_to_string(&full_path).await {
                    Ok(content) => {
                        file_snapshots.push(FileSnapshot {
                            file_path: file_path.clone(),
                            content,
                            timestamp: timestamp.clone(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to read file for snapshot {}: {}", file_path, e);
                    }
                }
            }
        }

        let snapshot = WorkflowSnapshot {
            snapshot_id: format!("{}-{}-{}", execution_id, node_id, uuid::Uuid::new_v4()),
            execution_id: execution_id.to_string(),
            node_id: node_id.to_string(),
            node_name: node_name.to_string(),
            files: file_snapshots,
            timestamp,
        };

        info!("Created snapshot {} with {} files", snapshot.snapshot_id, snapshot.files.len());
        Ok(snapshot)
    }

    /// スナップショットから復元
    pub async fn restore_snapshot(
        snapshot: &WorkflowSnapshot,
        project_root: &PathBuf,
    ) -> Result<()> {
        info!("Restoring snapshot {}", snapshot.snapshot_id);

        for file_snapshot in &snapshot.files {
            let full_path = project_root.join(&file_snapshot.file_path);

            // ディレクトリが存在しない場合は作成
            if let Some(parent) = full_path.parent() {
                if !parent.exists() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }

            // ファイルを復元
            tokio::fs::write(&full_path, &file_snapshot.content).await?;
            info!("Restored file: {}", file_snapshot.file_path);
        }

        info!("Successfully restored {} files from snapshot {}",
              snapshot.files.len(), snapshot.snapshot_id);
        Ok(())
    }

    /// プロジェクト内の全ファイルを取得（スナップショット対象）
    pub async fn get_tracked_files(project_root: &PathBuf) -> Result<Vec<String>> {
        let mut tracked_files = Vec::new();

        // .git, node_modules, target などを除外するパターン
        let ignore_patterns = vec![
            ".git",
            "node_modules",
            "target",
            ".berrycode",
            "dist",
            "build",
            ".cache",
        ];

        fn visit_dirs(
            dir: &std::path::Path,
            root: &std::path::Path,
            files: &mut Vec<String>,
            ignore_patterns: &[&str],
        ) -> std::io::Result<()> {
            if dir.is_dir() {
                // ディレクトリ名をチェック
                if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
                    if ignore_patterns.contains(&dir_name) {
                        return Ok(());
                    }
                }

                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_dir() {
                        visit_dirs(&path, root, files, ignore_patterns)?;
                    } else {
                        // 相対パスを保存
                        if let Ok(relative) = path.strip_prefix(root) {
                            if let Some(path_str) = relative.to_str() {
                                files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        visit_dirs(project_root, project_root, &mut tracked_files, &ignore_patterns)?;

        info!("Found {} tracked files in project", tracked_files.len());
        Ok(tracked_files)
    }
}

/// TDDループのプリセット
pub fn create_tdd_loop_preset() -> Pipeline {
    let mut pipeline = Pipeline::new(
        "tdd-loop".to_string(),
        "TDD Loop (Test → Fix → Re-Test)".to_string(),
        "test".to_string(),
    );

    // テストノード
    pipeline.add_node(FlowNode {
        id: "test".to_string(),
        name: "🧪 Test".to_string(),
        action: NodeAction::Test,
        next_on_success: None, // テスト成功したら終了
        next_on_failure: Some("fix".to_string()), // 失敗したら修正へ
        conditional_transitions: None,
        parallel_nodes: None,
    });

    // 修正ノード
    pipeline.add_node(FlowNode {
        id: "fix".to_string(),
        name: "🐛 Fix".to_string(),
        action: NodeAction::Fix,
        next_on_success: Some("test".to_string()), // 修正したら再テスト
        next_on_failure: None,
        conditional_transitions: None,
        parallel_nodes: None,
    });

    pipeline.max_loops = 5;
    pipeline
}

/// フルスタック開発パイプライン
pub fn create_full_dev_pipeline() -> Pipeline {
    let mut pipeline = Pipeline::new(
        "full-dev".to_string(),
        "Full Development (Design → Implement → Test → Fix → Refactor)".to_string(),
        "design".to_string(),
    );

    // 設計
    pipeline.add_node(FlowNode {
        id: "design".to_string(),
        name: "📐 Design".to_string(),
        action: NodeAction::Design,
        next_on_success: Some("implement".to_string()),
        next_on_failure: None,
        conditional_transitions: None,
        parallel_nodes: None,
    });

    // 実装
    pipeline.add_node(FlowNode {
        id: "implement".to_string(),
        name: "💻 Implement".to_string(),
        action: NodeAction::Implement,
        next_on_success: Some("test".to_string()),
        next_on_failure: None,
        conditional_transitions: None,
        parallel_nodes: None,
    });

    // テスト
    pipeline.add_node(FlowNode {
        id: "test".to_string(),
        name: "🧪 Test".to_string(),
        action: NodeAction::Test,
        next_on_success: Some("refactor".to_string()),
        next_on_failure: Some("fix".to_string()),
        conditional_transitions: None,
        parallel_nodes: None,
    });

    // 修正
    pipeline.add_node(FlowNode {
        id: "fix".to_string(),
        name: "🐛 Fix".to_string(),
        action: NodeAction::Fix,
        next_on_success: Some("test".to_string()), // 修正したら再テスト
        next_on_failure: None,
        conditional_transitions: None,
        parallel_nodes: None,
    });

    // リファクタリング
    pipeline.add_node(FlowNode {
        id: "refactor".to_string(),
        name: "🧹 Refactor".to_string(),
        action: NodeAction::Refactor,
        next_on_success: None, // 完了
        next_on_failure: None,
        conditional_transitions: None,
        parallel_nodes: None,
    });

    pipeline.max_loops = 3;
    pipeline
}

impl Pipeline {
    /// HTTP APIコールを実行
    async fn execute_http_request_static(
        url: &str,
        method: &str,
        headers: Option<&std::collections::HashMap<String, String>>,
        body: Option<&String>,
    ) -> Result<NodeResult> {
        info!("Executing HTTP request: {} {}", method, url);

        let client = reqwest::Client::new();
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => return Ok(NodeResult::Failure(format!("Unsupported HTTP method: {}", method))),
        };

        // ヘッダーを追加
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                request = request.header(key, value);
            }
        }

        // ボディを追加
        if let Some(b) = body {
            request = request.body(b.clone());
        }

        // リクエストを実行
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                let output = format!(
                    "✅ HTTP {} {}\nStatus: {}\nResponse:\n{}",
                    method.to_uppercase(),
                    url,
                    status,
                    body
                );

                if status.is_success() {
                    Ok(NodeResult::Success(output, ExecutionMetrics::default()))
                } else {
                    Ok(NodeResult::Failure(format!(
                        "❌ HTTP request failed with status {}\nResponse: {}",
                        status, body
                    )))
                }
            }
            Err(e) => Ok(NodeResult::Failure(format!("❌ HTTP request error: {}", e))),
        }
    }

    /// データ変換を実行（簡易版 - JSONパース/フォーマット）
    async fn execute_data_transform_static(
        input_context: &str,
        script: &str,
    ) -> Result<NodeResult> {
        info!("Executing data transform");

        // 簡易的なデータ変換（スクリプト言語なしでJSON操作のみ）
        if script.starts_with("json.parse") {
            match serde_json::from_str::<serde_json::Value>(input_context) {
                Ok(json) => {
                    let formatted = serde_json::to_string_pretty(&json)?;
                    Ok(NodeResult::Success(
                        format!("✅ JSON parsed:\n{}", formatted),
                        ExecutionMetrics::default(),
                    ))
                }
                Err(e) => Ok(NodeResult::Failure(format!("❌ JSON parse error: {}", e))),
            }
        } else if script.starts_with("json.stringify") {
            // 入力をそのままJSONとして整形
            Ok(NodeResult::Success(
                format!("✅ JSON stringified:\n{}", input_context),
                ExecutionMetrics::default(),
            ))
        } else {
            // その他のスクリプトは実行結果をそのまま返す
            Ok(NodeResult::Success(
                format!("⚠️ データ変換: {}\n入力:\n{}", script, input_context),
                ExecutionMetrics::default(),
            ))
        }
    }

    /// ファイル操作を実行
    async fn execute_file_operation_static(
        project_root: &PathBuf,
        operation: &str,
        file_path: &str,
        content: Option<&String>,
    ) -> Result<NodeResult> {
        info!("Executing file operation: {} on {}", operation, file_path);

        let full_path = project_root.join(file_path);

        match operation {
            "read" => {
                match tokio::fs::read_to_string(&full_path).await {
                    Ok(file_content) => Ok(NodeResult::Success(
                        format!("✅ ファイル読み込み成功: {}\n\n{}", file_path, file_content),
                        ExecutionMetrics::default(),
                    )),
                    Err(e) => Ok(NodeResult::Failure(
                        format!("❌ ファイル読み込みエラー: {}\n{}", file_path, e)
                    )),
                }
            }
            "write" => {
                if let Some(c) = content {
                    // 親ディレクトリを作成
                    if let Some(parent) = full_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }

                    match tokio::fs::write(&full_path, c).await {
                        Ok(_) => Ok(NodeResult::Success(
                            format!("✅ ファイル書き込み成功: {}\n\n書き込んだ内容:\n{}", file_path, c),
                            ExecutionMetrics::default(),
                        )),
                        Err(e) => Ok(NodeResult::Failure(
                            format!("❌ ファイル書き込みエラー: {}\n{}", file_path, e)
                        )),
                    }
                } else {
                    Ok(NodeResult::Failure(
                        "❌ 書き込み操作にはcontentが必要です".to_string()
                    ))
                }
            }
            _ => Ok(NodeResult::Failure(
                format!("❌ 不明なファイル操作: {}", operation)
            )),
        }
    }

    /// スクリプト実行
    async fn execute_script_static(
        project_root: &PathBuf,
        language: &str,
        script: &str,
    ) -> Result<NodeResult> {
        info!("Executing {} script", language);

        let (command, args) = match language {
            "bash" | "sh" => ("sh", vec!["-c", script]),
            "python" | "python3" => ("python3", vec!["-c", script]),
            "node" | "javascript" | "js" => ("node", vec!["-e", script]),
            "ruby" => ("ruby", vec!["-e", script]),
            _ => return Ok(NodeResult::Failure(
                format!("❌ サポートされていない言語: {}", language)
            )),
        };

        let output = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(project_root)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        if output.status.success() {
            Ok(NodeResult::Success(
                format!("✅ {} スクリプト実行成功:\n{}", language, combined),
                ExecutionMetrics::default(),
            ))
        } else {
            Ok(NodeResult::Failure(
                format!("❌ {} スクリプト実行失敗:\n{}", language, combined)
            ))
        }
    }
}
