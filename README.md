# Audio Logger

## Overview

Audio Logger is a program designed to run on a Raspberry Pi that automatically starts recording when noise is detected and stops when the noise ceases. The program monitors the audio input from a microphone and sends data over MQTT to a specified server. Key features include:

- Automatically starting and stopping the recording based on detected noise levels.
- Counting the number of times a loud noise is detected.
- Sending the current sound level, the level at which the recorder triggers, and the number of detected events to MQTT for Home Assistant integration.

## Environment Variables

The program requires the following environment variables to be set:

1. `MQTT_HOST`: The MQTT server's hostname or IP address.
2. `MQTT_USER`: The MQTT server's username.
3. `MQTT_PASSWORD`: The MQTT server's password.

## How it works

The program uses the `cpal` library to handle audio input and processing. It continuously listens for audio input from the default microphone, and processes the received data in real-time. The current RMS level is computed, and this information, along with other relevant data, is sent to the MQTT server.

The MQTT client is configured using the `paho_mqtt` library, and it publishes messages to the MQTT server with the current RMS level, event count, recorder state, target noise level, and availability status. The messages are sent every 2 seconds, or as configured by the `last_publish_time` variable.
