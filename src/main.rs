use clap::Parser;
use crossbeam::sync::WaitGroup;
use memmap2::MmapOptions;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of files to create
    #[arg(short = 'f', long, default_value_t = 10)]
    num_files: usize,

    /// Size of each file in bytes
    #[arg(short = 's', long, default_value_t = 1024 * 1024)]
    file_size: usize,

    /// Waiting time after file creation in seconds
    #[arg(short = 'w', long, default_value_t = 1)]
    wait_time: u64,

    /// Number of threads for read operations
    #[arg(short = 't', long, default_value_t = 4)]
    num_threads: usize,

    /// Random seed for reproducible experiments
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Size of blocks to read in bytes
    #[arg(short = 'b', long, default_value_t = 4096)]
    block_size: usize,

    /// Number of read operations to perform
    #[arg(short = 'n', long, default_value_t = 1000)]
    num_operations: usize,

    /// Use memory-mapped files instead of standard I/O
    #[arg(short = 'm', long)]
    use_mmap: bool,

    /// Prefix for test files
    #[arg(long, default_value = "testfile")]
    file_prefix: String,
}

#[derive(Debug, Clone)]
struct ReadResult {
    latency: Duration,
    is_first_read: bool,
}

#[derive(Debug)]
struct Statistics {
    count: usize,
    avg: Duration,
    median: Duration,
    p90: Duration,
    p95: Duration,
    p99: Duration,
    min: Duration,
    max: Duration,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🚀 Random I/O Tester Starting...");
    println!("Configuration:");
    println!("  Files: {} × {} bytes", args.num_files, args.file_size);
    println!("  Threads: {}", args.num_threads);
    println!("  Block size: {} bytes", args.block_size);
    println!("  Operations: {}", args.num_operations);
    println!("  Mode: {}", if args.use_mmap { "Memory-mapped" } else { "Standard I/O" });
    println!("  Seed: {}", args.seed);
    println!();

    // Phase 1: Create test files
    println!("📝 Creating test files...");
    let file_paths = create_test_files(&args)?;
    println!("✅ Created {} files", file_paths.len());

    // Phase 2: Wait
    println!("⏳ Waiting {} seconds...", args.wait_time);
    std::thread::sleep(Duration::from_secs(args.wait_time));

    // Phase 3: Run performance tests
    println!("🔬 Running performance tests...");
    let results = if args.use_mmap {
        run_mmap_tests(&args, &file_paths)?
    } else {
        run_standard_io_tests(&args, &file_paths)?
    };

    // Phase 4: Analyze and report results
    println!("\n📊 Performance Results:");
    analyze_and_report_results(results);

    // Cleanup
    cleanup_test_files(&file_paths)?;
    println!("\n🧹 Cleaned up test files");

    Ok(())
}

fn create_test_files(args: &Args) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file_paths = Vec::new();
    
    // Create test data buffer
    let test_data = vec![0xAB; args.file_size];
    
    for i in 0..args.num_files {
        let file_path = format!("{}_{}.dat", args.file_prefix, i);
        let mut file = File::create(&file_path)?;
        file.write_all(&test_data)?;
        file.sync_all()?;
        file_paths.push(file_path);
    }
    
    Ok(file_paths)
}

fn run_standard_io_tests(args: &Args, file_paths: &[String]) -> Result<Vec<ReadResult>, Box<dyn std::error::Error>> {
    let results = Arc::new(Mutex::new(Vec::new()));
    let read_blocks = Arc::new(Mutex::new(HashSet::new()));
    
    // Prepare random operations for each thread
    let operations_per_thread = args.num_operations / args.num_threads;
    let remainder = args.num_operations % args.num_threads;
    
    let wg = WaitGroup::new();
    
    for thread_id in 0..args.num_threads {
        let thread_operations = operations_per_thread + if thread_id < remainder { 1 } else { 0 };
        let results_clone = Arc::clone(&results);
        let read_blocks_clone = Arc::clone(&read_blocks);
        let file_paths_clone = file_paths.to_vec();
        let args_clone = args.clone();
        let wg_clone = wg.clone();
        
        std::thread::spawn(move || {
            let _guard = wg_clone;
            
            // Create thread-specific RNG with derived seed
            let mut rng = StdRng::seed_from_u64(args_clone.seed + thread_id as u64);
            let mut thread_results = Vec::new();
            
            for _ in 0..thread_operations {
                // Select random file
                let file_index = rng.gen_range(0..file_paths_clone.len());
                let file_path = &file_paths_clone[file_index];
                
                // Calculate random block position
                let max_blocks = args_clone.file_size / args_clone.block_size;
                if max_blocks == 0 { continue; }
                
                let block_index = rng.gen_range(0..max_blocks);
                let offset = block_index * args_clone.block_size;
                
                // Check if this block has been read before
                let is_first_read = {
                    let mut blocks = read_blocks_clone.lock().unwrap();
                    blocks.insert(format!("{}:{}", file_index, block_index))
                };
                
                // Perform the read operation
                let start = Instant::now();
                let result = perform_standard_read(file_path, offset, args_clone.block_size);
                let latency = start.elapsed();
                
                if result.is_ok() {
                    thread_results.push(ReadResult {
                        latency,
                        is_first_read,
                    });
                }
            }
            
            // Add thread results to global results
            {
                let mut global_results = results_clone.lock().unwrap();
                global_results.extend(thread_results);
            }
        });
    }
    
    // Wait for all threads to complete
    wg.wait();
    
    let results = results.lock().unwrap();
    Ok(results.clone())
}

fn perform_standard_read(file_path: &str, offset: usize, block_size: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(file_path)?;
    file.seek(SeekFrom::Start(offset as u64))?;
    
    let mut buffer = vec![0u8; block_size];
    file.read_exact(&mut buffer)?;
    
    Ok(buffer)
}

