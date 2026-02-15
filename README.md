# Untracker

Untracker is a high-quality stem extraction tool for tracker music modules (MOD, S3M, XM, IT, etc.). It leverages the `libopenmpt` extension API to provide true isolation for instruments and samples, ensuring that each extracted stem is clean and free from interference from other tracks.

## Features

- **True Isolation**: Uses the OpenMPT Interactive Interface to mute/unmute instruments at the engine level.
- **Broad Format Support**: Supports all formats handled by `libopenmpt` (over 400 formats).
- **Flexible Output**: Supports WAV, Ogg Vorbis, Opus, and FLAC (feature-gated).
- **Smart Detection**: Automatically detects whether to use instrument-based or sample-based isolation.
- **Parallel Processing**: Extract multiple stems simultaneously with the `--parallel` flag.
- **High-Quality Resampling**: Multiple resampling methods available (nearest, linear, cubic, sinc).
- **Customizable Audio Quality**: Adjustable sample rate, bit depth, and format-specific settings.

## Installation

### Prerequisites

- Rust (latest stable)
- `libopenmpt-dev` and `pkg-config` (Linux/macOS)

### Building

Standard build (requires `libopenmpt` installed on the system):
```bash
cargo build --release
```

Static build (builds `libopenmpt` from source and bundles it):
```bash
make static
```

### Note on musl builds

Building for musl targets requires additional setup due to cross-compilation complexities. The project can be built normally for glibc systems using the commands above.

## Usage

```bash
untracker --input my_module.xm --output-dir ./stems --format wav
```

### Command Line Options

```text
  -i, --input <INPUT>
          Input module file path
  -o, --output-dir <OUTPUT_DIR>
          Output directory for stem files
      --sample-rate <SAMPLE_RATE>
          Sample rate [default: 44100]
      --channels <CHANNELS>
          Number of channels (1 or 2) [default: 2]
      --resample <RESAMPLE>
          Resampling method [default: sinc] [possible values: nearest, linear, cubic, sinc]
      --format <FORMAT>
          Output format: wav, vorbis, opus, flac [default: wav]
      --bit-depth <BIT_DEPTH>
          Bit depth for lossless formats (16 or 24) [default: 16]
      --opus-bitrate <OPUS_BITRATE>
          Bitrate for Opus format in kbps [default: 128]
      --vorbis-quality <VORBIS_QUALITY>
          Vorbis quality level (0-10) [default: 5]
      --stereo-separation <STEREO_SEPARATION>
          Stereo separation in percent (0-200) [default: 100]
  -p, --parallel
          Render stems in parallel
  -h, --help
          Print help
  -V, --version
          Print version
```

#### Audio Formats

- **WAV**: Uncompressed PCM audio (default format)
- **Vorbis**: Ogg Vorbis compressed audio (requires `vorbis` feature)
- **Opus**: Opus compressed audio (requires `opus` feature)
- **FLAC**: Lossless compressed audio (requires `flac` feature)

#### Advanced Options

- **Sample Rate**: Supports any sample rate (though Opus is limited to 8, 12, 16, 24, or 48 kHz)
- **Channels**: 1 (mono) or 2 (stereo)
- **Resampling**: Choose from nearest neighbor, linear, cubic, or sinc interpolation
- **Stereo Separation**: Adjust left/right channel separation (0% = mono, 100% = normal, 200% = exaggerated)
- **Bit Depth**: 16-bit or 24-bit for lossless formats
- **Opus Bitrate**: Custom bitrate from 64 kbps to 512 kbps
- **Vorbis Quality**: Scale from 0 (lowest) to 10 (highest)

## Examples

Extract stems in WAV format:
```bash
untracker -i song.xm -o stems/
```

Extract with high-quality 24-bit FLAC output:
```bash
untracker -i song.it -o stems/ --format flac --bit-depth 24
```

Extract with parallel processing for faster results:
```bash
untracker -i song.mod -o stems/ --parallel
```

Extract with custom settings (48kHz, stereo, high quality):
```bash
untracker -i song.s3m -o stems/ --sample-rate 48000 --format vorbis --vorbis-quality 9
```

## License

This project is licensed under the BSD-3-Clause-Attribution License.
