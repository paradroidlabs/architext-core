//! Gemini REST Client — Bespoke, Zero-Dependency API Interface
//!
//! A lightweight async client for the Google Gemini API.
//! No SDK. No wrapper libraries. Just reqwest and raw REST.
//! This is the anti-abstraction-tax approach to LLM integration.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// The Gemini API client.
pub struct GeminiClient {
    /// HTTP client (connection-pooled, TLS-native)
    client: reqwest::Client,

    /// API key for authentication
    api_key: String,

    /// Base URL for the Gemini API
    base_url: String,

    /// Default model for prose generation
    pro_model: String,

    /// Default model for planning/review (fast, cheap)
    flash_model: String,
}

/// A message in the Gemini conversation format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiMessage {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

/// A content part in a Gemini message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

/// The request body for Gemini's generateContent endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiMessage>,
    generation_config: GenerationConfig,
}

/// Generation configuration parameters.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

/// The response from Gemini's generateContent endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: CandidateContent,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

/// Which tier of model to use for a given task.
#[derive(Debug, Clone, Copy)]
pub enum ModelTier {
    /// Fast/cheap model for planning, review, context ops
    Flash,
    /// Heavy creative model for prose generation
    Pro,
}

/// Parameters for a generation request.
pub struct GenerateParams {
    /// The system prompt (injected as system_instruction)
    pub system_prompt: String,
    /// The user message / task prompt (ignored if contents is Some)
    pub user_prompt: String,
    /// Multi-turn conversation messages
    pub contents: Option<Vec<GeminiMessage>>,
    /// Which model tier to use
    pub tier: ModelTier,
    /// Temperature (0.0 - 2.0). Higher = more creative.
    pub temperature: f32,
    /// Maximum output tokens
    pub max_tokens: u32,
}

impl GeminiClient {
    /// Create a new Gemini client with the given API key.
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 min timeout for long generations
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            pro_model: "gemini-2.5-flash".to_string(),
            flash_model: "gemini-2.5-flash".to_string(),
        }
    }

    /// Get the current pro model name.
    pub fn model(&self) -> &str {
        &self.pro_model
    }

    /// Check if the client has a valid API key configured.
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Execute a generation request against the Gemini API.
    /// Includes automatic retry with exponential backoff for transient errors (429, 503).
    pub async fn generate(&self, params: GenerateParams) -> Result<String> {
        if !self.is_configured() {
            anyhow::bail!("Gemini API key not configured. Set GEMINI_API_KEY in .env");
        }

        let model = match params.tier {
            ModelTier::Flash => &self.flash_model,
            ModelTier::Pro => &self.pro_model,
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        debug!("Gemini request → model={}, temp={}, max_tokens={}",
            model, params.temperature, params.max_tokens);

        let contents = match params.contents {
            Some(c) => c,
            None => vec![GeminiMessage {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: params.user_prompt,
                }],
            }],
        };

        let request_body = GenerateContentRequest {
            contents,
            system_instruction: Some(GeminiMessage {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: params.system_prompt,
                }],
            }),
            generation_config: GenerationConfig {
                temperature: params.temperature,
                max_output_tokens: params.max_tokens,
                top_p: Some(0.95),
                top_k: Some(40),
            },
        };

        // Retry loop: up to 3 retries with exponential backoff for transient errors
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_SECS: u64 = 5; // 5s, 15s, 45s

        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = BASE_DELAY_SECS * 3u64.pow(attempt - 1);
                warn!("[RETRY] Attempt {}/{} after {}s backoff...", attempt, MAX_RETRIES, delay);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }

            let response = match self.client
                .post(&url)
                .json(&request_body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    // Network-level failures are retryable
                    warn!("[RETRY] Network error on attempt {}: {}", attempt, e);
                    last_error = Some(format!("Network error: {}", e));
                    continue;
                }
            };

            let status = response.status();

            if status.is_success() {
                // Parse and return the successful response
                let response_body: GenerateContentResponse = response
                    .json()
                    .await
                    .context("Failed to parse Gemini API response")?;

                let text = response_body
                    .candidates
                    .and_then(|c| c.into_iter().next())
                    .map(|c| {
                        c.content
                            .parts
                            .into_iter()
                            .filter_map(|p| p.text)
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();

                if let Some(usage) = response_body.usage_metadata {
                    info!(
                        "[TOKENS] prompt={} output={} total={}",
                        usage.prompt_token_count.unwrap_or(0),
                        usage.candidates_token_count.unwrap_or(0),
                        usage.total_token_count.unwrap_or(0),
                    );
                }

                if text.is_empty() {
                    warn!("Gemini returned empty response");
                }

                if attempt > 0 {
                    info!("[RETRY] Succeeded on attempt {} after previous failures.", attempt);
                }

                return Ok(text);
            }

            // Check if the error is retryable
            let error_text = response.text().await.unwrap_or_default();
            let is_retryable = matches!(status.as_u16(), 429 | 503 | 500);

            if is_retryable && attempt < MAX_RETRIES {
                warn!(
                    "[RETRY] Transient error {} on attempt {}/{} — will retry. Body: {}",
                    status, attempt, MAX_RETRIES,
                    error_text.chars().take(200).collect::<String>()
                );
                last_error = Some(format!("HTTP {} — {}", status, error_text));
                continue;
            }

            // Non-retryable or exhausted retries — fail immediately
            anyhow::bail!(
                "Gemini API returned {} (after {} attempts) — {}",
                status, attempt + 1, error_text
            );
        }

        // Should only reach here if all retries were network errors
        anyhow::bail!(
            "Gemini API failed after {} retries — {}",
            MAX_RETRIES, last_error.unwrap_or_else(|| "unknown error".to_string())
        );
    }
}