fn run_mmap_tests(args: &Args, file_paths: &[String]) -> Result<Vec<ReadResult>, Box<dyn std::error::Error>> {
    // Memory map all files first
    let mut mmaps = Vec::new();
    for file_path in file_paths {
        let file = File::open(file_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        mmaps.push(mmap);
    }
    
    let results = Arc::new(Mutex::new(Vec::new()));
    let read_blocks = Arc::new(Mutex::new(HashSet::new()));
    let mmaps = Arc::new(mmaps);
    
    // Prepare random operations for each thread
    let operations_per_thread = args.num_operations / args.num_threads;
    let remainder = args.num_operations % args.num_threads;
    
    let wg = WaitGroup::new();
    
    for thread_id in 0..args.num_threads {
        let thread_operations = operations_per_thread + if thread_id < remainder { 1 } else { 0 };
        let results_clone = Arc::clone(&results);
        let read_blocks_clone = Arc::clone(&read_blocks);
        let mmaps_clone = Arc::clone(&mmaps);
        let args_clone = args.clone();
        let wg_clone = wg.clone();
        
        std::thread::spawn(move || {
            let _guard = wg_clone;
            
            // Create thread-specific RNG with derived seed
            let mut rng = StdRng::seed_from_u64(args_clone.seed + thread_id as u64);
            let mut thread_results = Vec::new();
            
            for _ in 0..thread_operations {
                // Select random file
                let file_index = rng.gen_range(0..mmaps_clone.len());
                
                // Calculate random block position
                let max_blocks = args_clone.file_size / args_clone.block_size;
                if max_blocks == 0 { continue; }
                
                let block_index = rng.gen_range(0..max_blocks);
                let offset = block_index * args_clone.block_size;
                
                // Check if this block has been read before
                let is_first_read = {
                    let mut blocks = read_blocks_clone.lock().unwrap();
                    blocks.insert(format!("{}:{}", file_index, block_index))
                };
                
                // Perform the memory access
                let start = Instant::now();
                let result = perform_mmap_read(&mmaps_clone[file_index], offset, args_clone.block_size);
                let latency = start.elapsed();
                
                if result.is_ok() {
                    thread_results.push(ReadResult {
                        latency,
                        is_first_read,
                    });
                }
            }
            
            // Add thread results to global results
            {
                let mut global_results = results_clone.lock().unwrap();
                global_results.extend(thread_results);
            }
        });
    }
    
    // Wait for all threads to complete
    wg.wait();
    
    let results = results.lock().unwrap();
    Ok(results.clone())
}

fn perform_mmap_read(mmap: &memmap2::Mmap, offset: usize, block_size: usize) -> Result<Vec<u8>, std::io::Error> {
    if offset + block_size > mmap.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Read beyond file bounds",
        ));
    }
    
    // Force memory access by copying the data
    let data = &mmap[offset..offset + block_size];
    Ok(data.to_vec())
}

fn analyze_and_report_results(results: Vec<ReadResult>) {
    if results.is_empty() {
        println!("❌ No results to analyze");
        return;
    }
    
    let all_results = &results;
    let first_reads: Vec<_> = results.iter().filter(|r| r.is_first_read).collect();
    let repeated_reads: Vec<_> = results.iter().filter(|r| !r.is_first_read).collect();
    
    println!("\n📈 All Reads ({} operations):", all_results.len());
    print_statistics(calculate_statistics(all_results.iter().map(|r| &r.latency).collect()));
    
    if !first_reads.is_empty() {
        println!("\n🆕 First Reads ({} operations):", first_reads.len());
        print_statistics(calculate_statistics(first_reads.iter().map(|r| &r.latency).collect()));
    }
    
    if !repeated_reads.is_empty() {
        println!("\n🔄 Repeated Reads ({} operations):", repeated_reads.len());
        print_statistics(calculate_statistics(repeated_reads.iter().map(|r| &r.latency).collect()));
    }
}

fn calculate_statistics(latencies: Vec<&Duration>) -> Statistics {
    if latencies.is_empty() {
        return Statistics {
            count: 0,
            avg: Duration::ZERO,
            median: Duration::ZERO,
            p90: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            min: Duration::ZERO,
            max: Duration::ZERO,
        };
    }
    
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort();
    
    let count = sorted_latencies.len();
    let sum: Duration = sorted_latencies.iter().map(|&d| *d).sum();
    let avg = sum / count as u32;
    
    let median = *sorted_latencies[count / 2];
    let p90 = *sorted_latencies[((count as f64) * 0.90) as usize];
    let p95 = *sorted_latencies[((count as f64) * 0.95) as usize];
    let p99 = *sorted_latencies[((count as f64) * 0.99) as usize];
    let min = **sorted_latencies.first().unwrap();
    let max = **sorted_latencies.last().unwrap();
    
    Statistics {
        count,
        avg,
        median,
        p90,
        p95,
        p99,
        min,
        max,
    }
}

fn print_statistics(stats: Statistics) {
    println!("  Count:     {}", stats.count);
    println!("  Average:   {:.2}μs", stats.avg.as_micros());
    println!("  Median:    {:.2}μs", stats.median.as_micros());
    println!("  90th %ile: {:.2}μs", stats.p90.as_micros());
    println!("  95th %ile: {:.2}μs", stats.p95.as_micros());
    println!("  99th %ile: {:.2}μs", stats.p99.as_micros());
    println!("  Min:       {:.2}μs", stats.min.as_micros());
    println!("  Max:       {:.2}μs", stats.max.as_micros());
}

fn cleanup_test_files(file_paths: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    for file_path in file_paths {
        if Path::new(file_path).exists() {
            std::fs::remove_file(file_path)?;
        }
    }
    Ok(())
}
