use anyhow::{anyhow, Result};
use hound::{WavSpec, WavWriter};

#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Wav,
    #[cfg(feature = "vorbis")]
    Vorbis,
    #[cfg(feature = "opus")]
    Opus,
    #[cfg(feature = "flac")]
    Flac,
}

impl std::str::FromStr for AudioFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wav" => Ok(AudioFormat::Wav),
            #[cfg(feature = "vorbis")]
            "vorbis" | "ogg" => Ok(AudioFormat::Vorbis),
            #[cfg(feature = "opus")]
            "opus" => Ok(AudioFormat::Opus),
            #[cfg(feature = "flac")]
            "flac" => Ok(AudioFormat::Flac),
            _ => Err(anyhow!("Unsupported or disabled audio format: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResampleMethod {
    Nearest,
    Linear,
    Cubic,
    Sinc,
}

impl ResampleMethod {
    pub fn to_openmpt_filter_length(self) -> i32 {
        match self {
            ResampleMethod::Nearest => 1,
            ResampleMethod::Linear => 2,
            ResampleMethod::Cubic => 4,
            ResampleMethod::Sinc => 8,
        }
    }
}

pub struct ExportOptions {
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_depth: u32,
    #[allow(dead_code)]
    pub opus_bitrate: u32,
    #[allow(dead_code)]
    pub vorbis_quality: u32,
    pub resample: ResampleMethod,
    pub stereo_separation: i32,
}

pub fn write_audio_file(samples: &[i16], filename: &str, options: &ExportOptions) -> Result<()> {
    match options.format {
        AudioFormat::Wav => write_wav_file(samples, filename, options),
        #[cfg(feature = "vorbis")]
        AudioFormat::Vorbis => write_vorbis_file(samples, filename, options),
        #[cfg(feature = "opus")]
        AudioFormat::Opus => write_opus_file(samples, filename, options),
        #[cfg(feature = "flac")]
        AudioFormat::Flac => write_flac_file(samples, filename, options),
    }
}

fn write_wav_file(samples: &[i16], filename: &str, options: &ExportOptions) -> Result<()> {
    let spec = WavSpec {
        channels: options.channels as u16,
        sample_rate: options.sample_rate,
        bits_per_sample: options.bit_depth as u16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(filename, spec)?;
    for &sample in samples {
        // If we want 24-bit, we need to shift. Hound's write_sample for i16 into 24-bit spec might need care.
        // Actually hound supports i32 for 24-bit.
        if options.bit_depth == 24 {
                          writer.write_sample((sample as i32) << 8)?;
             
        } else {
             writer.write_sample(sample)?;
        }
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(feature = "vorbis")]
fn write_vorbis_file(samples: &[i16], filename: &str, options: &ExportOptions) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(filename)?;
    // Placeholder for real vorbis encoding
    file.write_all(b"OggS")?;
    file.write_all(&options.sample_rate.to_le_bytes())?;
    file.write_all(&[options.vorbis_quality as u8])?;
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }
    Ok(())
}

#[cfg(feature = "opus")]
fn write_opus_file(samples: &[i16], filename: &str, options: &ExportOptions) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(filename)?;
    file.write_all(b"OpusHead")?;
    file.write_all(&[1, options.channels as u8])?;
    file.write_all(&0u16.to_le_bytes())?;
    file.write_all(&options.sample_rate.to_le_bytes())?;
    file.write_all(&0u16.to_le_bytes())?;
    file.write_all(&[0])?;
    // Bitrate would be used here in a real encoder
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }
    Ok(())
}

#[cfg(feature = "flac")]
fn write_flac_file(samples: &[i16], filename: &str, options: &ExportOptions) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    let mut file = File::create(filename)?;
    file.write_all(b"fLaC")?;
    file.write_all(&options.sample_rate.to_le_bytes())?;
    file.write_all(&[options.bit_depth as u8])?;
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }
    Ok(())
}
