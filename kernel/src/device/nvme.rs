/// NVMe Spec 4.2
/// Submission queue entry
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct NvmeCommand {
    /// Opcode
    pub opcode: u8,
    /// Flags; FUSE (2 bits) | Reserved (4 bits) | PSDT (2 bits)
    pub flags: u8,
    /// Command ID
    pub c_id: u16,
    /// Namespace ID
    pub ns_id: u32,
    /// Reserved
    pub _rsvd: u64,
    /// Metadata pointer
    pub md_ptr: u64,
    /// Data pointer
    pub d_ptr: [u64; 2],
    /// Command dword 10
    pub cdw10: u32,
    /// Command dword 11
    pub cdw11: u32,
    /// Command dword 12
    pub cdw12: u32,
    /// Command dword 13
    pub cdw13: u32,
    /// Command dword 14
    pub cdw14: u32,
    /// Command dword 15
    pub cdw15: u32,
}

impl NvmeCommand {
    pub fn create_io_completion_queue(c_id: u16, qid: u16, ptr: usize, size: u16) -> Self {
        Self {
            opcode: 5,
            flags: 0,
            c_id,
            ns_id: 0,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr as u64, 0],
            cdw10: ((size as u32) << 16) | (qid as u32),
            cdw11: 1, // Physically Contiguous
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn create_io_submission_queue(
        c_id: u16,
        q_id: u16,
        ptr: usize,
        size: u16,
        cq_id: u16,
    ) -> Self {
        Self {
            opcode: 1,
            flags: 0,
            c_id,
            ns_id: 0,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr as u64, 0],
            cdw10: ((size as u32) << 16) | (q_id as u32),
            cdw11: ((cq_id as u32) << 16) | 1, /* Physically Contiguous */
            //TODO: QPRIO
            cdw12: 0, //TODO: NVMSETID
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn delete_io_submission_queue(c_id: u16, q_id: u16) -> Self {
        Self {
            opcode: 0,
            c_id,
            cdw10: q_id as u32,
            ..Default::default()
        }
    }

    pub fn delete_io_completion_queue(c_id: u16, q_id: u16) -> Self {
        Self {
            opcode: 4,
            c_id,
            cdw10: q_id as u32,
            ..Default::default()
        }
    }

    pub fn identify_namespace(c_id: u16, ptr: usize, ns_id: u32) -> Self {
        Self {
            opcode: 6,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr as u64, 0],
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn identify_controller(c_id: u16, ptr: usize) -> Self {
        Self {
            opcode: 6,
            flags: 0,
            c_id,
            ns_id: 0,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr as u64, 0],
            cdw10: 1,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn identify_namespace_list(c_id: u16, ptr: usize, base: u32) -> Self {
        Self {
            opcode: 6,
            flags: 0,
            c_id,
            ns_id: base,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr as u64, 0],
            cdw10: 2,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn get_features(_c_id: u16, ptr: usize, fid: u8) -> Self {
        Self {
            opcode: 0xA,
            d_ptr: [ptr as u64, 0],
            cdw10: u32::from(fid), // TODO: SEL
            ..Default::default()
        }
    }

    pub fn io_read(c_id: u16, ns_id: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> Self {
        Self {
            opcode: 2,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr0, ptr1],
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: blocks_1 as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn io_write(c_id: u16, ns_id: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> Self {
        Self {
            opcode: 1,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [ptr0, ptr1],
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: blocks_1 as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn format_nvm(c_id: u16, ns_id: u32) -> Self {
        Self {
            opcode: 0x80,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: 1 << 9,
            // TODO: dealloc and prinfo bits
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn async_event_req(c_id: u16) -> Self {
        Self {
            opcode: 0xC,
            flags: 0,
            c_id,
            ns_id: 0,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    pub fn get_log_page(c_id: u16, numd: u32, ptr0: u64, ptr1: u64, lid: u8, lpid: u16) -> Self {
        Self {
            c_id,
            d_ptr: [ptr0, ptr1],
            cdw10: (numd << 16) | lid as u32,
            cdw11: ((lpid as u32) << 16) | numd >> 16,
            ..Self::default()
        }
    }

    // not supported by samsung
    pub fn write_zeroes(c_id: u16, ns_id: u32, slba: u64, nlb: u16, deac: bool) -> Self {
        Self {
            opcode: 8,
            flags: 0,
            c_id,
            ns_id,
            _rsvd: 0,
            md_ptr: 0,
            d_ptr: [0, 0],
            cdw10: slba as u32,
            // TODO: dealloc and prinfo bits
            cdw11: (slba >> 32) as u32,
            cdw12: ((deac as u32) << 25) | nlb as u32,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }
}

use alloc::boxed::Box;
use core::{
    error::Error,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFull, RangeTo},
    slice,
};
use spin::Mutex;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

const PAGE_BITS: u32 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;

pub struct Dma<T> {
    pub virt: *mut T,
    pub phys: usize,
    pub size: usize,
}

// should be safe
impl<T> Deref for Dma<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.virt }
    }
}

impl<T> DerefMut for Dma<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.virt }
    }
}

// Trait for types that can be viewed as DMA slices
pub trait DmaSlice {
    type Item;

    fn chunks(&self, bytes: usize) -> DmaChunks<u8>;
    fn slice(&self, range: Range<usize>) -> Self::Item;
}

// mildly overengineered lol
pub struct DmaChunks<'a, T> {
    current_offset: usize,
    chunk_size: usize,
    dma: &'a Dma<T>,
}

impl<'a, T> Iterator for DmaChunks<'a, T> {
    type Item = DmaChunk<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_offset >= self.dma.size {
            None
        } else {
            let chunk_phys_addr = self.dma.phys + self.current_offset * core::mem::size_of::<T>();
            let offset_ptr = unsafe { self.dma.virt.add(self.current_offset) };
            let len = core::cmp::min(
                self.chunk_size,
                (self.dma.size - self.current_offset) / core::mem::size_of::<T>(),
            );

            self.current_offset += len;

            Some(DmaChunk {
                phys_addr: chunk_phys_addr,
                slice: unsafe { core::slice::from_raw_parts_mut(offset_ptr, len) },
            })
        }
    }
}

// Represents a chunk obtained from a Dma<T>, with physical address and slice.
pub struct DmaChunk<'a, T> {
    pub phys_addr: usize,
    pub slice: &'a mut [T],
}

impl DmaSlice for Dma<u8> {
    type Item = Dma<u8>;
    fn chunks(&self, bytes: usize) -> DmaChunks<u8> {
        DmaChunks {
            current_offset: 0,
            chunk_size: bytes,
            dma: self,
        }
    }

    fn slice(&self, index: Range<usize>) -> Self::Item {
        assert!(index.end <= self.size, "Index out of bounds");

        unsafe {
            Dma {
                virt: self.virt.add(index.start),
                phys: self.phys + index.start,
                size: (index.end - index.start),
            }
        }
    }
}

impl Index<Range<usize>> for Dma<u8> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        assert!(index.end <= self.size, "Index out of bounds");

        unsafe { slice::from_raw_parts(self.virt.add(index.start), index.end - index.start) }
    }
}

impl IndexMut<Range<usize>> for Dma<u8> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        assert!(index.end <= self.size, "Index out of bounds");
        unsafe { slice::from_raw_parts_mut(self.virt.add(index.start), index.end - index.start) }
    }
}

impl Index<RangeTo<usize>> for Dma<u8> {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[0..index.end]
    }
}

