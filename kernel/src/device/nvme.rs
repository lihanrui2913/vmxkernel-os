use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use nvme::{
    memory::{Allocator, Dma},
    nvme::NvmeDevice,
};
use spin::Mutex;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

use crate::memory::{convert_physical_to_virtual, MemoryManager};

use super::pci::get_device_by_class_code;

pub struct AllocatorImpl;

impl Allocator for AllocatorImpl {
    unsafe fn allocate(&self, size: usize) -> (usize, usize) {
        let (paddr, vaddr) = <MemoryManager>::alloc_for_dma(size / 4096);
        (paddr.as_u64() as usize, vaddr.as_u64() as usize)
    }
}

static NVME_CONS: Mutex<Vec<NvmeDevice<AllocatorImpl>>> = Mutex::new(Vec::new());
static NVME_SIZES: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

pub fn init() {
    let pci_devices = get_device_by_class_code(0x01, 0x08);
    if pci_devices.len() > 0 {
        let mut nvme_cons = NVME_CONS.lock();
        let mut nvme_sizes = NVME_SIZES.lock();

        let mut idx = 0;
        for pci_device in pci_devices {
            if let Some(bar0) = pci_device.bars[0] {
                let addr: PhysAddr;
                let len: usize;
                let (addr0, len0) = bar0.unwrap_mem();
                if let Some(bar1) = pci_device.bars[1] {
                    let (addr1, len1) = bar1.unwrap_mem();
                    addr = PhysAddr::new(
                        ((addr1 as u64) << 32) | ((addr0 as u64) & 0xFFFFFFFF) as u64,
                    );
                    len = len0 + len1;
                } else {
                    addr = PhysAddr::new(addr0 as u64);
                    len = len0;
                }

                let vaddr = convert_physical_to_virtual(addr);

                <MemoryManager>::map_virt_to_phys(
                    vaddr.as_u64() as usize,
                    addr.as_u64() as usize,
                    len,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                );

                log::info!("NVMe OK");
                let mut nvme_device =
                    NvmeDevice::init(vaddr.as_u64() as usize, len as usize, AllocatorImpl)
                        .expect("Cannot init NVMe device");

                nvme_device
                    .identify_controller()
                    .expect("Cannot identify controller");
                let ns = nvme_device.identify_namespace_list(0);

                let mut nvmcap = 0;
                for n in ns {
                    let cap = nvme_device.identify_namespace(n).1 as usize;
                    nvmcap += cap;
                }

                nvme_cons.push(nvme_device);
                log::info!("NVM capacity = {}", nvmcap);
                nvme_sizes.insert(idx, nvmcap);
            }

            idx += 1;
        }
    }
}

/// Reads a block from the NVMe driver at block block_id
pub fn read_block(hd: usize, block_id: u64, buf: &mut [u8]) {
    let dma: Dma<u8> = Dma::<u8>::allocate(&AllocatorImpl, buf.len());
    let mut cons = NVME_CONS.lock();
    let nvme = cons.get_mut(hd).expect("Cannot get hd");
    nvme.read(&dma, block_id).expect("Cannot read");
    unsafe { buf.as_mut_ptr().copy_from(dma.virt, 512) };
}

/// Writes a block to the NVMe driver at block block_id
pub fn write_block(hd: usize, block_id: u64, buf: &[u8]) {
    let dma: Dma<u8> = Dma::<u8>::allocate(&AllocatorImpl, buf.len());
    unsafe { dma.virt.copy_from(buf.as_ptr(), 512) };
    let mut cons = NVME_CONS.lock();
    let nvme = cons.get_mut(hd).expect("Cannot get hd");
    nvme.write(&dma, block_id).expect("Cannot write");
}

/// Gets the number of NVMe drives
pub fn get_hd_num() -> usize {
    let cons = NVME_CONS.lock();
    cons.len()
}

/// Get the driver size of the NVMe driver.
pub fn get_hd_size(hd: usize) -> Option<usize> {
    let cons = NVME_SIZES.lock();
    cons.get(&hd).copied()
}
