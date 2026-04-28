//! WAV → waveform-peaks decode for the audio preview panel.
//!
//! For waveform display we don't need every sample — at 44.1 kHz a
//! 30-second clip is ~1.3 M samples and rendering even one bar per
//! sample is wasteful. We downsample to a fixed number of buckets
//! and store the (min, max) pair per bucket so the rendered bar
//! captures the peak excursion in either direction.
//!
//! `hound` is the only decoder we lean on for now; mp3 / ogg / flac
//! support arrives in v0.6.1 by swapping `decode_wav_peaks` for a
//! `decode_peaks` dispatcher backed by `symphonia`.

use std::path::Path;

/// One bar of the rendered waveform — minimum and maximum sample
/// values inside the bucket, both in [-1.0, 1.0] floats. The render
/// path draws a vertical line between `min * h/2` and `max * h/2`
/// from the centre of each bar.
#[derive(Debug, Clone, Copy)]
pub struct PeakPair {
    pub min: f32,
    pub max: f32,
}

/// Result of decoding an audio file for preview. Holds the peak
/// array plus the metadata the UI needs to render axis labels.
#[derive(Debug, Clone)]
pub struct DecodedPeaks {
    /// One [`PeakPair`] per bar.
    pub peaks: Vec<PeakPair>,
    /// Sample rate of the source audio (e.g. 44_100).
    pub sample_rate: u32,
    /// Number of channels in the source. Peaks are mixed to mono
    /// before bucketing; this field is informational so the UI can
    /// surface "Stereo / 44.1 kHz" alongside the waveform.
    pub channels: u16,
    /// Total decoded duration in seconds.
    pub duration_secs: f32,
}

/// Decode a `.wav` file into `bucket_count` peak pairs. Returns
/// `Err` with a human message on any decode failure (file missing,
/// unsupported PCM bit depth, truncated stream, …).
///
/// The mono mix is naive (sum of all channels divided by count);
/// good enough for visual peaks. Stereo correlation analysis is a
/// future polish item the data flow already supports — just keep
/// per-channel peaks.
pub fn decode_wav_peaks(path: &Path, bucket_count: usize) -> Result<DecodedPeaks, String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV: {e}"))?;
    let spec = reader.spec();
    let total_frames = reader.duration() as usize;
    let channels = spec.channels.max(1) as usize;

    if total_frames == 0 {
        return Err("WAV file contains no audio frames".to_string());
    }

    // Mix-to-mono pass. `hound` exposes either int or float samples
    // depending on the file's bit-depth; convert both to f32 in
    // [-1.0, 1.0] up front so the bucketing loop doesn't branch on
    // sample type.
    let mono: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max = match spec.bits_per_sample {
                8 => i32::from(i8::MAX),
                16 => i32::from(i16::MAX),
                24 => (1i32 << 23) - 1,
                32 => i32::MAX,
                other => return Err(format!("Unsupported bit depth: {other}")),
            } as f32;
            let samples: Result<Vec<i32>, _> = reader.samples::<i32>().collect();
            let samples = samples.map_err(|e| format!("WAV decode error: {e}"))?;
            mix_mono_int(&samples, channels, max)
        }
        hound::SampleFormat::Float => {
            let samples: Result<Vec<f32>, _> = reader.samples::<f32>().collect();
            let samples = samples.map_err(|e| format!("WAV decode error: {e}"))?;
            mix_mono_float(&samples, channels)
        }
    };

    let bucket_count = bucket_count.max(1);
    let frames_per_bucket = (mono.len() / bucket_count).max(1);
    let mut peaks = Vec::with_capacity(bucket_count);
    for bucket in 0..bucket_count {
        let start = bucket * frames_per_bucket;
        if start >= mono.len() {
            break;
        }
        let end = (start + frames_per_bucket).min(mono.len());
        let mut min = 0.0f32;
        let mut max = 0.0f32;
        for &v in &mono[start..end] {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
        peaks.push(PeakPair { min, max });
    }

    Ok(DecodedPeaks {
        peaks,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        duration_secs: total_frames as f32 / spec.sample_rate as f32,
    })
}

fn mix_mono_int(samples: &[i32], channels: usize, max: f32) -> Vec<f32> {
    samples
        .chunks_exact(channels)
        .map(|frame| {
            let sum: f32 = frame.iter().map(|&s| s as f32 / max).sum();
            sum / channels as f32
        })
        .collect()
}

fn mix_mono_float(samples: &[f32], channels: usize) -> Vec<f32> {
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_wav(path: &Path, samples: &[i16], sample_rate: u32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for s in samples {
            writer.write_sample(*s).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn decode_simple_wav() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sine.wav");
        // 1 kHz sine at 8 kHz sample rate, 1 second long.
        let mut samples = Vec::with_capacity(8000);
        for i in 0..8000 {
            let t = i as f32 / 8000.0;
            let s = (t * 1000.0 * std::f32::consts::TAU).sin();
            samples.push((s * i16::MAX as f32) as i16);
        }
        write_test_wav(&path, &samples, 8000);

        let decoded = decode_wav_peaks(&path, 64).unwrap();
        assert_eq!(decoded.sample_rate, 8000);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.peaks.len(), 64);
        assert!((decoded.duration_secs - 1.0).abs() < 0.01);
        // Sine: every bucket should contain near-±1 samples.
        for p in &decoded.peaks {
            assert!(p.max > 0.5, "expected peak > 0.5, got {}", p.max);
            assert!(p.min < -0.5, "expected trough < -0.5, got {}", p.min);
        }
    }

    #[test]
    fn decode_missing_returns_err() {
        let path = std::path::Path::new("/nonexistent/file.wav");
        assert!(decode_wav_peaks(path, 64).is_err());
    }

    // Reference `Write` to keep the import live without warnings if
    // we extend the test module later.
    #[allow(dead_code)]
    fn _force_use(_w: impl Write) {}
}
