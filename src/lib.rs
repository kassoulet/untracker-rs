pub mod audio;

use anyhow::{anyhow, Result};
pub use audio::{write_audio_file, AudioFormat, ExportOptions, ResampleMethod};
use openmpt::ext::ModuleExt;
use openmpt::module::Logger;

use indicatif::ProgressBar;

pub fn render_stem(
    buffer: &[u8],
    index: i32,
    is_instrument: bool,
    output_dir: &str,
    base_name: &str,
    options: &ExportOptions,
    progress_bar: Option<&ProgressBar>,
) -> Result<()> {
    let options = *options;
    #[cfg(feature = "opus")]
    let options = if options.format == AudioFormat::Opus
        && ![8000, 12000, 16000, 24000, 48000].contains(&options.sample_rate)
    {
        ExportOptions {
            sample_rate: 48000,
            ..options
        }
    } else {
        options
    };

    let type_label = if is_instrument {
        "instrument"
    } else {
        "sample"
    };

    if let Some(pb) = progress_bar {
        pb.set_message(format!("Rendering {} {}...", type_label, index + 1));
    } else if cfg!(test) {
        // Only print to stdout in test mode for compatibility
        println!("  Rendering {} {}...", type_label, index + 1);
    }
    // In non-test mode with no progress bar, don't print individual messages to avoid console spam

    log::info!(
        "Starting to render {} {} to {}",
        type_label,
        index + 1,
        output_dir
    );

    let module_ext = ModuleExt::from_memory(buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to re-load module for rendering"))?;

    let interactive = module_ext
        .get_interactive_interface()
        .ok_or_else(|| anyhow!("Interactive interface not available"))?;

    let mut module = module_ext.get_module();

    // Configure render parameters
    module.set_render_interpolation_filter_length(options.resample.to_openmpt_filter_length());
    module.set_render_stereo_separation(options.stereo_separation);

    let count = if is_instrument {
        module.get_num_instruments()
    } else {
        module.get_num_samples()
    };

    // Mute everything except the target
    for i in 0..count {
        interactive.set_instrument_mute_status(&module_ext, i, i != index);
    }

    let ext_str = match options.format {
        AudioFormat::Wav => "wav",
        #[cfg(feature = "vorbis")]
        AudioFormat::Vorbis => "ogg",
        #[cfg(feature = "opus")]
        AudioFormat::Opus => "opus",
        #[cfg(feature = "flac")]
        AudioFormat::Flac => "flac",
    };

    let output_path = format!(
        "{}/{}_{}_{:03}.{}",
        output_dir,
        base_name,
        type_label,
        index + 1,
        ext_str
    );

    log::debug!("Writing to: {}", output_path);

    let mut samples = vec![0i16; 8192];
    let mut all_audio = Vec::new();

    // Calculate total duration for progress tracking
    let total_duration = module_ext.get_duration_seconds();
    let mut last_percentage = 0.0;

    loop {
        let rendered = if options.channels == 2 {
            module_ext.read_interleaved_stereo(options.sample_rate as i32, &mut samples)
        } else {
            module.read_mono(options.sample_rate as i32, &mut samples[..4096])
        };

        if rendered == 0 {
            break;
        }

        let num_samples_to_copy = rendered * (options.channels as usize);
        all_audio.extend_from_slice(&samples[..num_samples_to_copy]);

        let current_position = module_ext.get_position_seconds();
        let percentage = if total_duration > 0.0 {
            (current_position / total_duration) * 100.0
        } else {
            0.0
        };

        if let Some(pb) = progress_bar {
            // Update progress bar with percentage
            let rounded_percentage = (percentage as u64).min(100);
            if rounded_percentage > last_percentage as u64 {
                last_percentage = rounded_percentage as f64;
                pb.set_message(format!(
                    "{} {} - {:.1}% complete",
                    type_label,
                    index + 1,
                    percentage
                ));
            }
        }

        if current_position >= total_duration {
            break;
        }
    }

    write_audio_file(&all_audio, &output_path, &options)?;
    log::info!(
        "Successfully rendered {} {} to {}",
        type_label,
        index + 1,
        output_path
    );

    if !cfg!(test) {
        if let Some(pb) = progress_bar {
            // Clear the progress bar line and print completed stem
            pb.println(format!("  Extracted {}", output_path));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_method_mapping() {
        assert_eq!(ResampleMethod::Nearest.to_openmpt_filter_length(), 1);
        assert_eq!(ResampleMethod::Linear.to_openmpt_filter_length(), 2);
        assert_eq!(ResampleMethod::Cubic.to_openmpt_filter_length(), 4);
        assert_eq!(ResampleMethod::Sinc.to_openmpt_filter_length(), 8);
    }

    #[test]
    fn test_audio_format_parsing() {
        assert!("wav".parse::<AudioFormat>().is_ok());
        assert!("WAV".parse::<AudioFormat>().is_ok());
        #[cfg(feature = "vorbis")]
        {
            assert!("vorbis".parse::<AudioFormat>().is_ok());
            assert!("ogg".parse::<AudioFormat>().is_ok());
        }
        #[cfg(feature = "opus")]
        assert!("opus".parse::<AudioFormat>().is_ok());
        #[cfg(feature = "flac")]
        assert!("flac".parse::<AudioFormat>().is_ok());
        assert!("invalid".parse::<AudioFormat>().is_err());
    }

    #[test]
    fn test_export_options_struct() {
        let options = ExportOptions {
            format: AudioFormat::Wav,
            sample_rate: 44100,
            channels: 2,
            bit_depth: 16,
            opus_bitrate: 128,
            vorbis_quality: 5,
            resample: ResampleMethod::Sinc,
            stereo_separation: 100,
        };
        assert_eq!(options.sample_rate, 44100);
        assert_eq!(options.channels, 2);
    }

    #[test]
    fn test_render_stem_invalid_buffer() {
        let options = ExportOptions {
            format: AudioFormat::Wav,
            sample_rate: 44100,
            channels: 2,
            bit_depth: 16,
            opus_bitrate: 128,
            vorbis_quality: 5,
            resample: ResampleMethod::Sinc,
            stereo_separation: 100,
        };
        let result = render_stem(&[], 0, false, ".", "test", &options, None);
        assert!(result.is_err());
    }
}
