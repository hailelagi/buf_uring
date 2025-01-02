use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use io_uring::{IoUring, opcode, types};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Barrier};
use std::thread;
use rand::prelude::*;

const KB: usize = 1024;
const MB: usize = 1024 * KB;
const BUFFER_SIZE: usize = 4 * KB;

struct Config {
    block_size: usize,
    file_size: usize,
    is_random: bool,
    queue_depth: u32,
    num_workers: usize,
}

fn io_uring_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_uring_write");
    
    let configs = vec![
        Config {
            block_size: 4 * KB,
            file_size: 100 * MB,
            is_random: false,
            queue_depth: 32,
            num_workers: 4,
        },
        Config {
            block_size: 64 * KB,
            file_size: 100 * MB,
            is_random: true,
            queue_depth: 64,
            num_workers: 8,
        },
    ];

    for config in &configs {
        let buffer = Arc::new(vec![0u8; config.file_size]);
        thread_rng().fill(&mut buffer[..]);

        group.throughput(Throughput::Bytes(config.file_size as u64));
        
        let id = BenchmarkId::new(
            format!("workers_{}_qd_{}", config.num_workers, config.queue_depth),
            format!("{}KB", config.block_size / KB)
        );

        group.bench_function(id, |b| {
            b.iter_custom(|iters| {
                let mut total_time = std::time::Duration::ZERO;
                
                for _ in 0..iters {
                    let file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open("/dev/random")
                        .unwrap();
                    
                    let start = std::time::Instant::now();
                    run_io_uring_workers(
                        &file,
                        Arc::clone(&buffer),
                        config.block_size,
                        config.queue_depth,
                        config.num_workers,
                        config.is_random,
                    );
                    total_time += start.elapsed();
                    
                    std::fs::remove_file("/dev/random").unwrap();
                }
                total_time
            });
        });
    }
    group.finish();
}

fn io_uring_and_worker_pool(direct_io: bool, data: &[u8], entries: usize, workers: usize) {
    let name = format!("{}io+k_pwrite_{}_entries", workers, entries);
    
    let wg = WaitGroup::new();
    let work_size = data.len() / workers;
    let data = Arc::new(data.to_vec());
    
    let file = File::create("test.data").unwrap();
    let fd = file.as_raw_fd();

    for i in (0..data.len()).step_by(work_size) {
        let wg = wg.clone();
        let data = Arc::clone(&data);
        
        thread::spawn(move || {
            let mut ring = IoUring::new(entries as u32).unwrap();
            
            let mut i = i;
            while i < i + work_size {
                let mut submitted_entries = 0;
                
                for k in 0..entries {
                    let base = i + k * BUFFER_SIZE;
                    if base >= i + work_size || base >= data.len() {
                        break;
                    }
                    
                    submitted_entries += 1;
                    let size = std::cmp::min(
                        std::cmp::min(BUFFER_SIZE, (i + work_size) - base),
                        data.len() - base
                    );
                    
                    let write_op = opcode::Write::new(
                        fd,
                        data[base..base + size].as_ptr(),
                        size as u32
                    )
                    .offset(base as u64)
                    .build();
                    
                    unsafe {
                        ring.submission()
                            .push(&write_op)
                            .expect("submission queue is full");
                    }
                }
                
                if submitted_entries == 0 {
                    continue;
                }
                
                ring.submit_and_wait(submitted_entries).unwrap();
                
                while let Some(cqe) = ring.completion().next() {
                    assert_eq!(cqe.result(), BUFFER_SIZE as i32);
                }
                
                i += BUFFER_SIZE * entries;
            }
            drop(wg);
        });
    }
    
    wg.wait();
}

criterion_group!(benches, io_uring_write_benchmark);
criterion_main!(benches);