impl IndexMut<RangeTo<usize>> for Dma<u8> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self[0..index.end]
    }
}

impl Index<RangeFull> for Dma<u8> {
    type Output = [u8];

    fn index(&self, _: RangeFull) -> &Self::Output {
        &self[0..self.size]
    }
}

impl IndexMut<RangeFull> for Dma<u8> {
    fn index_mut(&mut self, _: RangeFull) -> &mut Self::Output {
        let len = self.size;
        &mut self[0..len]
    }
}

impl<T> Dma<T> {
    /// Allocates DMA Memory on a huge page
    // TODO: vfio support?
    pub fn allocate(size: usize) -> Result<Dma<T>, Box<dyn Error>> {
        let size = if size % 4096 != 0 {
            ((size >> PAGE_BITS) + 1) << PAGE_BITS
        } else {
            size
        };

        let (paddr, vaddr) = <MemoryManager>::alloc_for_dma(size / PAGE_SIZE);

        Ok(Dma {
            virt: vaddr.as_mut_ptr(),
            phys: paddr.as_u64() as usize,
            size,
        })
    }
}

use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use core::hint::spin_loop;

// clippy doesnt like this
#[allow(unused, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
pub enum NvmeRegs32 {
    VS = 0x8,        // Version
    INTMS = 0xC,     // Interrupt Mask Set
    INTMC = 0x10,    // Interrupt Mask Clear
    CC = 0x14,       // Controller Configuration
    CSTS = 0x1C,     // Controller Status
    NSSR = 0x20,     // NVM Subsystem Reset
    AQA = 0x24,      // Admin Queue Attributes
    CMBLOC = 0x38,   // Contoller Memory Buffer Location
    CMBSZ = 0x3C,    // Controller Memory Buffer Size
    BPINFO = 0x40,   // Boot Partition Info
    BPRSEL = 0x44,   // Boot Partition Read Select
    BPMBL = 0x48,    // Bood Partition Memory Location
    CMBSTS = 0x58,   // Controller Memory Buffer Status
    PMRCAP = 0xE00,  // PMem Capabilities
    PMRCTL = 0xE04,  // PMem Region Control
    PMRSTS = 0xE08,  // PMem Region Status
    PMREBS = 0xE0C,  // PMem Elasticity Buffer Size
    PMRSWTP = 0xE10, // PMem Sustained Write Throughput
}

