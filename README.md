# Edge Agent

## Overview
The `edge-agent` is a high-performance, Rust-based edge node designed to capture audio data, evaluate local anomaly thresholds (in dB), and securely push anomaly payloads to RabbitMQ over AMQP. It acts as the distributed sensory layer of the acoustic monitoring infrastructure.

## Audio Modes
The agent supports two distinct modes of operation, configurable via the `AUDIO_SOURCE` environment variable:

- **`FILE`**: Reads audio from pre-recorded `.wav` files (specified by `AUDIO_FILE_PATH`). This mode is primarily used for automated Docker Compose simulations and testing.
- **`MICROPHONE` (or `MIC`)**: Captures live audio streams directly from the device's hardware.

> [!WARNING]
> **Crucial Note for Windows Users (Microphone Mode):**
> For accurate ML inference and anomaly detection, you **MUST** disable all "Audio Enhancements" and AI noise suppression features. Navigate to the Windows Sound Control Panel -> Recording -> Properties (of your microphone) -> Enhancements, and check "Disable all enhancements". Failing to do so will distort the raw acoustic data and degrade classification accuracy.

## Environment Variables
Configure the edge agent using an `.env` file or direct environment variables. The following are available:

| Variable | Description |
|----------|-------------|
| `AMQP_URI` | The RabbitMQ connection URI (e.g., `amqp://guest:guest@localhost:5672/%2f`). |
| `DEVICE_ID` | A unique identifier for this specific edge node (e.g., `sensor-01`). |
| `LATITUDE` | The geographical latitude of the sensor deployment. |
| `LONGITUDE` | The geographical longitude of the sensor deployment. |
| `AUDIO_SOURCE` | Sets the audio capture mode. Use `FILE` or `MIC`. |
| `AUDIO_FILE_PATH` | The path to the `.wav` file (used when `AUDIO_SOURCE=FILE`). |
| `ANOMALY_THRESHOLD_DB` | The local decibel threshold to trigger an anomaly event. |
| `TELEMETRY_INTERVAL_SECS` | The interval, in seconds, at which telemetry data is dispatched. |
| `DEFAULT_SAMPLE_RATE` | The sample rate for audio capture (default: `44100`). |
| `COOLDOWN_DURATION_SECS` | Seconds to wait between consecutive anomaly triggers. |

## Setup & Running

### Local Development
To run the agent locally during development, ensure you have Rust and Cargo installed, then execute:
```bash
cargo run
```

### Release Build
For production deployment, compile a highly optimized release binary:
```bash
cargo build --release
```
The resulting executable will be located in `target/release/edge-agent`.
