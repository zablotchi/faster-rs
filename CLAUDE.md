# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

faster-rs is a Rust wrapper for Microsoft Research's FASTER concurrent key-value store. It uses a two-crate architecture with FFI bindings to the C++ FASTER library.

## System Dependencies

Before building, install required system libraries (Ubuntu):
```bash
sudo add-apt-repository -y ppa:ubuntu-toolchain-r/test
sudo apt update
sudo apt install -y g++-7 libaio-dev uuid-dev libtbb-dev
```

**Note**: The FASTER C++ library is automatically downloaded from https://github.com/zablotchi/FASTER.git during the first build. The build script will always pull the latest version from the `main` branch. To skip updates and use a cached version, set the environment variable `FASTER_NO_UPDATE=1`.

## Common Commands

### Building
```bash
cargo build                    # Build main crate
cargo build --examples         # Build all examples
cargo build --release          # Release build
```

### Testing
```bash
cargo test                     # Run all tests
cargo test -- --test-threads=1 # Run tests serially (for checkpoint tests)
```

### Running Examples
```bash
cargo run --example basic                          # Basic usage
cargo run --example custom_keys                    # Custom key types
cargo run --example custom_values                  # Custom value types with RMW
cargo run --example sum_store_single -- populate   # Checkpoint demo (populate)
cargo run --example sum_store_single -- recover <token>  # Checkpoint demo (recover)
cargo run --example sum_store_concurrent           # Multi-threaded usage
```

### Benchmarking
```bash
cd benchmark
cargo run --release -- help                        # See benchmark options
cargo run --release -- process-ycsb <input> <output>  # Process YCSB data
cargo run --release -- run <load_keys> <run_keys>  # Run benchmark
```

## Architecture

### Two-Crate Structure

1. **`faster-rs` (root crate)**: High-level Rust API wrapper
   - Provides safe, idiomatic Rust interface
   - Located in `src/`

2. **`libfaster-sys` (FFI crate)**: Low-level C bindings
   - Uses `bindgen` to generate FFI bindings from C header
   - Uses CMake to compile C++ FASTER library
   - Located in `libfaster-sys/`

### Key Modules

**`src/lib.rs`**: Core `FasterKv` type and all operations (upsert, read, rmw, delete, checkpoint, recover, session management)

**`src/builder.rs`**: `FasterKvBuilder` for configuring table size, log size, disk persistence, and mutable fraction

**`src/faster_traits.rs`**:
- `FasterKey`: Auto-implemented for any `Serialize + DeserializeOwned` type
- `FasterValue`: Auto-implemented for any `Serialize + DeserializeOwned` type
- `FasterRmw`: Trait for custom Read-Modify-Write logic
- `read_callback` and `rmw_callback`: C-to-Rust callback functions used at FFI boundary

**`src/impls.rs`**: Default `FasterRmw` implementations for primitive types and collections
- Numeric types: addition
- Bool/Char: replacement
- String/Vec: concatenation
- HashSet: union

**`src/util.rs`**: `CheckPoint` and `Recover` utility structs

**`libfaster-sys/build.rs`**: Critical build script that:
1. Automatically clones/updates FASTER C++ source from zablotchi/FASTER fork (main branch)
2. Runs bindgen on `FASTER/cc/src/core/faster-c.h`
3. Compiles C++ FASTER library via CMake
4. Links against static libfaster.a and system libraries (uuid, tbb, aio, stdc++fs)
5. Respects `FASTER_NO_UPDATE=1` environment variable to skip git pull (useful for offline builds)

### FFI Boundary Design

**Memory Management Pattern**:
- Rust serializes keys/values using bincode, transfers ownership to C++ via `mem::forget()`
- C++ calls back to `deallocate_vec()` (exported from Rust) to free memory
- Read/RMW operations use callbacks to pass data back to Rust

**Callback System**:
- `read_callback<T>`: C++ invokes with serialized value bytes; Rust deserializes and sends via mpsc channel
- `rmw_callback<T>`: C++ invokes with current value + modification; Rust calls user's `FasterRmw::rmw()` method and returns serialized result

**Serialization Strategy**: All keys/values must be `Serialize + DeserializeOwned`. Uses bincode for compact binary serialization.

### Checkpoint & Recovery (CPR)

FASTER uses **Concurrent Prefix Recovery** for fault tolerance:
- `checkpoint()` returns a UUID token representing the snapshot state
- Requires disk-backed store (use `FasterKvBuilder::with_disk()`)
- Three variants: full checkpoint, index-only, hybrid-log-only
- `recover(index_token, hybrid_log_token)` restores from tokens
- Returns session IDs to resume from with `continue_session()`
- **Guarantee**: If operation X is persisted, all operations before X are persisted

### Session Management (Thread Coordination)

Each thread must:
1. Call `start_session()` to register (returns session ID)
2. Perform operations with **monotonic serial numbers**
3. Periodically call `refresh()` to report progress
4. Call `complete_pending(true/false)` to drain pending operations
5. Call `stop_session()` when done

See `examples/sum_store_concurrent.rs` (7840 lines) for production multi-threaded patterns.

### Serial Numbers

All operations (read, upsert, rmw, delete) require a **monotonic serial number**:
- Forms the sequence of operations for CPR checkpointing
- Ensures consistency at checkpoint boundaries
- If persistence not needed, can use `1` for all operations (see `examples/basic.rs`)

## Test Organization

**`tests/basic_tests.rs`**: Core functionality (upsert, read, rmw, delete, custom types)

**`tests/checkpoint_tests.rs`**: Checkpoint/recovery scenarios, error conditions

**`tests/thread_tests.rs`**: Multi-threaded concurrency tests with proper session management

## Important Implementation Notes

- `FasterKv` is marked `unsafe impl Send + Sync` for thread-safety
- All write operations use `mem::forget()` to transfer ownership to C++
- Read operations return `(u8, Receiver<V>)` - status code + channel for result
- Status codes defined in `src/status.rs`: `OK`, `PENDING`, `NOT_FOUND`, etc.
- `Drop` implementation calls `faster_destroy()` to cleanup C++ resources
- Builder pattern validates configuration before constructing store
- In-memory stores (default) will error on checkpoint operations

## Build System Details

The build process is complex due to FFI:
1. `libfaster-sys/build.rs` runs first (dependency)
2. Automatically clones/updates FASTER C++ source from https://github.com/zablotchi/FASTER.git
3. Generates Rust bindings with bindgen (v0.69)
4. Compiles C++ with CMake (requires C++11)
5. Main crate links against static library
6. All FFI types live in `libfaster_sys::ffi` namespace

**FASTER Source Management**:
- First build: Clones FASTER from zablotchi fork (shallow clone, main branch)
- Subsequent builds: Automatically runs `git pull` to get latest changes
- Offline/pinned builds: Set `FASTER_NO_UPDATE=1` to skip git pull
- Clean build: `rm -rf libfaster-sys/FASTER && cargo clean && cargo build`

**Why the zablotchi fork?** The fork includes critical patches for modern toolchains:
- C++17 compatibility (upgraded from C++14)
- GoogleTest main branch support (was master)
- Namespace fixes for C FFI
- Benchmark enhancements with realistic key/value sizes

If build fails, check:
- Network connectivity (for first build or updates)
- System dependencies installed (g++-7, libaio-dev, uuid-dev, libtbb-dev)
- Git is installed and accessible
- CMake and bindgen versions compatible
