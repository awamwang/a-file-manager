// SPDX-License-Identifier: GPL-3.0-or-later
//! 麦克风采集：独立线程持有 cpal Stream（Stream 不进静态会话，
//! 会话只握停止信号与共享采样缓冲）。混音为单声道 f32 原生采样率，
//! 重采样推迟到收尾（`engine::resample_to_16k_mono`），采集路径保持无状态。

use super::MAX_RECORDING_SECS;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 采集线程的就绪句柄：会话侧只拿共享缓冲与控制通道。
pub struct CaptureHandle {
    pub samples: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub stop_tx: Sender<()>,
    pub done_rx: Receiver<()>,
}

pub enum CaptureError {
    NoDevice,
    Permission(String),
}

fn stream_error(error: cpal::StreamError) {
    eprintln!("voice capture stream error: {error}");
}

/// 单帧下混 + 缓冲写入；超上限（60s×1.5 余量）丢弃，防失控。
fn push_mono(iter: impl Iterator<Item = f32>, channels: usize, buf: &Mutex<Vec<f32>>, max_samples: usize) {
    let mut guard = match buf.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    if guard.len() >= max_samples {
        return;
    }
    let mut sum = 0.0f32;
    let mut count = 0usize;
    for (index, value) in iter.enumerate() {
        sum += value;
        count += 1;
        if (index + 1) % channels == 0 {
            guard.push(sum / count as f32);
            sum = 0.0;
            count = 0;
        }
    }
    // 数据长度不是声道整数倍的尾部帧：按已有声道均值入缓冲。
    if count > 0 {
        guard.push(sum / count as f32);
    }
}

/// 启动采集线程并等待设备就绪。线程内：建流 → play → 阻塞等停止信号
/// → drop stream → 发完成信号退出。
pub fn spawn_capture() -> Result<CaptureHandle, CaptureError> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<u32, CaptureError>>();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let thread_samples = Arc::clone(&samples);

    std::thread::spawn(move || {
        let setup = || -> Result<u32, CaptureError> {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or(CaptureError::NoDevice)?;
            let supported = device
                .default_input_config()
                .map_err(|error| {
                    CaptureError::Permission(format!(
                        "default input config failed: {error}"
                    ))
                })?;
            let sample_rate = supported.sample_rate().0;
            let channels = supported.channels().max(1) as usize;
            let sample_format = supported.sample_format();
            let stream_config: cpal::StreamConfig = supported.into();
            let max_samples =
                (sample_rate as f32 * MAX_RECORDING_SECS * 1.5) as usize;

            let buf = thread_samples.clone();
            let stream = match sample_format {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        push_mono(data.iter().copied(), channels, &buf, max_samples)
                    },
                    stream_error,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        push_mono(
                            data.iter().map(|value| *value as f32 / 32768.0),
                            channels,
                            &buf,
                            max_samples,
                        )
                    },
                    stream_error,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        push_mono(
                            data.iter()
                                .map(|value| (*value as f32 - 32768.0) / 32768.0),
                            channels,
                            &buf,
                            max_samples,
                        )
                    },
                    stream_error,
                    None,
                ),
                _ => return Err(CaptureError::NoDevice),
            }
            .map_err(|error| {
                CaptureError::Permission(format!(
                    "build input stream failed: {error}"
                ))
            })?;
            stream
                .play()
                .map_err(|error| {
                    CaptureError::Permission(format!(
                        "input stream play failed: {error}"
                    ))
                })?;
            Ok(sample_rate)
        };

        match setup() {
            Ok(sample_rate) => {
                let _ = ready_tx.send(Ok(sample_rate));
                // 阻塞等停止信号；发送端 drop 视同停止。
                loop {
                    match stop_rx.recv_timeout(Duration::from_millis(250)) {
                        Ok(()) => break,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error));
                return;
            }
        }
        // drop stream 在此，采集正式落停后通知会话侧。
        drop(stop_rx);
        let _ = done_tx.send(());
    });

    let sample_rate = ready_rx
        .recv()
        .map_err(|_| CaptureError::NoDevice)?
        .map_err(CaptureError::map)?;
    Ok(CaptureHandle {
        samples,
        sample_rate,
        stop_tx,
        done_rx,
    })
}

impl CaptureError {
    fn map(error: CaptureError) -> CaptureError {
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_mono_downmixes_stereo_and_caps_buffer() {
        let buf = Mutex::new(Vec::<f32>::new());
        // 双声道交替 L=1.0 R=0.0 → 单声道均值 0.5。
        let stereo: Vec<f32> = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        push_mono(stereo.iter().copied(), 2, &buf, 100);
        assert_eq!(*buf.lock().unwrap(), vec![0.5, 0.5, 0.5]);

        // 超上限后不再写入。
        let full = Mutex::new(vec![0.0f32; 4]);
        push_mono(vec![9.0f32].into_iter(), 1, &full, 4);
        assert_eq!(full.lock().unwrap().len(), 4);
        assert!(!full.lock().unwrap().contains(&9.0));
    }

    #[test]
    fn push_mono_keeps_partial_trailing_frame() {
        let buf = Mutex::new(Vec::<f32>::new());
        // 3 帧完整 + 1 个尾样本（L=0.8）：尾部按单声道均值入缓冲。
        push_mono(vec![0.2, 0.4, 0.2, 0.4, 0.2, 0.4, 0.8].into_iter(), 2, &buf, 100);
        assert_eq!(*buf.lock().unwrap(), vec![0.3, 0.3, 0.3, 0.8]);
    }
}
