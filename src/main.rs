#![allow(dead_code)]

const INTPUT_TIMEOUT: Option<Duration> = Some(Duration::from_millis(250));

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{error::Error, sync::Arc, thread, time::Duration};

mod cir_buf; // Import the circular buffer module
mod recorder;
const BUFFER_SIZE_AT_16KHZ: usize = 12000;  // Assuming a standerd 16khz sample rate, this is 750ms of audio
const BUFFER_SIZE_AT_8KHZ: usize = 6000;  // Assuming a standerd 16khz sample rate, this is 750ms of audio

fn main() -> Result<(), Box<dyn Error>> {
    let host: cpal::Host = cpal::default_host();

    host.input_devices().unwrap().enumerate().for_each(|device| println!("Input Device[{}]: {}", device.0, device.1.name().unwrap()));

    let input_device = host.default_input_device().expect("Failed to get default input device");
    let input_default_config = input_device.default_input_config()?;
    let recording_state = Arc::new(std::sync::Mutex::new(recorder::RecorderState::new()));
    
    // input_device.supported_input_configs().unwrap().enumerate().for_each(|config| println!("Input Config[{}]: {:?}", config.0, config.1));
    // TODO: Look into the supported_input_configs() method and make sure one can do 16Khz or at least 8Khz.

    let input_buffer_size = input_default_config.sample_rate().0 * 250 / 1000;

    let input_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: input_default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(input_buffer_size),
    };
    
    println!("Using input device: {:?}", input_device.name()?);
    println!("\tWith config: {:?}", input_config);

    let audio_buffer = Arc::new(std::sync::Mutex::new(cir_buf::CircularBuffer::<BUFFER_SIZE_AT_16KHZ, f32>::new()));
    
    let stream = input_device.build_input_stream(
        &input_config,
        {
            let audio_buffer = audio_buffer.clone();
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut audio_buffer = audio_buffer.lock().unwrap();
                for &sample in data {
                    audio_buffer.put(sample); // Update the callback to use the put method
                }
            }
        },
        move |_err| (),
        INTPUT_TIMEOUT
    )?;

    stream.play()?;

    // Keep the main thread alive until the user stops the program.
    loop {
        thread::sleep(Duration::from_millis(250));
        update_noise_state(&audio_buffer, &recording_state);
        let mut recording_state = recording_state.lock().unwrap();
        // check get_recorder_instruction and take the required actions.
        match recording_state.recording_state {
            recorder::RecordingStates::PostRoll => {
                // TODO: Stop the Recorder
                println!("PostRoll");
                recording_state.recording_stopped();
            }
            recorder::RecordingStates::Recording => (),
            recorder::RecordingStates::PreRoll => {
                    // TODO: Start the Recorder
                    println!("PreRoll");
                    recording_state.recording_started().expect("Invalid state transition");
            }
            recorder::RecordingStates::Waiting =>  (),
        }
    }
}

fn update_noise_state(audio_buffer: &Arc<std::sync::Mutex<cir_buf::CircularBuffer<BUFFER_SIZE_AT_16KHZ, f32>>>, recording_state: &Arc<std::sync::Mutex<recorder::RecorderState>>) {
    let audio_buffer = audio_buffer.lock().unwrap();
    let audio_data = audio_buffer.read_unordered();
    // Update the main loop to use the read_unordered method
    let rms = calculate_rms(&audio_data);
    let rms_db = 20.0 * rms.log10();
    //println!("Average RMS (last 750ms): {:.2} dB", rms_db);
    let mut recording_state = recording_state.lock().unwrap();
    if rms_db > -50.0 {
        recording_state.set_noise_state(recorder::NoiseStates::Noise);
    } else {
        recording_state.set_noise_state(recorder::NoiseStates::Quiet);
    }
}

fn calculate_rms(data: &[&f32]) -> f32 {
    let sum: f32 = data.iter().map(|&sample| *sample * *sample).sum();
    (sum / (data.len() as f32)).sqrt()
}

