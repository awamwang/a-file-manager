// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenAI 兼容转写引擎（ADR 0018）：`POST {baseUrl}/v1/audio/transcriptions`
//! 形态的批量语音转写。ureq 3 无内置 multipart —— 手工构造（纯函数可单测）。
//! 配置每次请求前从 settings.json 现读（改设置即时生效，无需运行时同步）。

use super::VoiceError;
use crate::settings;
use std::time::Duration;
use tauri::AppHandle;
use ureq::Agent;

/// 设置项键（Power 层，ADR 0018）。
pub const BASE_URL_SETTING_KEY: &str = "power.sttBaseUrl";
pub const MODEL_SETTING_KEY: &str = "power.sttModel";
pub const API_KEY_SETTING_KEY: &str = "power.sttApiKey";

const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(30);
const TARGET_SAMPLE_RATE: u32 = 16_000;

pub struct SttConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

/// 规范化服务 base URL：去空白、去尾部 `/`；空串视为未配置。
pub fn normalize_base_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// 读取转写配置；任一项缺失 → `not-configured`。
pub fn read_config(app: &AppHandle) -> Result<SttConfig, VoiceError> {
    let base_url = normalize_base_url(&settings::read_string_setting(
        app,
        BASE_URL_SETTING_KEY,
    ))
    .ok_or_else(VoiceError::not_configured)?;
    let model = settings::read_string_setting(app, MODEL_SETTING_KEY)
        .trim()
        .to_string();
    let api_key = settings::read_string_setting(app, API_KEY_SETTING_KEY)
        .trim()
        .to_string();
    if model.is_empty() || api_key.is_empty() {
        return Err(VoiceError::not_configured());
    }
    Ok(SttConfig {
        base_url,
        model,
        api_key,
    })
}

fn clamp_i16(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// 线性插值重采样到 16k 单声道 i16；源率相同则直通。
pub fn resample_to_16k_mono(input: &[f32], src_rate: u32) -> Vec<i16> {
    if input.is_empty() {
        return Vec::new();
    }
    if src_rate == TARGET_SAMPLE_RATE {
        return input.iter().map(|value| clamp_i16(*value)).collect();
    }
    let step = src_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let out_len = ((input.len() as f64) / step).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for index in 0..out_len {
        let position = index as f64 * step;
        let idx = position.floor() as usize;
        let frac = (position - position.floor()) as f32;
        let a = input[idx];
        let b = if idx + 1 < input.len() { input[idx + 1] } else { a };
        out.push(clamp_i16(a + (b - a) * frac));
    }
    out
}

/// 内存编码 WAV（16k / mono / 16bit PCM，手写 44 字节头；hound 的 finalize
/// 会连 writer 一起消费、取不回内存缓冲，故不引入）。
pub fn encode_wav(samples: &[i16]) -> Result<Vec<u8>, String> {
    let data_len = samples.len().checked_mul(2).ok_or("wav data overflow")?;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&TARGET_SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(TARGET_SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(out)
}

/// 手工构造 multipart/form-data 主体（ureq 3 无内置支持）。
pub fn build_multipart(boundary: &str, model: &str, wav: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"model\"\r\n\r\n");
    body.extend_from_slice(model.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(wav);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(TRANSCRIBE_TIMEOUT))
        // 401/5xx 不作为传输错误：必须读响应体做统一错误映射
        //（同 knowledge_service 的 ureq 3 注记）。
        .http_status_as_error(false)
        .build()
        .into()
}

/// 上传转写：成功返回识别文本。
pub fn transcribe(config: &SttConfig, wav: &[u8]) -> Result<String, VoiceError> {
    const BOUNDARY: &str = "afm-voice-boundary-7f3a1c9d";
    let url = format!("{}/audio/transcriptions", config.base_url);
    let body = build_multipart(BOUNDARY, &config.model, wav);
    let mut response = agent()
        .post(&url)
        .header(
            "Authorization",
            format!("Bearer {}", config.api_key).as_str(),
        )
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={BOUNDARY}").as_str(),
        )
        .send(body)
        .map_err(|error| {
            VoiceError::network(format!("transcription request failed ({url}): {error}"))
        })?;
    let status = response.status().as_u16();
    let value: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| {
            VoiceError::server(
                format!("transcription response decode failed: {error}"),
                status,
            )
        })?;
    if status == 401 || status == 403 {
        return Err(VoiceError::unauthorized(status));
    }
    if status >= 400 {
        return Err(VoiceError::server(
            format!("transcription failed with HTTP {status}"),
            status,
        ));
    }
    value
        .get("text")
        .and_then(|field| field.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            VoiceError::server("transcription response missing text field", status)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_base_url_trims_and_strips_slash() {
        assert_eq!(
            normalize_base_url("  https://api.siliconflow.cn/v1/ "),
            Some("https://api.siliconflow.cn/v1".to_string())
        );
        assert_eq!(normalize_base_url(""), None);
        assert_eq!(normalize_base_url("   "), None);
        assert_eq!(normalize_base_url("///"), None);
    }

    #[test]
    fn resample_passthrough_at_16k() {
        let input: Vec<f32> = vec![0.0, 0.5, -0.5, 1.0];
        let out = resample_to_16k_mono(&input, 16_000);
        assert_eq!(out, vec![0, 16383, -16383, 32767]);
    }

    #[test]
    fn resample_48k_to_16k_thirds_length_and_interpolates() {
        // 4800 样本 @48k = 0.1s → 1600 输出。
        let input: Vec<f32> = vec![0.25f32; 4800];
        let out = resample_to_16k_mono(&input, 48_000);
        assert_eq!(out.len(), 1600);
        assert!(out.iter().all(|value| *value == 8191 || *value == 8192));
    }

    #[test]
    fn resample_empty_input_yields_empty() {
        assert!(resample_to_16k_mono(&[], 48_000).is_empty());
    }

    #[test]
    fn encode_wav_writes_riff_header_and_16k_payload() {
        let samples: Vec<i16> = vec![0, 100, -100, 32767];
        let wav = encode_wav(&samples).expect("encode wav");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(rate, 16_000);
        assert_eq!(wav.len(), 44 + samples.len() * 2);
        // 首样本数据位于 44 偏移（小端 i16）。
        assert_eq!(i16::from_le_bytes([wav[44], wav[45]]), 0);
        assert_eq!(i16::from_le_bytes([wav[46], wav[47]]), 100);
    }

    #[test]
    fn build_multipart_wraps_model_and_wav() {
        let wav: Vec<u8> = vec![0x52, 0x49, 0x46, 0x46, 0x00, 0x01];
        let body = build_multipart("bnd", "whisper-1", &wav);
        let text_start = b"--bnd\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nwhisper-1\r\n--bnd\r\n";
        assert_eq!(&body[..text_start.len()], &text_start[..]);
        // 文件部分头完整且紧接 WAV 字节。
        let file_header =
            b"Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n";
        let header_pos = body
            .windows(file_header.len())
            .position(|window| window == file_header)
            .expect("file part header present");
        assert_eq!(&body[header_pos + file_header.len()..][..wav.len()], &wav[..]);
        // 以闭合 boundary 结尾。
        assert!(body.ends_with(b"\r\n--bnd--\r\n"));
    }
}
