use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;

pub trait BlockDevice: Send + Sync + 'static {
    fn read_block(&self, start_sec: usize, buf: &mut [u8]) -> Option<()>;
    fn write_block(&self, start_sec: usize, buf: &[u8]) -> Option<()>;

    fn get_size(&self) -> usize;
}

struct AHCIDisk {
    num: usize,
}

impl BlockDevice for AHCIDisk {
    fn read_block(&self, start_sec: usize, buf: &mut [u8]) -> Option<()> {
        super::ahci::AHCI
            .lock()
            .get_disk(self.num)
            .expect("Cannot get disk")
            .read_block(start_sec as u64, buf);
        Some(())
    }

    fn write_block(&self, start_sec: usize, buf: &[u8]) -> Option<()> {
        super::ahci::AHCI
            .lock()
            .get_disk(self.num)
            .expect("Cannot get disk")
            .write_block(start_sec as u64, buf);
        Some(())
    }

    fn get_size(&self) -> usize {
        super::ahci::AHCI
            .lock()
            .get_disk(self.num)
            .unwrap()
            .get_identity()
            .lba48_sectors as usize
            * 512
    }
}

struct NVMeDisk {
    num: usize,
}

impl BlockDevice for NVMeDisk {
    fn read_block(&self, start_sec: usize, buf: &mut [u8]) -> Option<()> {
        crate::device::nvme::read_block(self.num, start_sec as u64, buf);
        Some(())
    }

    fn write_block(&self, start_sec: usize, buf: &[u8]) -> Option<()> {
        crate::device::nvme::write_block(self.num, start_sec as u64, buf);
        Some(())
    }

    fn get_size(&self) -> usize {
        crate::device::nvme::get_hd_size(self.num).unwrap()
    }
}

pub static HD_LIST: Mutex<Vec<Arc<dyn BlockDevice>>> = Mutex::new(Vec::new());

pub fn init() {
    let ahci_disk_num = crate::device::ahci::AHCI.lock().len();

    for num in 0..ahci_disk_num {
        let disk = Arc::new(AHCIDisk { num });
        HD_LIST.lock().push(disk.clone());
    }

    let nvme_disk_num = crate::device::nvme::get_hd_num();

    for num in 0..nvme_disk_num {
        let disk = Arc::new(NVMeDisk { num });
        HD_LIST.lock().push(disk.clone());
    }
}
