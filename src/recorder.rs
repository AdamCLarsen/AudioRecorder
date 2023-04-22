#![allow(dead_code)]

use std::time::Instant;
use std::{sync::Arc};
use hound::WavWriter;
use crate::circular_buffer::CircularBuffer;
const BUFFER_SIZE: usize = 16000 * 20;  // Assuming a standerd 16khz sample rate, this is 20 seconds of audio

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RecordingStates {
    Recording, // Recording
    Waiting,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum NoiseStates {
    Noise,
    Quiet
}

pub struct RecordingHead{
    recording_state: RecordingStates,
    noise_state: NoiseStates,
    last_state_change: Instant,
    wav_writer: Option< WavWriter<std::io::BufWriter<std::fs::File>> >,
    wav_spec: hound::WavSpec,
    audio_buffer: Arc<std::sync::Mutex<CircularBuffer<BUFFER_SIZE, f32>>>,
}

impl RecordingHead {
    pub fn new(sample_rate:cpal::SampleRate) -> Self {
        RecordingHead {
            recording_state: RecordingStates::Waiting,
            noise_state: NoiseStates::Quiet,
            last_state_change: Instant::now(),
            wav_writer: None,
            wav_spec: hound::WavSpec {
                channels: 1,
                sample_rate: sample_rate.0 as u32,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Float,
            },
            audio_buffer: Arc::new(std::sync::Mutex::new(CircularBuffer::<BUFFER_SIZE, f32>::new()))
        }
    }

    pub fn put(&mut self, data: &[f32]) {
        let mut audio_buffer = self.audio_buffer.lock().unwrap();
        
        if self.recording_state == RecordingStates::Recording {
            if let Some(ref mut file) = self.wav_writer {
                for &sample in data {
                    audio_buffer.put(sample); // Update the callback to use the put method
                    file.write_sample(sample).unwrap();
                }
            } else {
                panic!("Recording state is recording, but file is not open.");
            }
        }else{
            for &sample in data {
                audio_buffer.put(sample); // Update the callback to use the put method
            }
        }
    }

    pub fn get_rms_as_db(&self, sample_count: usize) -> f32 {
        let audio_buffer = self.audio_buffer.lock().unwrap();
        let rms = calculate_rms(audio_buffer.read_fifo_last_n(sample_count));
        return 20.0 * rms.log10();
    }

    pub fn update_noise_state(&mut self, noise: NoiseStates) {
        if self.noise_state != noise {
            self.noise_state = noise;
            self.last_state_change = Instant::now();
        }

        self.update_recording_state();
        return;
    }

    fn update_recording_state(&mut self) {
        match self.recording_state {
            RecordingStates::Recording => {
                if self.noise_state == NoiseStates::Noise {
                   // If we continue to hear noise, we will continue to record.
                   return;
                }

                if self.last_state_change.elapsed().as_secs() > 10 {
                    // TODO: flush any remaining audio to the file, update the RIFF header with the lengths, and close the file.
                    let writer = self.wav_writer.take().expect("Nothing to do if we don't have a file.");
                    writer.finalize().expect("Failed to finalize WAV file");
                    self.recording_state = RecordingStates::Waiting;
                }
            },
            RecordingStates::Waiting => {
                if self.noise_state == NoiseStates::Quiet {
                    // Not recording, and no noise, do nothing.
                    return;
                }

                if self.last_state_change.elapsed().as_millis() > 750 {
                    let mut writer = WavWriter::create("output.wav", self.wav_spec).expect("Failed to create WAV file");
                    let audio_buffer = self.audio_buffer.lock().unwrap();
                    for &sample in audio_buffer.read_fifo_all() {
                        writer.write_sample(sample).unwrap();
                    }
    
                    self.wav_writer = Some(writer);
                    self.recording_state = RecordingStates::Recording;
                }
            },
        }
    }   
}

fn calculate_rms(data:Vec<&f32>) -> f32 {
    let sum: f32 = data.iter().map(|&sample| *sample * *sample).sum();
    (sum / (data.len() as f32)).sqrt()
}
