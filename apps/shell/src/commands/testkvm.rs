use alloc::{string::String, vec::Vec};
use vstd::fs::{ioctl, open};

pub fn testkvm(_args: Vec<String>) {
    let code: [u8; 12] = [
        0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
        0x00, 0xd8, /* add %bl, %al */
        0x04, b'0', /* add $'0', %al */
        0xee, /* out %al, (%dx) */
        0xb0, b'\n', /* mov $'\n', %al */
        0xee,  /* out %al, (%dx) */
        0xf4,  /* hlt */
    ];

    let addr = alloc::vec![0u8; 12].leak();
    addr.copy_from_slice(&code);

    let kvm_fd = open(String::from("/dev/kvm"), vstd::fs::OpenMode::Read);
    ioctl(
        kvm_fd,
        vstd::KvmDevIoctlCommand::KvmRun as usize,
        addr.as_ptr() as usize,
    );
}
