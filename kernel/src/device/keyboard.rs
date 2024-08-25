use crossbeam_queue::ArrayQueue;
use spin::Lazy;

const SCANCODE_QUEUE_SIZE: usize = 128;

static SCANCODE_QUEUE: Lazy<ArrayQueue<u8>> = Lazy::new(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE));

pub fn add_scancode(scancode: u8) {
    if let Err(_) = SCANCODE_QUEUE.push(scancode) {
        log::warn!("Scancode queue full, dropping keyboard input!");
    }
}

/// Return the scan code of the keyboard buffer, returns None is the buffer is empty.
pub fn get_scancode() -> Option<u8> {
    SCANCODE_QUEUE.pop()
}

/// Return whether the keyboard buffer is empty.
pub fn has_scancode() -> bool {
    !SCANCODE_QUEUE.is_empty()
}
