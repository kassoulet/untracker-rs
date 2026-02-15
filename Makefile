.PHONY: all static clean help

help:
	@echo "Available targets:"
	@echo "  all     - Build in release mode (dynamic linking by default)"
	@echo "  static  - Build with libopenmpt linked statically"
	@echo "  musl    - Build a fully static binary using musl (requires musl-tools)"
	@echo "  clean   - Clean build artifacts"

all:
	cargo build --release

static:
	# Build libopenmpt statically with local dependencies to include them in the .a
	$(MAKE) -j$(shell nproc) -C ../openmpt STATIC_LIB=1 SHARED_LIB=0 LOCAL_ZLIB=1 LOCAL_MPG123=1 LOCAL_OGG=1 LOCAL_VORBIS=1 TEST=0 EXAMPLES=0 OPENMPT123=0
	# Build untracker statically. We use .cargo/config.toml for target-specific flags
	LIBOPENMPT_STATIC=1 LIBOPENMPT_LIB_DIR=$(shell pwd)/../openmpt/bin \
	cargo build --release --features all_formats --target x86_64-unknown-linux-gnu
	@echo "Binary generated at target/x86_64-unknown-linux-gnu/release/untracker"
	@echo "Checking dynamic dependencies:"
	@ldd target/x86_64-unknown-linux-gnu/release/untracker || echo "Statically linked"

musl:
	# Check for musl-gcc
	@which musl-gcc > /dev/null || (echo "musl-gcc not found. Please install musl-tools." && exit 1)
	# Build libopenmpt for musl
	$(MAKE) -j$(shell nproc) -C ../openmpt clean
	# We use musl-gcc but libopenmpt is C++, so we need to be careful.
	# We'll try to build with musl-gcc but force static linking.
	$(MAKE) -j$(shell nproc) -C ../openmpt STATIC_LIB=1 SHARED_LIB=0 LOCAL_ZLIB=1 LOCAL_MPG123=1 LOCAL_OGG=1 LOCAL_VORBIS=1 TEST=0 EXAMPLES=0 OPENMPT123=0 \
		CC=musl-gcc CXX="g++ -static-libstdc++ -static-libgcc"
	# Build untracker with musl target. We point to a libstdc++.a location
	LIBOPENMPT_STATIC=1 LIBOPENMPT_LIB_DIR=$(shell pwd)/../openmpt/bin \
	RUSTFLAGS="-L native=/usr/lib/gcc/x86_64-linux-gnu/13" \
	cargo build --release --features all_formats --target x86_64-unknown-linux-musl
	@echo "Fully static binary should be at target/x86_64-unknown-linux-musl/release/untracker"
	@echo "Checking dynamic dependencies:"
	@ldd target/x86_64-unknown-linux-musl/release/untracker || echo "Statically linked"

clean:
	cargo clean
	$(MAKE) -j$(shell nproc) -C ../openmpt clean
