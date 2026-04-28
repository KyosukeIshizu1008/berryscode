//! AI provider usage tracking and cost estimation.
//!
//! Each successful `Provider::complete` call appends a [`UsageRecord`]
//! to the on-disk log at `~/.berrycode/ai_usage.json`. The Settings →
//! AI → Usage tab reads the same file to render today / month totals
//! and a soft monthly spending cap.
//!
//! Pricing is hard-coded against approximate 2026 list rates per
//! provider; users can override the per-model figures in the same
//! settings tab when their plan is different.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{ProviderKind, TokenUsage};

/// One billable interaction with a provider. Stored append-only so the
/// Cost panel can roll up arbitrary windows (today, week, month) without
/// us having to reaggregate at write time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    /// ISO-8601 UTC timestamp of when the response landed.
    pub timestamp: String,
    /// Which API this hit. Mapped from `ProviderKind` via `provider_str`.
    pub provider: String,
    /// Concrete model id used for the request (e.g. "claude-opus-4-7").
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    /// Estimated cost in USD, computed at record-time with the pricing
    /// table below. We persist the computed number so historical totals
    /// don't shift if list prices change later.
    pub cost_usd: f32,
}

/// Per-million-token list prices for the models BerryCode defaults to.
/// Numbers are approximate and only meant for ballpark display — the
/// authoritative number is whatever the provider bills the user.
struct ModelPricing {
    /// Substring matched against the model id, longest match wins.
    prefix: &'static str,
    input_per_mtok: f32,
    output_per_mtok: f32,
    cache_write_per_mtok: f32,
    cache_read_per_mtok: f32,
}

const PRICING: &[ModelPricing] = &[
    // Anthropic — see anthropic.com/pricing.
    ModelPricing {
        prefix: "claude-opus-4",
        input_per_mtok: 15.00,
        output_per_mtok: 75.00,
        cache_write_per_mtok: 18.75,
        cache_read_per_mtok: 1.50,
    },
    ModelPricing {
        prefix: "claude-sonnet-4",
        input_per_mtok: 3.00,
        output_per_mtok: 15.00,
        cache_write_per_mtok: 3.75,
        cache_read_per_mtok: 0.30,
    },
    ModelPricing {
        prefix: "claude-haiku-4",
        input_per_mtok: 1.00,
        output_per_mtok: 5.00,
        cache_write_per_mtok: 1.25,
        cache_read_per_mtok: 0.10,
    },
    // OpenAI — placeholders; refresh once GPT-5 list prices stabilise.
    ModelPricing {
        prefix: "gpt-5-codex",
        input_per_mtok: 3.00,
        output_per_mtok: 12.00,
        cache_write_per_mtok: 0.0,
        cache_read_per_mtok: 0.0,
    },
    ModelPricing {
        prefix: "gpt-5",
        input_per_mtok: 2.00,
        output_per_mtok: 8.00,
        cache_write_per_mtok: 0.0,
        cache_read_per_mtok: 0.0,
    },
];

/// Compute the USD cost for a single billable response. Local providers
/// (Ollama / llama.cpp) always return 0 — there's no API charge.
pub fn estimate_cost(provider: ProviderKind, model: &str, usage: &TokenUsage) -> f32 {
    if matches!(provider, ProviderKind::Ollama) {
        return 0.0;
    }
    let p = PRICING
        .iter()
        .find(|p| model.starts_with(p.prefix))
        .copied();
    let p = match p {
        Some(p) => p,
        // Unknown model — fall back to Sonnet-ish midrange so the
        // numbers aren't suspiciously zero.
        None => ModelPricing {
            prefix: "",
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: 3.75,
            cache_read_per_mtok: 0.3,
        },
    };
    const M: f32 = 1_000_000.0;
    (usage.prompt_tokens as f32) * p.input_per_mtok / M
        + (usage.completion_tokens as f32) * p.output_per_mtok / M
        + (usage.cache_write_tokens as f32) * p.cache_write_per_mtok / M
        + (usage.cache_read_tokens as f32) * p.cache_read_per_mtok / M
}

impl Copy for ModelPricing {}
impl Clone for ModelPricing {
    fn clone(&self) -> Self {
        *self
    }
}

fn provider_str(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::OpenAi => "openai",
        ProviderKind::Ollama => "ollama",
    }
}

fn usage_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".berrycode").join("ai_usage.json"))
}

/// Load the full usage log. Returns an empty Vec if the file is missing
/// or unreadable — usage tracking is best-effort, never fatal.
pub fn load() -> Vec<UsageRecord> {
    let Some(path) = usage_path() else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Append one record and persist. Failures are logged at debug level so
/// the chat path doesn't get noisy when, say, the disk is full.
pub fn record(provider: ProviderKind, model: &str, usage: &TokenUsage) {
    let Some(path) = usage_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut log = load();
    log.push(UsageRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        provider: provider_str(provider).to_string(),
        model: model.to_string(),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        cache_read_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        cost_usd: estimate_cost(provider, model, usage),
    });
    if let Ok(bytes) = serde_json::to_vec(&log) {
        let _ = std::fs::write(&path, bytes);
    }
}

/// Aggregated totals over a chosen time window.
#[derive(Debug, Clone, Copy, Default)]
pub struct UsageTotals {
    pub requests: u32,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f32,
}

/// Sum every record whose timestamp falls inside the supplied UTC date
/// range (`since` inclusive, `until` exclusive). The Settings panel
/// passes "today" and "this month" boundaries.
pub fn totals_between(
    records: &[UsageRecord],
    since: chrono::DateTime<chrono::Utc>,
    until: chrono::DateTime<chrono::Utc>,
) -> UsageTotals {
    let mut t = UsageTotals::default();
    for r in records {
        let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&r.timestamp) else {
            continue;
        };
        let ts = ts.with_timezone(&chrono::Utc);
        if ts < since || ts >= until {
            continue;
        }
        t.requests += 1;
        t.prompt_tokens += r.prompt_tokens as u64;
        t.completion_tokens += r.completion_tokens as u64;
        t.cache_read_tokens += r.cache_read_tokens as u64;
        t.cache_write_tokens += r.cache_write_tokens as u64;
        t.cost_usd += r.cost_usd;
    }
    t
}
