use anyhow::{anyhow, Result};
use hound::{WavSpec, WavWriter};
use log::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy)]
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
    info!("Writing audio file: {} ({} samples, {}Hz)", filename, samples.len(), options.sample_rate);
    let result = match options.format {
        AudioFormat::Wav => write_wav_file(samples, filename, options),
        #[cfg(feature = "vorbis")]
        AudioFormat::Vorbis => write_vorbis_file(samples, filename, options),
        #[cfg(feature = "opus")]
        AudioFormat::Opus => write_opus_file(samples, filename, options),
        #[cfg(feature = "flac")]
        AudioFormat::Flac => write_flac_file(samples, filename, options),
    };
    
    match &result {
        Ok(_) => info!("Successfully wrote audio file: {}", filename),
        Err(e) => log::error!("Failed to write audio file {}: {}", filename, e),
    }
    
    result
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
    use ogg::{PacketWriteEndInfo, PacketWriter};
    use opus::{Application, Channels, Encoder};
    use std::fs::File;

    let channels = match options.channels {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        _ => return Err(anyhow!("Opus only supports 1 or 2 channels")),
    };

    // Opus supports 8, 12, 16, 24, or 48 kHz.
    let rate = options.sample_rate;
    if ![8000, 12000, 16000, 24000, 48000].contains(&rate) {
        return Err(anyhow!("Opus only supports 8, 12, 16, 24, or 48 kHz sample rates. Please use --sample-rate 48000."));
    }

    let mut encoder = Encoder::new(rate, channels, Application::Audio)?;
    encoder.set_bitrate(opus::Bitrate::Bits(options.opus_bitrate as i32 * 1000))?;

    let file = File::create(filename)?;
    let mut packet_writer = PacketWriter::new(file);

    let pre_skip = 312u64;

    // 1. OpusHead
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // version
    head.push(options.channels as u8);
    head.extend_from_slice(&(pre_skip as u16).to_le_bytes()); // pre-skip
    head.extend_from_slice(&rate.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // gain
    head.push(0); // mapping family

    packet_writer.write_packet(head, 0x01, PacketWriteEndInfo::EndPage, 0)?;

    // 2. OpusTags
    let mut tags = Vec::new();
    tags.extend_from_slice(b"OpusTags");
    let vendor = "untracker";
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor.as_bytes());
    tags.extend_from_slice(&0u32.to_le_bytes()); // user comment list length

    packet_writer.write_packet(tags, 0x01, PacketWriteEndInfo::EndPage, 0)?;

    // 3. Audio packets
    let frame_size = (rate / 50) as usize; // 20ms
    let samples_per_frame = frame_size * options.channels as usize;
    let granule_mult = 48000 / rate;

    let mut granule_pos = pre_skip;
    for chunk in samples.chunks(samples_per_frame) {
        let packet = if chunk.len() < samples_per_frame {
            let mut padded = chunk.to_vec();
            padded.resize(samples_per_frame, 0);
            encoder.encode_vec(&padded, 4000)?
        } else {
            encoder.encode_vec(chunk, 4000)?
        };
        granule_pos += (chunk.len() / options.channels as usize) as u64 * granule_mult as u64;
        packet_writer.write_packet(packet, 0x01, PacketWriteEndInfo::EndPage, granule_pos)?;
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