#[allow(unused, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
pub enum NvmeRegs64 {
    CAP = 0x0,      // Controller Capabilities
    ASQ = 0x28,     // Admin Submission Queue Base Address
    ACQ = 0x30,     // Admin Completion Queue Base Address
    CMBMSC = 0x50,  // Controller Memory Buffer Space Control
    PMRMSC = 0xE14, // Persistent Memory Buffer Space Control
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub enum NvmeArrayRegs {
    SQyTDBL,
    CQyHDBL,
}

// who tf is abbreviating this stuff
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
struct IdentifyNamespaceData {
    pub nsze: u64,
    pub ncap: u64,
    nuse: u64,
    nsfeat: u8,
    pub nlbaf: u8,
    pub flbas: u8,
    mc: u8,
    dpc: u8,
    dps: u8,
    nmic: u8,
    rescap: u8,
    fpi: u8,
    dlfeat: u8,
    nawun: u16,
    nawupf: u16,
    nacwu: u16,
    nabsn: u16,
    nabo: u16,
    nabspf: u16,
    noiob: u16,
    nvmcap: u128,
    npwg: u16,
    npwa: u16,
    npdg: u16,
    npda: u16,
    nows: u16,
    _rsvd1: [u8; 18],
    anagrpid: u32,
    _rsvd2: [u8; 3],
    nsattr: u8,
    nvmsetid: u16,
    endgid: u16,
    nguid: [u8; 16],
    eui64: u64,
    pub lba_format_support: [u32; 16],
    _rsvd3: [u8; 192],
    vendor_specific: [u8; 3712],
}

pub struct NvmeQueuePair {
    pub id: u16,
    pub sub_queue: NvmeSubQueue,
    comp_queue: NvmeCompQueue,
}

impl NvmeQueuePair {
    /// returns amount of requests pushed into submission queue
    pub fn submit_io(&mut self, data: &impl DmaSlice, mut lba: u64, write: bool) -> usize {
        let mut reqs = 0;
        // TODO: contruct PRP list?
        for chunk in data.chunks(2 * 4096) {
            let blocks = (chunk.slice.len() as u64 + 512 - 1) / 512;

            let addr = chunk.phys_addr as u64;
            let bytes = blocks * 512;
            let ptr1 = if bytes <= 4096 {
                0
            } else {
                addr + 4096 // self.page_size
            };

            let entry = if write {
                NvmeCommand::io_write(
                    self.id << 11 | self.sub_queue.tail as u16,
                    1,
                    lba,
                    blocks as u16 - 1,
                    addr,
                    ptr1,
                )
            } else {
                NvmeCommand::io_read(
                    self.id << 11 | self.sub_queue.tail as u16,
                    1,
                    lba,
                    blocks as u16 - 1,
                    addr,
                    ptr1,
                )
            };

            if let Some(tail) = self.sub_queue.submit_checked(entry) {
                unsafe {
                    core::ptr::write_volatile(self.sub_queue.doorbell as *mut u32, tail as u32);
                }
            } else {
                log::error!("queue full");
                return reqs;
            }

            lba += blocks;
            reqs += 1;
        }
        reqs
    }

    // TODO: maybe return result
    pub fn complete_io(&mut self, n: usize) -> Option<u16> {
        assert!(n > 0);
        let (tail, c_entry, _) = self.comp_queue.complete_n(n);
        unsafe {
            core::ptr::write_volatile(self.comp_queue.doorbell as *mut u32, tail as u32);
        }
        self.sub_queue.head = c_entry.sq_head as usize;
        let status = c_entry.status >> 1;
        if status != 0 {
            log::error!(
                "Status: 0x{:x}, Status Code 0x{:x}, Status Code Type: 0x{:x}",
                status,
                status & 0xFF,
                (status >> 8) & 0x7
            );
            log::error!("{:?}", c_entry);
            return None;
        }
        Some(c_entry.sq_head)
    }

