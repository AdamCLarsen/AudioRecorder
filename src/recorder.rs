#![allow(dead_code)]

use std::time::Instant;
use std::{sync::Arc};
use crate::circular_buffer::CircularBuffer;
const BUFFER_SIZE: usize = 16000 * 20;  // Assuming a standerd 16khz sample rate, this is 20 seconds of audio

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RecordingStates {
    PostRoll, // Will stop recording on the next cyle, and will write any buffered data to disk
    Recording, // Recording
    PreRoll,  // Not recording, but will start on next cyle
    Waiting,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum NoiseStates {
    Noise,
    Quiet
}

pub struct RecordingHead{
    pub recording_state: RecordingStates,
    noise_state: NoiseStates,
    last_state_change: Instant,
    audio_buffer: Arc<std::sync::Mutex<CircularBuffer<BUFFER_SIZE, f32>>>,
}

impl RecordingHead {

    pub fn new() -> Self {
        RecordingHead {
            recording_state: RecordingStates::Waiting,
            noise_state: NoiseStates::Quiet,
            last_state_change: Instant::now(),
            audio_buffer: Arc::new(std::sync::Mutex::new(CircularBuffer::<BUFFER_SIZE, f32>::new()))
        }
    }

    pub fn put(&self, data: &[f32]) {
        let mut audio_buffer = self.audio_buffer.lock().unwrap();
        for &sample in data {
            audio_buffer.put(sample); // Update the callback to use the put method
        }
    }

    pub fn get_rms_as_db(&self, sample_count: usize) -> f32 {
        let audio_buffer = self.audio_buffer.lock().unwrap();
        let rms = calculate_rms(audio_buffer.read_fifo_last_n(sample_count));
        return 20.0 * rms.log10();
    }

    pub fn update_noise_state(&mut self, noise: NoiseStates) {
        if self.noise_state == noise {
            self.update_state();
            return;
        }

        self.noise_state = noise;
        self.last_state_change = Instant::now();
    }

    pub fn recording_started(&mut self) -> Result<(), String> {
        if self.recording_state != RecordingStates::Waiting && self.recording_state != RecordingStates::PreRoll {
            return Err("Recording already started".to_string());
        }

        self.recording_state = RecordingStates::Recording;
        return Ok(());
    }

    pub fn recording_stopped(&mut self) {
        self.recording_state = RecordingStates::Waiting;
    }

    fn update_state(&mut self) {
        match self.recording_state {
            RecordingStates::PostRoll => {
               // Do nothing, once we have trigger the stop recording, we will wait for the next cycle.
            },
            RecordingStates::Recording => {
                if self.noise_state == NoiseStates::Noise {
                   // If we continue to hear noise, we will continue to record.
                   return;
                }

                if self.last_state_change.elapsed().as_secs() > 10 {
                    self.recording_state = RecordingStates::PostRoll;
                }
            },
            RecordingStates::PreRoll => {
                 // Do nothing, once we have trigger the stop recording, we will wait for the next cycle.
            },
            RecordingStates::Waiting => {
                if self.noise_state == NoiseStates::Quiet {
                    // Not recording, and no noise, do nothing.
                    return;
                }

                if self.last_state_change.elapsed().as_millis() > 750 {
                    self.recording_state = RecordingStates::PreRoll;
                }
            },
        }
    }   
}

fn calculate_rms(data:Vec<&f32>) -> f32 {
    let sum: f32 = data.iter().map(|&sample| *sample * *sample).sum();
    (sum / (data.len() as f32)).sqrt()
}
