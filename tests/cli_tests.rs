use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;
use std::fs;
use hound::WavReader;

#[test]
fn test_mod_extraction() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path);
    
    cmd.assert()
       .success()
       .stdout(predicate::str::contains("Extracting 31 sample stems"));

    let entries = fs::read_dir(out_path)?.count();
    assert_eq!(entries, 31);

    // Check one filename
    let expected_file = out_dir.path().join("cndmcrrp_sample_001.wav");
    assert!(expected_file.exists());

    // Basic format check: WAV files start with "RIFF"
    let content = fs::read(&expected_file)?;
    assert!(content.starts_with(b"RIFF"));

    // Verify WAV properties
    let reader = WavReader::open(expected_file)?;
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.bits_per_sample, 16);

    Ok(())
}

#[test]
fn test_s3m_extraction() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/nova.s3m")
       .arg("-o").arg(out_path);
    
    cmd.assert()
       .success()
       .stdout(predicate::str::contains("Extracting 31 sample stems"));

    let entries = fs::read_dir(out_path)?.count();
    assert_eq!(entries, 31);

    let expected_file = out_dir.path().join("nova_sample_001.wav");
    assert!(expected_file.exists());

    // Verify WAV properties
    let reader = WavReader::open(expected_file)?;
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.bits_per_sample, 16);

    Ok(())
}

#[test]
fn test_xm_extraction() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/zalza-karate_muffins.xm")
       .arg("-o").arg(out_path);
    
    cmd.assert()
       .success()
       .stdout(predicate::str::contains("Extracting 47 instrument stems"));

    let entries = fs::read_dir(out_path)?.count();
    assert_eq!(entries, 47);

    let expected_file = out_dir.path().join("zalza-karate_muffins_instrument_001.wav");
    assert!(expected_file.exists());

    // Verify WAV properties
    let reader = WavReader::open(expected_file)?;
    let spec = reader.spec();
    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.bits_per_sample, 16);

    Ok(())
}


#[test]
fn test_complex_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    // Use a small module to keep test fast
    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path)
       .arg("--sample-rate").arg("22050")
       .arg("--channels").arg("1")
       .arg("--resample").arg("nearest")
       .arg("--bit-depth").arg("24")
       .arg("--stereo-separation").arg("0");
    
    cmd.assert()
       .success();

    let expected_file = out_dir.path().join("cndmcrrp_sample_001.wav");
    assert!(expected_file.exists());

    // Basic format check: WAV files start with "RIFF"
    let content = fs::read(&expected_file)?;
    assert!(content.starts_with(b"RIFF"));

    // Verify WAV properties
    let reader = WavReader::open(expected_file)?;
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 22050);
    assert_eq!(spec.bits_per_sample, 24);

    Ok(())
}

#[test]
fn test_invalid_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    cmd.arg("-i").arg("non_existent_file.mod")
       .arg("-o").arg(".");
    
    cmd.assert()
       .failure();

    Ok(())
}

#[test]
#[cfg(feature = "vorbis")]
fn test_vorbis_format() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path)
       .arg("--format").arg("vorbis")
       .arg("--vorbis-quality").arg("7");
    
    cmd.assert()
       .success();

    let expected_file = out_dir.path().join("cndmcrrp_sample_001.ogg");
    assert!(expected_file.exists());

    // Basic format check: Ogg files start with "OggS"
    let content = fs::read(expected_file)?;
    assert!(content.starts_with(b"OggS"));

    Ok(())
}

#[test]
#[cfg(feature = "opus")]
fn test_opus_format() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path)
       .arg("--format").arg("opus")
       .arg("--sample-rate").arg("48000")
       .arg("--opus-bitrate").arg("64");
    
    cmd.assert()
       .success();

    let expected_file = out_dir.path().join("cndmcrrp_sample_001.opus");
    assert!(expected_file.exists());

    // Basic format check: Ogg-encapsulated Opus files start with "OggS"
    let content = fs::read(expected_file)?;
    assert!(content.starts_with(b"OggS"));
    // And should contain "OpusHead" early on
    assert!(content[..100].windows(8).any(|w| w == b"OpusHead"));

    Ok(())
}

#[test]
#[cfg(feature = "flac")]
fn test_flac_format() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path)
       .arg("--format").arg("flac");
    
    cmd.assert()
       .success();

    let expected_file = out_dir.path().join("cndmcrrp_sample_001.flac");
    assert!(expected_file.exists());

    // Basic format check: FLAC files start with "fLaC"
    let content = fs::read(expected_file)?;
    assert!(content.starts_with(b"fLaC"));

    Ok(())
}

#[test]
fn test_all_resample_methods() -> Result<(), Box<dyn std::error::Error>> {
    for method in &["nearest", "linear", "cubic", "sinc"] {
        let mut cmd = Command::cargo_bin("untracker")?;
        let out_dir = tempdir()?;
        let out_path = out_dir.path().to_str().unwrap();

        // Render only 1 stem to be fast
        // Wait, the app currently renders ALL stems. 
        // I should probably add an option to render only specific stems to speed up tests.
        // For now I'll just run it, but it will be slow.
        cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
           .arg("-o").arg(out_path)
           .arg("--resample").arg(method);
        
        cmd.assert().success();
    }
    Ok(())
}

#[test]
fn test_stereo_separation_options() -> Result<(), Box<dyn std::error::Error>> {
    for sep in &["0", "100", "200"] {
        let mut cmd = Command::cargo_bin("untracker")?;
        let out_dir = tempdir()?;
        let out_path = out_dir.path().to_str().unwrap();

        cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
           .arg("-o").arg(out_path)
           .arg("--stereo-separation").arg(sep);
        
        cmd.assert().success();
    }
    Ok(())
}

#[test]
fn test_invalid_channels() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(".")
       .arg("--channels").arg("3");
    
    cmd.assert().failure();
    Ok(())
}

#[test]
fn test_invalid_bit_depth() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(".")
       .arg("--bit-depth").arg("32");
    
    cmd.assert().failure();
    Ok(())
}

#[test]
fn test_parallel_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("untracker")?;
    let out_dir = tempdir()?;
    let out_path = out_dir.path().to_str().unwrap();

    cmd.arg("-i").arg("tests/modules/cndmcrrp.mod")
       .arg("-o").arg(out_path)
       .arg("--parallel");
    
    cmd.assert()
       .success()
       .stdout(predicate::str::contains("Extracting 31 sample stems"));

    let entries = fs::read_dir(out_path)?.count();
    assert_eq!(entries, 31);

    Ok(())
}
