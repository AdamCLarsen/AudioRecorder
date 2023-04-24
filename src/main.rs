const INTPUT_TIMEOUT: Option<Duration> = Some(Duration::from_millis(250));
const RMT_HISTORY_SIZE:usize = 4 * 60  ; // 4 samples per second, 60 seconds per minute

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{error::Error, sync::Arc, thread, sync::Mutex, time::Duration};
use std::cmp::Ordering;
use std::env;
use std::time::Instant;

mod circular_buffer_stack;
mod circular_buffer;
mod recorder;

struct TargetNoiseFloor {
    target_noise_floor: f32,
    noise_floor: f32,
    event_count: u32,
}

// Add the required imports at the beginning of the file
use paho_mqtt::{Client, Message};

// Add a helper function to initialize the MQTT client

fn init_mqtt_client() -> Result<Client, Box<dyn Error>> {
    let host = env::var("MQTT_HOST").expect("Please set the MQTT_HOST environment variable");
    //let port = env::var("MQTT_PORT")
    //    .expect("Please set the MQTT_PORT environment variable")
    //    .parse::<u16>()
    //    .expect("MQTT_PORT must be a valid number");
    let user_name = env::var("MQTT_USER").expect("Please set the MQTT_USER environment variable");
    let password = env::var("MQTT_PASSWORD").expect("Please set the MQTT_PASSWORD environment variable");

    let create_opts = paho_mqtt::CreateOptionsBuilder::new()
    .server_uri(host)
    .client_id("Audio Logger")
    .finalize();

    // Create a client.
    let cli = paho_mqtt::Client::new(create_opts).expect("Error creating the MQTT client");

    let last_will_message = Message::new("homeassistant/sensor/audioLog/availability", "offline", paho_mqtt::QOS_1);
    last_will_message.retained();
   
    let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
    .keep_alive_interval(Duration::from_secs(20))
    .clean_session(true)
    .user_name(user_name)
    .password(password)
    .automatic_reconnect(Duration::from_secs(10),Duration::from_secs(240))
    .will_message(last_will_message)
    .finalize();

    cli.connect(conn_opts).expect("Error connecting to MQTT server");
    Ok(cli)
}

fn main() -> Result<(), Box<dyn Error>> {
    let host: cpal::Host = cpal::default_host();

    host.input_devices().unwrap().enumerate().for_each(|device| println!("Input Device[{}]: {}", device.0, device.1.name().unwrap()));

    let input_device = host.default_input_device().expect("Failed to get default input device");
    let input_default_config = input_device.default_input_config()?;
    
    // input_device.supported_input_configs().unwrap().enumerate().for_each(|config| println!("Input Config[{}]: {:?}", config.0, config.1));
    // TODO: Look into the supported_input_configs() method and make sure one can do 16Khz or at least 8Khz.
    let sample_rate = input_default_config.sample_rate();
    let input_buffer_size = sample_rate.0 * 250 / 1000;
    let noise_sample_count = usize::try_from(sample_rate.0 * 500 / 1000).unwrap();

    let input_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: sample_rate,
        buffer_size: cpal::BufferSize::Fixed(input_buffer_size),
    };
    
    let recording_head = Arc::new(Mutex::new(recorder::RecordingHead::new(sample_rate)));
    let rms_history = Arc::new(Mutex::new(circular_buffer_stack::CircularBufferStack::<RMT_HISTORY_SIZE, f32>::new()));

    println!("Using input device: {:?}", input_device.name()?);
    println!("\tWith config: {:?}", input_config);
    
    let stream = input_device.build_input_stream(
        &input_config,
        {
            let recording_head = recording_head.clone();
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut recording_head = recording_head.lock().unwrap();
                recording_head.put(data);
            }
        },
        move |_err| (),
        INTPUT_TIMEOUT
    )?;

    println!("Starting stream");
    stream.play()?;

    println!("Starting MQTT client");
    let mut last_publish_time = Instant::now();
    let mqtt_client = init_mqtt_client().expect("Failed to initialize MQTT client");
    let mut target_noise_floor = TargetNoiseFloor {
        target_noise_floor: -30.0,
        noise_floor: -20.0,
        event_count: 0,
    };

    let mut is_triggered: bool = false;
    println!("Monitoring for noise...");

    // Keep the main thread alive until the user stops the program.
    loop {
        thread::sleep(Duration::from_millis(250));
        let cur_rms = update_noise_state(&recording_head, &rms_history, noise_sample_count, target_noise_floor.target_noise_floor);

        // Compute the 90th percentile of the RMS history, and use that as the noise floor going forward.
        let rms_history = rms_history.lock().unwrap();
        
        if rms_history.is_full() {
            let mut rms_history = rms_history.clone();
            rms_history.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            let noise_floor_50th = rms_history[rms_history.len() * 5 / 10];
            let noise_floor_90th = rms_history[rms_history.len() * 9 / 10] ;

            target_noise_floor.noise_floor = *noise_floor_50th;
            target_noise_floor.target_noise_floor = noise_floor_90th + 6.0;
        }

        if cur_rms > target_noise_floor.target_noise_floor {
            if !is_triggered {
                is_triggered = true;
                target_noise_floor.event_count += 1;
            }
        } else {
            is_triggered = false;
        }

        let recording_head = recording_head.lock().unwrap();
        let recorder_state = &recording_head.recording_state;
        print!("\rRMS {} - floor {} | target {} - events {} - recorder {:?}        ", cur_rms, target_noise_floor.noise_floor, target_noise_floor.target_noise_floor, target_noise_floor.event_count, recorder_state);
        
        if last_publish_time.elapsed() >= Duration::from_secs(2) {
            let messages = [
                Message::new("homeassistant/sensor/audioLog/rms/state", cur_rms.to_string(), paho_mqtt::QOS_1),
                Message::new("homeassistant/sensor/audioLog/eventCount/state", target_noise_floor.event_count.to_string(), paho_mqtt::QOS_1),
                Message::new("homeassistant/sensor/audioLog/recorder/state", (*recorder_state).to_string(), paho_mqtt::QOS_1),
                Message::new("homeassistant/sensor/audioLog/targetNoise/state", target_noise_floor.target_noise_floor.to_string(), paho_mqtt::QOS_1),
                Message::new("homeassistant/sensor/audioLog/availability", "online", paho_mqtt::QOS_1),
            ];
        
            for message in &messages {
                if let Err(e) = mqtt_client.publish(message.clone()) {
                    eprintln!("Error publishing message: {:?}", e);
                }
            }
        
            last_publish_time = Instant::now();
        }
   
    }
}

fn update_noise_state(recording_head: &Arc<std::sync::Mutex<recorder::RecordingHead>>, rms_history: &Arc<Mutex<circular_buffer_stack::CircularBufferStack<RMT_HISTORY_SIZE, f32>>>, noise_sample_count: usize, noise_floor: f32) -> f32 {
    let mut recording_head = recording_head.lock().unwrap();
    let rms_db = recording_head.get_rms_as_db(noise_sample_count);
    rms_history.lock().unwrap().put(rms_db);
    if rms_db > noise_floor {
        recording_head.update_noise_state(recorder::NoiseStates::Noise);
    } else {
        recording_head.update_noise_state(recorder::NoiseStates::Quiet);
    }

    return rms_db;
}

