## 2025-05-22 - Missing Input Validation and Potential DoS

**Vulnerability:**
1. Lack of validation for `sample_rate` and `stereo_separation` allows potentially invalid or extreme values to be passed to the underlying C library, which could lead to unexpected behavior or crashes.
2. `read_file_to_buffer` reads the entire input file into memory without any size limit, allowing for a Denial of Service (DoS) attack via Out of Memory (OOM) if a very large file is provided.
3. Path construction for output files uses string concatenation, which can lead to writing to the root directory if the output directory is empty.

**Learning:**
The application prioritizes functionality and assumes well-behaved input. In a security-conscious environment, all user-provided parameters must be validated against reasonable bounds, and resource consumption (like memory) should be capped.

**Prevention:**
- Always validate numeric inputs against supported ranges.
- Check file sizes before reading them into memory.
- Use platform-aware path manipulation libraries (like `std::path::PathBuf` in Rust) instead of string concatenation for file paths.
