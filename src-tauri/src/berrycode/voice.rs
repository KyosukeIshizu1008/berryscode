//! Voice input support using OpenAI Whisper API

use crate::berrycode::Result;
use anyhow::{anyhow, Context};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Audio format supported by Whisper
#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Ogg,
    WebM,
    M4a,
}

impl AudioFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Ogg => "ogg",
            AudioFormat::WebM => "webm",
            AudioFormat::M4a => "m4a",
        }
    }

    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "wav" => Some(AudioFormat::Wav),
            "mp3" => Some(AudioFormat::Mp3),
            "ogg" => Some(AudioFormat::Ogg),
            "webm" => Some(AudioFormat::WebM),
            "m4a" => Some(AudioFormat::M4a),
            _ => None,
        }
    }

    /// Get MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::WebM => "audio/webm",
            AudioFormat::M4a => "audio/m4a",
        }
    }
}

/// Response from Whisper API
#[derive(Debug, Serialize, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Voice input handler
#[cfg(feature = "voice")]
pub struct VoiceInput {
    client: reqwest::Client,
    api_key: String,
}

#[cfg(feature = "voice")]
impl VoiceInput {
    /// Create a new VoiceInput instance
    pub fn new(api_key: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, api_key })
    }

    /// Transcribe audio data using OpenAI Whisper API
    pub async fn transcribe(
        &self,
        audio_data: &[u8],
        format: AudioFormat,
    ) -> Result<String> {
        tracing::debug!("Transcribing audio ({} bytes, format: {:?})", audio_data.len(), format);

        // Create multipart form with audio file
        let file_part = Part::bytes(audio_data.to_vec())
            .file_name(format!("audio.{}", format.extension()))
            .mime_str(format.mime_type())
            .context("Failed to create file part")?;

        let form = Form::new()
            .text("model", "whisper-1")
            .part("file", file_part);

        // Send request to Whisper API
        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .context("Failed to send request to Whisper API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Whisper API request failed with status {}: {}", status, error_text);
        }

        let result: TranscriptionResponse = response
            .json()
            .await
            .context("Failed to parse Whisper API response")?;

        tracing::debug!("Transcription result: {}", result.text);

        Ok(result.text)
    }

    /// Transcribe audio from a file path
    pub async fn transcribe_file<P: AsRef<Path>>(&self, file_path: P) -> Result<String> {
        let path = file_path.as_ref();

        // Detect format from extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow!("File has no extension"))?;

        let format = AudioFormat::from_extension(extension)
            .ok_or_else(|| anyhow!("Unsupported audio format: {}", extension))?;

        // Read file
        let audio_data = std::fs::read(path)
            .context("Failed to read audio file")?;

        // Transcribe
        self.transcribe(&audio_data, format).await
    }

    /// Legacy listen method (deprecated, use transcribe instead)
    pub fn listen(&self) -> Result<Option<String>> {
        // This is a synchronous method, cannot use async Whisper API
        // Return None to indicate no input
        Ok(None)
    }
}

// Non-voice feature version
#[cfg(not(feature = "voice"))]
pub struct VoiceInput;

#[cfg(not(feature = "voice"))]
impl VoiceInput {
    pub fn new(_api_key: String) -> Result<Self> {
        Ok(Self)
    }

    pub async fn transcribe(&self, _audio_data: &[u8], _format: AudioFormat) -> Result<String> {
        anyhow::bail!("Voice feature is not enabled")
    }

    pub async fn transcribe_file<P: AsRef<Path>>(&self, _file_path: P) -> Result<String> {
        anyhow::bail!("Voice feature is not enabled")
    }

    pub fn listen(&self) -> Result<Option<String>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
        assert_eq!(AudioFormat::WebM.extension(), "webm");
    }

    #[test]
    fn test_audio_format_from_extension() {
        assert!(matches!(
            AudioFormat::from_extension("wav"),
            Some(AudioFormat::Wav)
        ));
        assert!(matches!(
            AudioFormat::from_extension("mp3"),
            Some(AudioFormat::Mp3)
        ));
        assert!(matches!(
            AudioFormat::from_extension("WAV"),
            Some(AudioFormat::Wav)
        ));
        assert!(AudioFormat::from_extension("unknown").is_none());
    }

    #[test]
    fn test_audio_format_mime_type() {
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
    }
}
