#![allow(dead_code)]

use std::time::Instant;

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

#[derive(Debug)]
pub struct RecorderState{
    recording_state: RecordingStates,
    noise_state: NoiseStates,
    last_state_change: Instant,
}

impl RecorderState {

    pub fn new() -> Self {
        RecorderState {
            recording_state: RecordingStates::Waiting,
            noise_state: NoiseStates::Quiet,
            last_state_change: Instant::now(),
        }
    }

    pub fn set_noise_state(&mut self, noise: NoiseStates) {
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

                if self.last_state_change.elapsed().as_secs() > 30 {
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
