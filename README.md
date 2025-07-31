# Random I/O Tester

A comprehensive disk performance testing tool written in Rust that measures random read performance using both standard I/O and memory-mapped files.

## Features

- **Configurable test parameters**: File count, file size, block size, operation count, thread count
- **Two testing modes**: Standard I/O (seek+read) vs Memory-mapped files
- **Multi-threaded testing**: Configurable number of concurrent threads
- **Reproducible experiments**: Pseudo-random with configurable seed
- **First vs Repeated read tracking**: Distinguishes cache effects
- **Comprehensive statistics**: Average, median, 90th/95th/99th percentiles, min/max
- **Automatic cleanup**: Test files are removed after completion

## Usage

### Basic Example
```bash
# Test with defaults (10 files, 1MB each, 1000 operations, 4 threads)
cargo run

# Small quick test
cargo run -- -f 3 -s 32768 -n 100 -t 2

# Large scale test
cargo run -- -f 50 -s 10485760 -n 10000 -t 8
```

### Testing Standard I/O vs Memory-Mapped Performance
```bash
# Standard I/O mode
cargo run -- -f 10 -s 1048576 -n 1000 -t 4 --seed 42

# Memory-mapped mode (same parameters for comparison)
cargo run -- -f 10 -s 1048576 -n 1000 -t 4 --seed 42 -m
```

### Advanced Configuration
```bash
cargo run -- \
  --num-files 20 \
  --file-size 5242880 \
  --block-size 8192 \
  --num-operations 5000 \
  --num-threads 8 \
  --wait-time 5 \
  --seed 123 \
  --use-mmap \
  --file-prefix "perftest"
```

## Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--num-files` | `-f` | Number of test files to create | 10 |
| `--file-size` | `-s` | Size of each file in bytes | 1048576 (1MB) |
| `--wait-time` | `-w` | Wait time after file creation (seconds) | 1 |
| `--num-threads` | `-t` | Number of concurrent threads | 4 |
| `--seed` | | Random seed for reproducibility | 42 |
| `--block-size` | `-b` | Size of blocks to read (bytes) | 4096 |
| `--num-operations` | `-n` | Total number of read operations | 1000 |
| `--use-mmap` | `-m` | Use memory-mapped files | false |
| `--file-prefix` | | Prefix for test file names | "testfile" |

## Output Interpretation

The tool reports three sets of statistics:

1. **All Reads**: Combined statistics for all operations
2. **First Reads**: Blocks read for the first time (likely from disk)  
3. **Repeated Reads**: Blocks read again (likely from cache)

### Metrics Explained
- **Average/Median**: Central tendency of latencies
- **90th/95th/99th percentile**: Tail latency performance  
- **Min/Max**: Best and worst case performance
- **Count**: Number of operations in each category

## Example Output
```
📊 Performance Results:

📈 All Reads (1000 operations):
  Count:     1000
  Average:   15.23μs
  Median:    12.45μs
  90th %ile: 28.67μs
  95th %ile: 45.23μs
  99th %ile: 89.12μs
  Min:       3.21μs
  Max:       156.78μs

🆕 First Reads (842 operations):
  Count:     842
  Average:   16.78μs
  [... additional stats ...]

🔄 Repeated Reads (158 operations):  
  Count:     158
  Average:   8.43μs
  [... additional stats ...]
```

## Use Cases

- **Storage performance benchmarking**
- **Cache effectiveness analysis** 
- **Multi-threaded I/O scalability testing**
- **Standard I/O vs memory-mapped performance comparison**
- **Reproducible performance experiments**
- **Tail latency analysis**

## Building and Running

```bash
# Build the project
cargo build --release

# Run with optimizations
cargo run --release -- [OPTIONS]

# Get help
cargo run -- --help
```

If you are on Linux and want to build a static executable, you can try
this:

```bash
rustup target add x86_64-unknown-linux-musl
sudo apt-get install musl-tools
cargo build --target x86_64-unknown-linux-musl --release
```