    pub fn quick_poll(&mut self) -> Option<()> {
        if let Some((tail, c_entry, _)) = self.comp_queue.complete() {
            unsafe {
                core::ptr::write_volatile(self.comp_queue.doorbell as *mut u32, tail as u32);
            }
            self.sub_queue.head = c_entry.sq_head as usize;
            let status = c_entry.status >> 1;
            if status != 0 {
                log::error!(
                    "Status: 0x{:x}, Status Code 0x{:x}, Status Code Type: 0x{:x}",
                    status,
                    status & 0xFF,
                    (status >> 8) & 0x7
                );
                log::error!("{:?}", c_entry);
            }
            return Some(());
        }
        None
    }
}

#[allow(unused)]
pub struct NvmeDevice {
    addr: *mut u8,
    len: usize,
    // Doorbell stride
    dstrd: u16,
    admin_sq: NvmeSubQueue,
    admin_cq: NvmeCompQueue,
    io_sq: NvmeSubQueue,
    io_cq: NvmeCompQueue,
    buffer: Dma<u8>,           // 2MiB of buffer
    prp_list: Dma<[u64; 512]>, // Address of PRP's, devices doesn't necessarily support 2MiB page sizes; 8 Bytes * 512 = 4096
    pub namespaces: BTreeMap<u32, NvmeNamespace>,
    pub stats: NvmeStats,
    q_id: u16,
}

// TODO
unsafe impl Send for NvmeDevice {}
unsafe impl Sync for NvmeDevice {}

#[allow(unused)]
impl NvmeDevice {
    pub fn init(addr: usize, len: usize) -> Result<Self, Box<dyn Error>> {
        let mut dev = Self {
            addr: addr as *mut u8,
            dstrd: {
                unsafe {
                    ((core::ptr::read_volatile(
                        (addr as usize + NvmeRegs64::CAP as usize) as *const u64,
                    ) >> 32)
                        & 0b1111) as u16
                }
            },
            len,
            admin_sq: NvmeSubQueue::new(QUEUE_LENGTH, 0)?,
            admin_cq: NvmeCompQueue::new(QUEUE_LENGTH, 0)?,
            io_sq: NvmeSubQueue::new(QUEUE_LENGTH, 0)?,
            io_cq: NvmeCompQueue::new(QUEUE_LENGTH, 0)?,
            buffer: Dma::allocate(4096)?,
            prp_list: Dma::allocate(8 * 512)?,
            namespaces: BTreeMap::new(),
            stats: NvmeStats::default(),
            q_id: 1,
        };

        for i in 1..512 {
            dev.prp_list[i - 1] = (dev.buffer.phys + i * 4096) as u64;
        }

        log::info!("CAP: 0x{:x}", dev.get_reg64(NvmeRegs64::CAP as u64));
        log::info!("VS: 0x{:x}", dev.get_reg32(NvmeRegs32::VS as u32));
        log::info!("CC: 0x{:x}", dev.get_reg32(NvmeRegs32::CC as u32));

        log::info!("Disabling controller");
        // Set Enable bit to 0
        let ctrl_config = dev.get_reg32(NvmeRegs32::CC as u32) & 0xFFFF_FFFE;
        dev.set_reg32(NvmeRegs32::CC as u32, ctrl_config);

        // Wait for not ready
        loop {
            let csts = dev.get_reg32(NvmeRegs32::CSTS as u32);
            if csts & 1 == 1 {
                spin_loop();
            } else {
                break;
            }
        }

        // Configure Admin Queues
        dev.set_reg64(NvmeRegs64::ASQ as u32, dev.admin_sq.get_addr() as u64);
        dev.set_reg64(NvmeRegs64::ACQ as u32, dev.admin_cq.get_addr() as u64);
        dev.set_reg32(
            NvmeRegs32::AQA as u32,
            (QUEUE_LENGTH as u32 - 1) << 16 | (QUEUE_LENGTH as u32 - 1),
        );

        // Configure other stuff
        // TODO: check css values
        let mut cc = dev.get_reg32(NvmeRegs32::CC as u32);
        // mask out reserved stuff
        cc &= 0xFF00_000F;
        // Set Completion (2^4 = 16 Bytes) and Submission Entry (2^6 = 64 Bytes) sizes
        cc |= (4 << 20) | (6 << 16);

        // Set Memory Page Size
        // let mpsmax = ((dev.get_reg64(NvmeRegs64::CAP as u64) >> 52) & 0xF) as u32;
        // cc |= (mpsmax << 7);
        // log::info!("MPS {}", (cc >> 7) & 0xF);
        dev.set_reg32(NvmeRegs32::CC as u32, cc);

        // Enable the controller
        log::info!("Enabling controller");
        let ctrl_config = dev.get_reg32(NvmeRegs32::CC as u32) | 1;
        dev.set_reg32(NvmeRegs32::CC as u32, ctrl_config);

        // wait for ready
        loop {
            let csts = dev.get_reg32(NvmeRegs32::CSTS as u32);
            if csts & 1 == 0 {
                spin_loop();
            } else {
                break;
            }
        }

        let q_id = dev.q_id;
        let addr = dev.io_cq.get_addr();
        log::info!("Requesting i/o completion queue");
        let comp = dev.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::create_io_completion_queue(c_id, q_id, addr, (QUEUE_LENGTH - 1) as u16)
        })?;
        let addr = dev.io_sq.get_addr();
        log::info!("Requesting i/o submission queue");
        let comp = dev.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::create_io_submission_queue(
                c_id,
                q_id,
                addr,
                (QUEUE_LENGTH - 1) as u16,
                q_id,
            )
        })?;
        dev.q_id += 1;

        Ok(dev)
    }

    pub fn identify_controller(&mut self) -> Result<(), Box<dyn Error>> {
        log::info!("Trying to identify controller");
        let _entry = self.submit_and_complete_admin(NvmeCommand::identify_controller);

        log::info!("Dumping identify controller");
        let mut serial = String::new();
        let data = &self.buffer;

        for &b in &data[4..24] {
            if b == 0 {
                break;
            }
            serial.push(b as char);
        }

        let mut model = String::new();
        for &b in &data[24..64] {
            if b == 0 {
                break;
            }
            model.push(b as char);
        }

        let mut firmware = String::new();
        for &b in &data[64..72] {
            if b == 0 {
                break;
            }
            firmware.push(b as char);
        }

        log::info!(
            "  - Model: {} Serial: {} Firmware: {}",
            model.trim(),
            serial.trim(),
            firmware.trim()
        );

        Ok(())
    }

    // 1 to 1 Submission/Completion Queue Mapping
    pub fn create_io_queue_pair(&mut self, len: usize) -> Result<NvmeQueuePair, Box<dyn Error>> {
        let q_id = self.q_id;
        log::info!("Requesting i/o queue pair with id {q_id}");

        let offset = 0x1000 + ((4 << self.dstrd) * (2 * q_id + 1) as usize);
        assert!(offset <= self.len - 4, "SQ doorbell offset out of bounds");

        let dbl = self.addr as usize + offset;

        let comp_queue = NvmeCompQueue::new(len, dbl)?;
        let comp = self.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::create_io_completion_queue(
                c_id,
                q_id,
                comp_queue.get_addr(),
                (len - 1) as u16,
            )
        })?;

        let dbl = self.addr as usize + 0x1000 + ((4 << self.dstrd) * (2 * q_id) as usize);
        let sub_queue = NvmeSubQueue::new(len, dbl)?;
        let comp = self.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::create_io_submission_queue(
                c_id,
                q_id,
                sub_queue.get_addr(),
                (len - 1) as u16,
                q_id,
            )
        })?;

        self.q_id += 1;
        Ok(NvmeQueuePair {
            id: q_id,
            sub_queue,
            comp_queue,
        })
    }

    pub fn delete_io_queue_pair(&mut self, qpair: NvmeQueuePair) -> Result<(), Box<dyn Error>> {
        log::info!("Deleting i/o queue pair with id {}", qpair.id);
        self.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::delete_io_submission_queue(c_id, qpair.id)
        })?;
        self.submit_and_complete_admin(|c_id, _| {
            NvmeCommand::delete_io_completion_queue(c_id, qpair.id)
        })?;
        Ok(())
    }

    pub fn identify_namespace_list(&mut self, base: u32) -> Vec<u32> {
        self.submit_and_complete_admin(|c_id, addr| {
            NvmeCommand::identify_namespace_list(c_id, addr, base)
        });

        // TODO: idk bout this/don't hardcode len
        let data: &[u32] =
            unsafe { core::slice::from_raw_parts(self.buffer.virt as *const u32, 1024) };

        data.iter()
            .copied()
            .take_while(|&id| id != 0)
            .collect::<Vec<u32>>()
    }

    pub fn identify_namespace(&mut self, id: u32) -> (NvmeNamespace, u64) {
        self.submit_and_complete_admin(|c_id, addr| {
            NvmeCommand::identify_namespace(c_id, addr, id)
        });

        let namespace_data: IdentifyNamespaceData =
            unsafe { *(self.buffer.virt as *const IdentifyNamespaceData) };

        // let namespace_data = unsafe { *tmp_buff.virt };
        let size = namespace_data.nsze;
        let blocks = namespace_data.ncap;

        // figure out block size
        let flba_idx = (namespace_data.flbas & 0xF) as usize;
        let flba_data = (namespace_data.lba_format_support[flba_idx] >> 16) & 0xFF;
        let block_size = if !(9..32).contains(&flba_data) {
            0
        } else {
            1 << flba_data
        };

        // TODO: check metadata?
        log::info!("Namespace {id}, Size: {size}, Blocks: {blocks}, Block size: {block_size}");

        let namespace = NvmeNamespace {
            id,
            blocks,
            block_size,
        };
        self.namespaces.insert(id, namespace);
        (namespace, blocks * block_size)
    }

    // TODO: currently namespace 1 is hardcoded
    pub fn write(&mut self, data: &impl DmaSlice, mut lba: u64) -> Result<(), Box<dyn Error>> {
        for chunk in data.chunks(2 * 4096) {
            let blocks = (chunk.slice.len() as u64 + 512 - 1) / 512;
            self.namespace_io(1, blocks, lba, chunk.phys_addr as u64, true)?;
            lba += blocks;
        }

        Ok(())
    }

    pub fn read(&mut self, dest: &impl DmaSlice, mut lba: u64) -> Result<(), Box<dyn Error>> {
        // let ns = *self.namespaces.get(&1).unwrap();
        for chunk in dest.chunks(2 * 4096) {
            let blocks = (chunk.slice.len() as u64 + 512 - 1) / 512;
            self.namespace_io(1, blocks, lba, chunk.phys_addr as u64, false)?;
            lba += blocks;
        }
        Ok(())
    }

    pub fn write_copied(&mut self, data: &[u8], mut lba: u64) -> Result<(), Box<dyn Error>> {
        let ns = *self.namespaces.get(&1).unwrap();
        for chunk in data.chunks(128 * 4096) {
            self.buffer[..chunk.len()].copy_from_slice(chunk);
            let blocks = (chunk.len() as u64 + ns.block_size - 1) / ns.block_size;
            self.namespace_io(1, blocks, lba, self.buffer.phys as u64, true)?;
            lba += blocks;
        }

        Ok(())
    }

    pub fn read_copied(&mut self, dest: &mut [u8], mut lba: u64) -> Result<(), Box<dyn Error>> {
        let ns = *self.namespaces.get(&1).unwrap();
        for chunk in dest.chunks_mut(128 * 4096) {
            let blocks = (chunk.len() as u64 + ns.block_size - 1) / ns.block_size;
            self.namespace_io(1, blocks, lba, self.buffer.phys as u64, false)?;
            lba += blocks;
            chunk.copy_from_slice(&self.buffer[..chunk.len()]);
        }
        Ok(())
    }

    fn submit_io(
        &mut self,
        ns: &NvmeNamespace,
        addr: u64,
        blocks: u64,
        lba: u64,
        write: bool,
    ) -> Option<usize> {
        assert!(blocks > 0);
        assert!(blocks <= 0x1_0000);
        let q_id = 1;

        let bytes = blocks * ns.block_size;
        let ptr1 = if bytes <= 4096 {
            0
        } else if bytes <= 8192 {
            addr + 4096 // self.page_size
        } else {
            // idk if this works
            let offset = (addr - self.buffer.phys as u64) / 8;
            self.prp_list.phys as u64 + offset
        };

        let entry = if write {
            NvmeCommand::io_write(
                self.io_sq.tail as u16,
                ns.id,
                lba,
                blocks as u16 - 1,
                addr,
                ptr1,
            )
        } else {
            NvmeCommand::io_read(
                self.io_sq.tail as u16,
                ns.id,
                lba,
                blocks as u16 - 1,
                addr,
                ptr1,
            )
        };
        self.io_sq.submit_checked(entry)
    }

    fn complete_io(&mut self, step: u64) -> Option<u16> {
        let q_id = 1;

        let (tail, c_entry, _) = self.io_cq.complete_n(step as usize);
        self.write_reg_idx(NvmeArrayRegs::CQyHDBL, q_id as u16, tail as u32);

        let status = c_entry.status >> 1;
        if status != 0 {
            log::error!(
                "Status: 0x{:x}, Status Code 0x{:x}, Status Code Type: 0x{:x}",
                status,
                status & 0xFF,
                (status >> 8) & 0x7
            );
            log::error!("{:?}", c_entry);
            return None;
        }
        self.stats.completions += 1;
        Some(c_entry.sq_head)
    }

    pub fn batched_write(
        &mut self,
        ns_id: u32,
        data: &[u8],
        mut lba: u64,
        batch_len: u64,
    ) -> Result<(), Box<dyn Error>> {
        let ns = *self.namespaces.get(&ns_id).unwrap();
        let block_size = 512;
        let q_id = 1;

        for chunk in data.chunks(4096) {
            self.buffer[..chunk.len()].copy_from_slice(chunk);
            let tail = self.io_sq.tail;

            let batch_len = core::cmp::min(batch_len, chunk.len() as u64 / block_size);
            let batch_size = chunk.len() as u64 / batch_len;
            let blocks = batch_size / ns.block_size;

            for i in 0..batch_len {
                if let Some(tail) = self.submit_io(
                    &ns,
                    self.buffer.phys as u64 + i * batch_size,
                    blocks,
                    lba,
                    true,
                ) {
                    self.stats.submissions += 1;
                    self.write_reg_idx(NvmeArrayRegs::SQyTDBL, q_id as u16, tail as u32);
                } else {
                    log::error!("tail: {tail}, batch_len: {batch_len}, batch_size: {batch_size}, blocks: {blocks}");
                }
                lba += blocks;
            }
            self.io_sq.head = self.complete_io(batch_len).unwrap() as usize;
        }

        Ok(())
    }

    pub fn batched_read(
        &mut self,
        ns_id: u32,
        data: &mut [u8],
        mut lba: u64,
        batch_len: u64,
    ) -> Result<(), Box<dyn Error>> {
        let ns = *self.namespaces.get(&ns_id).unwrap();
        let block_size = 512;
        let q_id = 1;

        for chunk in data.chunks_mut(4096) {
            let tail = self.io_sq.tail;

            let batch_len = core::cmp::min(batch_len, chunk.len() as u64 / block_size);
            let batch_size = chunk.len() as u64 / batch_len;
            let blocks = batch_size / ns.block_size;

            for i in 0..batch_len {
                if let Some(tail) = self.submit_io(
                    &ns,
                    self.buffer.phys as u64 + i * batch_size,
                    blocks,
                    lba,
                    false,
                ) {
                    self.stats.submissions += 1;
                    self.write_reg_idx(NvmeArrayRegs::SQyTDBL, q_id as u16, tail as u32);
                } else {
                    log::error!("tail: {tail}, batch_len: {batch_len}, batch_size: {batch_size}, blocks: {blocks}");
                }
                lba += blocks;
            }
            self.io_sq.head = self.complete_io(batch_len).unwrap() as usize;
            chunk.copy_from_slice(&self.buffer[..chunk.len()]);
        }
        Ok(())
    }

    #[inline(always)]
    fn namespace_io(
        &mut self,
        ns_id: u32,
        blocks: u64,
        lba: u64,
        addr: u64,
        write: bool,
    ) -> Result<(), Box<dyn Error>> {
        assert!(blocks > 0);
        assert!(blocks <= 0x1_0000);

        let q_id = 1;

        let bytes = blocks * 512;
        let ptr1 = if bytes <= 4096 {
            0
        } else if bytes <= 8192 {
            // self.buffer.phys as u64 + 4096 // self.page_size
            addr + 4096 // self.page_size
        } else {
            self.prp_list.phys as u64
        };

        let entry = if write {
            NvmeCommand::io_write(
                self.io_sq.tail as u16,
                ns_id,
                lba,
                blocks as u16 - 1,
                addr,
                ptr1,
            )
        } else {
            NvmeCommand::io_read(
                self.io_sq.tail as u16,
                ns_id,
                lba,
                blocks as u16 - 1,
                addr,
                ptr1,
            )
        };

        let tail = self.io_sq.submit(entry);
        self.stats.submissions += 1;

        self.write_reg_idx(NvmeArrayRegs::SQyTDBL, q_id as u16, tail as u32);
        self.io_sq.head = self.complete_io(1).unwrap() as usize;
        Ok(())
    }

    fn submit_and_complete_admin<F: FnOnce(u16, usize) -> NvmeCommand>(
        &mut self,
        cmd_init: F,
    ) -> Result<NvmeCompletion, Box<dyn Error>> {
        let cid = self.admin_sq.tail;
        let tail = self.admin_sq.submit(cmd_init(cid as u16, self.buffer.phys));
        self.write_reg_idx(NvmeArrayRegs::SQyTDBL, 0, tail as u32);

        let (head, entry, _) = self.admin_cq.complete_spin();
        self.write_reg_idx(NvmeArrayRegs::CQyHDBL, 0, head as u32);
        let status = entry.status >> 1;
        if status != 0 {
            log::error!(
                "Status: 0x{:x}, Status Code 0x{:x}, Status Code Type: 0x{:x}",
                status,
                status & 0xFF,
                (status >> 8) & 0x7
            );
            return Err("Requesting i/o completion queue failed".into());
        }
        Ok(entry)
    }

    pub fn clear_namespace(&mut self, ns_id: Option<u32>) {
        let ns_id = if let Some(ns_id) = ns_id {
            assert!(self.namespaces.contains_key(&ns_id));
            ns_id
        } else {
            0xFFFF_FFFF
        };
        self.submit_and_complete_admin(|c_id, _| NvmeCommand::format_nvm(c_id, ns_id));
    }

    /// Sets Queue `qid` Tail Doorbell to `val`
    fn write_reg_idx(&self, reg: NvmeArrayRegs, qid: u16, val: u32) {
        match reg {
            NvmeArrayRegs::SQyTDBL => unsafe {
                core::ptr::write_volatile(
                    (self.addr as usize + 0x1000 + ((4 << self.dstrd) * (2 * qid)) as usize)
                        as *mut u32,
                    val,
                );
            },
            NvmeArrayRegs::CQyHDBL => unsafe {
                core::ptr::write_volatile(
                    (self.addr as usize + 0x1000 + ((4 << self.dstrd) * (2 * qid + 1)) as usize)
                        as *mut u32,
                    val,
                );
            },
        }
    }

    /// Sets the register at `self.addr` + `reg` to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `self.addr` + `reg` does not belong to the mapped memory of the pci device.
    fn set_reg32(&self, reg: u32, value: u32) {
        assert!(reg as usize <= self.len - 4, "memory access out of bounds");

        unsafe {
            core::ptr::write_volatile((self.addr as usize + reg as usize) as *mut u32, value);
        }
    }

    /// Returns the register at `self.addr` + `reg`.
    ///
    /// # Panics
    ///
    /// Panics if `self.addr` + `reg` does not belong to the mapped memory of the pci device.
    fn get_reg32(&self, reg: u32) -> u32 {
        assert!(reg as usize <= self.len - 4, "memory access out of bounds");

        unsafe { core::ptr::read_volatile((self.addr as usize + reg as usize) as *mut u32) }
    }

    /// Sets the register at `self.addr` + `reg` to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `self.addr` + `reg` does not belong to the mapped memory of the pci device.
    fn set_reg64(&self, reg: u32, value: u64) {
        assert!(reg as usize <= self.len - 8, "memory access out of bounds");

        unsafe {
            core::ptr::write_volatile((self.addr as usize + reg as usize) as *mut u64, value);
        }
    }

    /// Returns the register at `self.addr` + `reg`.
    ///
    /// # Panics
    ///
    /// Panics if `self.addr` + `reg` does not belong to the mapped memory of the pci device.
    fn get_reg64(&self, reg: u64) -> u64 {
        assert!(reg as usize <= self.len - 8, "memory access out of bounds");

        unsafe { core::ptr::read_volatile((self.addr as usize + reg as usize) as *mut u64) }
    }
}

