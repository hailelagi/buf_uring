use std::error::Error;
use std::ffi::CString;
use std::sync::Arc;
use xnvme_sys::*;
use io_uring::{IoUring, Submitter, opcode};

const QUEUE_DEPTH: u32 = 32;
const NUM_WRITES: usize = 100;

struct IoContext {
    dev: *mut xnvme_dev_t,
    ring: IoUring,
}

impl Drop for IoContext {
    fn drop(&mut self) {
        unsafe {
            xnvme_dev_close(self.dev);
        }
    }
}

unsafe impl Send for IoContext {}
unsafe impl Sync for IoContext {}

fn main() -> Result<(), Box<dyn Error>> {
    let dev_path = CString::new("/dev/nvme0n1")?;
    let opts = std::ptr::null();
    let dev = unsafe { xnvme_dev_open(dev_path.as_ptr(), opts) };
    if dev.is_null() {
        return Err("Failed to open device".into());
    }

    let ring = IoUring::new(QUEUE_DEPTH)?;
    
    let ctx = Arc::new(IoContext {
        dev,
        ring,
    });

    let data = b"Hello from io_uring!".repeat(256); // Make it block-sized
    let data_ptr = data.as_ptr();

    let mut completion_count = 0;
    let submitter = ctx.ring.submitter();

    for i in 0..NUM_WRITES {
        let write_op = unsafe {
            let mut cmd = xnvme_cmd_ctx_t::default();
            
            xnvme_cmd_ctx_write(&mut cmd, data_ptr as *const _, i as u64, 1);
        };

        // Add write operation to submission queue
        unsafe {
            submitter.push(&write_op)?;
        }
    }

    // Submit all queued operations
    submitter.submit_and_wait(NUM_WRITES)?;

    // Process completions
    let mut cq = ctx.ring.completion();
    while completion_count < NUM_WRITES {
        match cq.next() {
            Some(cqe) => {
                if cqe.result() < 0 {
                    println!("Write failed for operation {}: {}", cqe.user_data(), cqe.result());
                } else {
                    completion_count += 1;
                }
            }
            None => continue,
        }
    }

    println!("Successfully completed {} write operations", completion_count);
    
    Ok(())
}
