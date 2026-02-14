# Untracker

Untracker is a high-quality stem extraction tool for tracker music modules (MOD, S3M, XM, IT, etc.). It leverages the `libopenmpt` extension API to provide true isolation for instruments and samples, ensuring that each extracted stem is clean and free from interference from other tracks.

## Features

- **True Isolation**: Uses the OpenMPT Interactive Interface to mute/unmute instruments at the engine level.
- **Broad Format Support**: Supports all formats handled by `libopenmpt` (over 400 formats).
- **Flexible Output**: Supports WAV, Ogg Vorbis, Opus, and FLAC (feature-gated).
- **Smart Detection**: Automatically detects whether to use instrument-based or sample-based isolation.

## Installation

### Prerequisites

- Rust (latest stable)
- `libopenmpt-dev` and `pkg-config` (Linux/macOS)

### Building

```bash
cargo build --release
```

## Usage

```bash
untracker --input my_module.xm --output-dir ./stems --format wav
```

### Options

- `-i, --input <PATH>`: The module file to extract stems from.
- `-o, --output-dir <DIR>`: Directory where the stem files will be saved (default: current directory).
- `-f, --format <FORMAT>`: Output audio format (wav, vorbis, opus, flac).
- `--sample-rate <RATE>`: Sample rate in Hz (default: 44100).
- `--channels <NUM>`: Number of channels: 1 (mono) or 2 (stereo) (default: 2).
- `--resample <METHOD>`: Resampling method: nearest, linear, cubic, sinc (default: sinc).
- `--bit-depth <DEPTH>`: Bit depth for lossless formats: 16 or 24 (default: 16).
- `--stereo-separation <PERCENT>`: Stereo separation in percent (0-200, default: 100).
- `--opus-bitrate <BITRATE>`: Bitrate for Opus format in kbps (default: 128).
- `--vorbis-quality <LEVEL>`: Vorbis quality level (0-10, default: 5).

## License

This project is licensed under the BSD-3-Clause-Attribution License.