use crate::memory::{convert_physical_to_virtual, MemoryManager};

use super::pci::get_device_by_class_code;

/// NVMe spec 4.6
/// Completion queue entry
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct NvmeCompletion {
    /// Command specific
    pub command_specific: u32,
    /// Reserved
    pub _rsvd: u32,
    // Submission queue head
    pub sq_head: u16,
    // Submission queue ID
    pub sq_id: u16,
    // Command ID
    pub c_id: u16,
    //  Status field
    pub status: u16,
}

/// maximum amount of submission entries on a 2MiB huge page
pub const QUEUE_LENGTH: usize = 1024;

/// Submission queue
pub struct NvmeSubQueue {
    // TODO: switch to mempool for larger queue
    commands: Dma<[NvmeCommand; QUEUE_LENGTH]>,
    pub head: usize,
    pub tail: usize,
    len: usize,
    pub doorbell: usize,
}

impl NvmeSubQueue {
    pub fn new(len: usize, doorbell: usize) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            commands: Dma::allocate(4096)?,
            head: 0,
            tail: 0,
            len: len.min(QUEUE_LENGTH),
            doorbell,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.head == (self.tail + 1) % self.len
    }

    pub fn submit_checked(&mut self, entry: NvmeCommand) -> Option<usize> {
        if self.is_full() {
            None
        } else {
            Some(self.submit(entry))
        }
    }

    #[inline(always)]
    pub fn submit(&mut self, entry: NvmeCommand) -> usize {
        // println!("SUBMISSION ENTRY: {:?}", entry);
        self.commands[self.tail] = entry;

        self.tail = (self.tail + 1) % self.len;
        self.tail
    }

    pub fn get_addr(&self) -> usize {
        self.commands.phys
    }
}

