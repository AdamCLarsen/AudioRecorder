#![allow(dead_code)]

const INTPUT_TIMEOUT: Option<Duration> = Some(Duration::from_millis(250));

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{error::Error, sync::Arc, thread, time::Duration};

mod cir_buf; // Import the circular buffer module
mod recorder;
const BUFFER_SIZE: usize = 750;

fn main() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("Failed to get default input device");
    let input_config = input_device.default_input_config()?;
    let recording_state = Arc::new(recorder::RecorderState::new());
 
    println!("Default input device: {:?}", input_device.name()?);
    println!("Default input config: {:?}", input_config);

    let audio_buffer = Arc::new(std::sync::Mutex::new(cir_buf::CircularBuffer::<BUFFER_SIZE, f32>::new())); // Replace VecDeque with CircularBuffer

    let stream = input_device.build_input_stream(
        &input_config.config(),
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
        let audio_buffer = audio_buffer.lock().unwrap();
        let audio_data = audio_buffer.read_unordered(); // Update the main loop to use the read_unordered method
        let rms = calculate_rms(&audio_data);
        let rms_db = 20.0 * rms.log10();
        println!("Average RMS (last 750ms): {:.2} dB", rms_db);
    }
}

fn calculate_rms(data: &[&f32]) -> f32 {
    let sum: f32 = data.iter().map(|&sample| *sample * *sample).sum();
    (sum / (data.len() as f32)).sqrt()
}

