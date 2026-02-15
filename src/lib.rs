pub mod audio;

use anyhow::{anyhow, Result};
pub use audio::{AudioFormat, ExportOptions, ResampleMethod, write_audio_file};
use openmpt::ext::ModuleExt;
use openmpt::module::Logger;

pub fn render_stem(
    buffer: &[u8],
    index: i32,
    is_instrument: bool,
    output_dir: &str,
    base_name: &str,
    options: &ExportOptions,
) -> Result<()> {
    let mut options = *options;
    #[cfg(feature = "opus")]
    if options.format == AudioFormat::Opus && ![8000, 12000, 16000, 24000, 48000].contains(&options.sample_rate) {
        options.sample_rate = 48000;
    }

    let type_label = if is_instrument { "instrument" } else { "sample" };
    println!("  Rendering {} {}...", type_label, index + 1);
    
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
        output_dir, base_name, type_label, index + 1, ext_str
    );

    let mut samples = vec![0i16; 8192];
    let mut all_audio = Vec::new();

    loop {
        let rendered = if options.channels == 2 {
            module_ext.read_interleaved_stereo(options.sample_rate as i32, &mut samples)
        } else {
            module.read_mono(options.sample_rate as i32, &mut samples[..4096])
        };

        if rendered == 0 { break; }
        
        let num_samples_to_copy = rendered * (options.channels as usize);
        all_audio.extend_from_slice(&samples[..num_samples_to_copy]);
        
        if module_ext.get_position_seconds() >= module_ext.get_duration_seconds() {
            break;
        }
    }

    write_audio_file(&all_audio, &output_path, &options)?;
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
        assert!( "wav".parse::<AudioFormat>().is_ok());
        assert!( "WAV".parse::<AudioFormat>().is_ok());
        #[cfg(feature = "vorbis")]
        {
            assert!( "vorbis".parse::<AudioFormat>().is_ok());
            assert!( "ogg".parse::<AudioFormat>().is_ok());
        }
        #[cfg(feature = "opus")]
        assert!( "opus".parse::<AudioFormat>().is_ok());
        #[cfg(feature = "flac")]
        assert!( "flac".parse::<AudioFormat>().is_ok());
        assert!( "invalid".parse::<AudioFormat>().is_err());
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
        let result = render_stem(&[], 0, false, ".", "test", &options);
        assert!(result.is_err());
    }
}