/// Completion queue
pub struct NvmeCompQueue {
    commands: Dma<[NvmeCompletion; QUEUE_LENGTH]>,
    head: usize,
    phase: bool,
    len: usize,
    pub doorbell: usize,
}

// TODO: error handling
impl NvmeCompQueue {
    pub fn new(len: usize, doorbell: usize) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            commands: Dma::allocate(4096)?,
            head: 0,
            phase: true,
            len: len.min(QUEUE_LENGTH),
            doorbell,
        })
    }

    #[inline(always)]
    pub fn complete(&mut self) -> Option<(usize, NvmeCompletion, usize)> {
        let entry = &self.commands[self.head];

        if ((entry.status & 1) == 1) == self.phase {
            let prev = self.head;
            self.head = (self.head + 1) % self.len;
            if self.head == 0 {
                self.phase = !self.phase;
            }
            Some((self.head, entry.clone(), prev))
        } else {
            None
        }
    }

    ///
    #[inline(always)]
    pub fn complete_n(&mut self, commands: usize) -> (usize, NvmeCompletion, usize) {
        let prev = self.head;
        self.head += commands - 1;
        if self.head >= self.len {
            self.phase = !self.phase;
        }
        self.head %= self.len;

        let (head, entry, _) = self.complete_spin();
        (head, entry, prev)
    }

    #[inline(always)]
    pub fn complete_spin(&mut self) -> (usize, NvmeCompletion, usize) {
        loop {
            if let Some(val) = self.complete() {
                return val;
            }
            spin_loop();
        }
    }

    pub fn get_addr(&self) -> usize {
        self.commands.phys
    }
}

