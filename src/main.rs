#![allow(dead_code)]

const INTPUT_TIMEOUT: Option<Duration> = Some(Duration::from_millis(250));

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{error::Error, sync::Arc, thread, time::Duration};
mod circular_buffer; // Import the circular buffer module

mod recorder;

fn main() -> Result<(), Box<dyn Error>> {
    let host: cpal::Host = cpal::default_host();

    host.input_devices().unwrap().enumerate().for_each(|device| println!("Input Device[{}]: {}", device.0, device.1.name().unwrap()));

    let input_device = host.default_input_device().expect("Failed to get default input device");
    let input_default_config = input_device.default_input_config()?;
    let recording_head = Arc::new(std::sync::Mutex::new(recorder::RecordingHead::new()));
    
    // input_device.supported_input_configs().unwrap().enumerate().for_each(|config| println!("Input Config[{}]: {:?}", config.0, config.1));
    // TODO: Look into the supported_input_configs() method and make sure one can do 16Khz or at least 8Khz.
    let sample_rate = input_default_config.sample_rate();
    let input_buffer_size = sample_rate.0 * 250 / 1000;
    let noise_sample_count = usize::try_from(sample_rate.0 * 750 / 1000).unwrap();

    let input_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: sample_rate,
        buffer_size: cpal::BufferSize::Fixed(input_buffer_size),
    };
    
    println!("Using input device: {:?}", input_device.name()?);
    println!("\tWith config: {:?}", input_config);
    
    let stream = input_device.build_input_stream(
        &input_config,
        {
            let recording_head = recording_head.clone();
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let audio_buffer = recording_head.lock().unwrap();
                audio_buffer.put(data);
            }
        },
        move |_err| (),
        INTPUT_TIMEOUT
    )?;

    stream.play()?;

    // Keep the main thread alive until the user stops the program.
    loop {
        thread::sleep(Duration::from_millis(250));
        update_noise_state(&recording_head, noise_sample_count);
    }
}

fn update_noise_state(recording_head: &Arc<std::sync::Mutex<recorder::RecordingHead>>, noise_sample_count: usize) {
    let mut recording_head = recording_head.lock().unwrap();
    let rms_db = recording_head.get_rms_as_db(noise_sample_count);
    if rms_db > -50.0 {
        recording_head.update_noise_state(recorder::NoiseStates::Noise);
    } else {
        recording_head.update_noise_state(recorder::NoiseStates::Quiet);
    }
}