static NVME_CONS: Mutex<Vec<NvmeDevice>> = Mutex::new(Vec::new());
static NVME_SIZES: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

pub fn init() {
    let pci_devices = get_device_by_class_code(0x01, 0x08);
    let mut nvme_cons = NVME_CONS.lock();
    let mut nvme_sizes = NVME_SIZES.lock();

    let mut idx = 0;
    for pci_device in pci_devices {
        if let Some(bar) = pci_device.bars[0] {
            let (addr, len) = bar.unwrap_mem();
            let addr = PhysAddr::new(addr as u64);
            let vaddr = convert_physical_to_virtual(addr);

            <MemoryManager>::map_virt_to_phys(
                vaddr.as_u64() as usize,
                addr.as_u64() as usize,
                len,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
            );

            log::info!("NVMe OK");
            let mut nvme_device = NvmeDevice::init(vaddr.as_u64() as usize, len as usize)
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

#[derive(Debug, Clone, Copy)]
pub struct NvmeNamespace {
    pub id: u32,
    pub blocks: u64,
    pub block_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NvmeStats {
    pub completions: u64,
    pub submissions: u64,
}

/// Reads a block from the NVMe driver at block block_id
pub fn read_block(hd: usize, block_id: u64, buf: &mut [u8]) {
    let dma: Dma<u8> = Dma::allocate(buf.len()).expect("Cannot allocate frame");
    let mut cons = NVME_CONS.lock();
    let nvme = cons.get_mut(hd).expect("Cannot get hd");
    nvme.read(&dma, block_id).expect("Cannot read");
    unsafe { buf.as_mut_ptr().copy_from(dma.virt, 512) };
}

/// Writes a block to the NVMe driver at block block_id
pub fn write_block(hd: usize, block_id: u64, buf: &[u8]) {
    let dma: Dma<u8> = Dma::allocate(buf.len()).expect("Cannot allocate frame");
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
