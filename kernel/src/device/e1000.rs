use crate::memory::{addr_to_array, convert_physical_to_virtual, MappingType, MemoryManager};
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::size_of;
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;
use vcell::VolatileCell;
use x86_64::{PhysAddr, VirtAddr};

use bit_field::*;
use bitflags::*;

use smoltcp::iface::*;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, *};

use super::pci::get_device_by_class_code;

//use super::EthernetAddress;

// At the beginning, all transmit descriptors have there status non-zero,
// so we need to track whether we are using the descriptor for the first time.
// When the descriptors wrap around, we set first_trans to false,
// and lookup status instead for checking whether it is empty.

pub struct E1000 {
    pub header: usize,
    pub size: usize,
    pub mac: EthernetAddress,
    pub registers: &'static mut [VolatileCell<u32>],
    pub send_queue: &'static mut [E1000SendDesc],
    pub send_buffers: Vec<usize>,
    pub recv_queue: &'static mut [E1000RecvDesc],
    pub recv_buffers: Vec<usize>,
    pub first_trans: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct E1000SendDesc {
    addr: u64,
    len: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct E1000RecvDesc {
    addr: u64,
    len: u16,
    chksum: u16,
    status: u16,
    error: u8,
    special: u8,
}

bitflags! {
    #[derive(Debug)]
    pub struct E1000Status : u32 {
        const FD = 1 << 0;
        const LU = 1 << 1;
        const TXOFF = 1 << 4;
        const TBIMODE = 1 << 5;
        const SPEED_100M = 1 << 6;
        const SPEED_1000M = 1 << 7;
        const ASDV_100M = 1 << 8;
        const ASDV_1000M = 1 << 9;
        const MTXCKOK = 1 << 10;
        const PCI66 = 1 << 11;
        const BUS64 = 1 << 12;
        const PCIX_MODE = 1 << 13;
        const GIO_MASTER_ENABLE = 1 << 19;
    }
}

impl E1000 {
    pub fn new(header: usize, size: usize, mac: EthernetAddress) -> Self {
        assert_eq!(size_of::<E1000SendDesc>(), 16);
        assert_eq!(size_of::<E1000RecvDesc>(), 16);

        let (send_queue_pa, send_queue_va) = <MemoryManager>::alloc_for_dma(1);
        let (recv_queue_pa, recv_queue_va) = <MemoryManager>::alloc_for_dma(1);
        let send_queue: &mut [E1000SendDesc] =
            addr_to_array(send_queue_va, 4096 / size_of::<E1000SendDesc>());
        let recv_queue: &mut [E1000RecvDesc] =
            addr_to_array(recv_queue_va, 4096 / size_of::<E1000RecvDesc>());

        let mut send_buffers = Vec::with_capacity(send_queue.len());
        let mut recv_buffers = Vec::with_capacity(recv_queue.len());

        let e1000: &mut [VolatileCell<u32>] = addr_to_array(VirtAddr::new(header as u64), size / 4);
        log::debug!(
            "status before setup: {:#?}",
            E1000Status::from_bits_truncate(e1000[E1000_STATUS].get())
        );

        // 4.6 Software Initialization Sequence

        // 4.6.6 Transmit Initialization

        // Program the descriptor base address with the address of the region.
        e1000[E1000_TDBAL].set(send_queue_pa.as_u64() as u32); // TDBAL
        e1000[E1000_TDBAH].set((send_queue_pa.as_u64() >> 32) as u32); // TDBAH

        // Set the length register to the size of the descriptor ring.
        e1000[E1000_TDLEN].set(4096u32); // TDLEN

        // If needed, program the head and tail registers.
        e1000[E1000_TDH].set(0); // TDH
        e1000[E1000_TDT].set(0); // TDT

        for i in 0..send_queue.len() {
            let (buffer_page_pa, buffer_page_va) = <MemoryManager>::alloc_for_dma(1);
            send_queue[i].addr = buffer_page_pa.as_u64();
            send_buffers.push(buffer_page_va.as_u64() as usize);
        }

        // EN | PSP | CT=0x10 | COLD=0x40
        e1000[E1000_TCTL].set((1 << 1) | (1 << 3) | (0x10 << 4) | (0x40 << 12)); // TCTL
                                                                                 // IPGT=0xa | IPGR1=0x8 | IPGR2=0xc
        e1000[E1000_TIPG].set(0xa | (0x8 << 10) | (0xc << 20)); // TIPG

        // 4.6.5 Receive Initialization
        let mut ral: u32 = 0;
        let mut rah: u32 = 0;
        for i in 0..4 {
            ral = ral | (mac.as_bytes()[i] as u32) << (i * 8);
        }
        for i in 0..2 {
            rah = rah | (mac.as_bytes()[i + 4] as u32) << (i * 8);
        }

        e1000[E1000_RAL].set(ral); // RAL
                                   // AV | AS=DA
        e1000[E1000_RAH].set(rah | (1 << 31)); // RAH

        // MTA
        for i in E1000_MTA..E1000_RAL {
            e1000[i].set(0);
        }

        // Program the descriptor base address with the address of the region.
        e1000[E1000_RDBAL].set(recv_queue_pa.as_u64() as u32); // RDBAL
        e1000[E1000_RDBAH].set((recv_queue_pa.as_u64() >> 32) as u32); // RDBAH

        // Set the length register to the size of the descriptor ring.
        e1000[E1000_RDLEN].set(4096u32); // RDLEN

        // If needed, program the head and tail registers. Note: the head and tail pointers are initialized (by hardware) to zero after a power-on or a software-initiated device reset.
        e1000[E1000_RDH].set(0); // RDH

        // The tail pointer should be set to point one descriptor beyond the end.
        e1000[E1000_RDT].set((recv_queue.len() - 1) as u32); // RDT

        // Receive buffers of appropriate size should be allocated and pointers to these buffers should be stored in the descriptor ring.
        for i in 0..recv_queue.len() {
            let (buffer_page_pa, buffer_page_va) = <MemoryManager>::alloc_for_dma(1);
            recv_queue[i].addr = buffer_page_pa.as_u64();
            recv_buffers.push(buffer_page_va.as_u64() as usize);
        }

        // EN | BAM | BSIZE=3 | BSEX | SECRC
        // BSIZE=3 | BSEX means buffer size = 4096
        e1000[E1000_RCTL].set((1 << 1) | (1 << 15) | (3 << 16) | (1 << 25) | (1 << 26)); // RCTL

        log::debug!(
            "status after setup: {:#?}",
            E1000Status::from_bits_truncate(e1000[E1000_STATUS].get())
        );

        // enable interrupt
        // clear interrupt
        e1000[E1000_ICR].set(e1000[E1000_ICR].get());
        // RXT0
        e1000[E1000_IMS].set(1 << 7); // IMS

        // clear interrupt
        e1000[E1000_ICR].set(e1000[E1000_ICR].get());

        E1000 {
            header,
            size,
            mac,
            registers: e1000,
            send_queue,
            send_buffers,
            recv_queue,
            recv_buffers,
            first_trans: true,
        }
    }

    pub fn handle_interrupt(&mut self) -> bool {
        let icr = self.registers[E1000_ICR].get();
        if icr != 0 {
            // clear it
            self.registers[E1000_ICR].set(icr);
            true
        } else {
            false
        }
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        let tdt = self.registers[E1000_TDT].get() as usize;
        let index = tdt % self.send_queue.len();
        let send_desc = &mut self.send_queue[index];

        let mut rdt = self.registers[E1000_RDT].get() as usize;
        let index = (rdt + 1) % self.recv_queue.len();
        let recv_desc = &mut self.recv_queue[index];

        let transmit_avail = self.first_trans || send_desc.status.get_bit(0);
        let receive_avail = recv_desc.status.get_bit(0);

        if !(transmit_avail && receive_avail) {
            return None;
        }
        let buffer = addr_to_array(
            VirtAddr::new(self.recv_buffers[index] as u64),
            recv_desc.len as usize,
        );

        recv_desc.status.set_bit(0, false);

        rdt = index;
        self.registers[E1000_RDT].set(rdt as u32);

        Some(buffer.to_vec())
    }

    pub fn can_send(&self) -> bool {
        let tdt = self.registers[E1000_TDT].get();
        let index = (tdt as usize) % self.send_queue.len();
        let send_desc = &self.send_queue[index];
        self.first_trans || send_desc.status.get_bit(0)
    }

    pub fn send(&mut self, buffer: &[u8]) {
        let mut tdt = self.registers[E1000_TDT].get();
        let index = (tdt as usize) % self.send_queue.len();
        let send_desc = &mut self.send_queue[index];
        assert!(self.first_trans || send_desc.status.get_bit(0));

        let target = addr_to_array(VirtAddr::new(self.send_buffers[index] as u64), buffer.len());
        target.copy_from_slice(&buffer);

        send_desc.len = buffer.len() as u16 + 4;
        send_desc.cmd = (1 << 3) | (1 << 1) | (1 << 0); // RS | IFCS | EOP
        send_desc.status = 0;
        fence(Ordering::SeqCst);

        tdt = (tdt + 1) % self.send_queue.len() as u32;
        self.registers[E1000_TDT].set(tdt);
        fence(Ordering::SeqCst);

        // round
        if tdt == 0 {
            self.first_trans = false;
        }
    }
}

impl Drop for E1000 {
    fn drop(&mut self) {
        <MemoryManager>::dealloc_for_dma(VirtAddr::from_ptr(self.send_queue.as_ptr()), 4096);
        <MemoryManager>::dealloc_for_dma(VirtAddr::from_ptr(self.recv_queue.as_ptr()), 4096);
        for &send_buffer in self.send_buffers.iter() {
            <MemoryManager>::dealloc_for_dma(VirtAddr::new(send_buffer as u64), 4096);
        }
        for &recv_buffer in self.recv_buffers.iter() {
            <MemoryManager>::dealloc_for_dma(VirtAddr::new(recv_buffer as u64), 4096);
        }
    }
}

pub const E1000_STATUS: usize = 0x0008 / 4;
pub const E1000_ICR: usize = 0x00C0 / 4;
pub const E1000_IMS: usize = 0x00D0 / 4;
pub const E1000_IMC: usize = 0x00D8 / 4;
pub const E1000_RCTL: usize = 0x0100 / 4;
pub const E1000_TCTL: usize = 0x0400 / 4;
pub const E1000_TIPG: usize = 0x0410 / 4;
pub const E1000_RDBAL: usize = 0x2800 / 4;
pub const E1000_RDBAH: usize = 0x2804 / 4;
pub const E1000_RDLEN: usize = 0x2808 / 4;
pub const E1000_RDH: usize = 0x2810 / 4;
pub const E1000_RDT: usize = 0x2818 / 4;
pub const E1000_TDBAL: usize = 0x3800 / 4;
pub const E1000_TDBAH: usize = 0x3804 / 4;
pub const E1000_TDLEN: usize = 0x3808 / 4;
pub const E1000_TDH: usize = 0x3810 / 4;
pub const E1000_TDT: usize = 0x3818 / 4;
pub const E1000_MTA: usize = 0x5200 / 4;
pub const E1000_RAL: usize = 0x5400 / 4;
pub const E1000_RAH: usize = 0x5404 / 4;

#[derive(Clone)]
pub struct E1000Driver(Arc<Mutex<E1000>>);

pub struct E1000Interface {
    pub iface: Mutex<Interface>,
    pub driver: E1000Driver,
    pub name: String,
    pub irq: Option<usize>,
}

pub struct E1000RxToken(Vec<u8>);
pub struct E1000TxToken(E1000Driver);

impl phy::Device for E1000Driver {
    type RxToken<'a> = E1000RxToken;
    type TxToken<'a> = E1000TxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        self.0
            .lock()
            .receive()
            .map(|vec| (E1000RxToken(vec), E1000TxToken(self.clone())))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if self.0.lock().can_send() {
            Some(E1000TxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(64);
        caps
    }
}

impl phy::RxToken for E1000RxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.0)
    }
}

impl phy::TxToken for E1000TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = [0u8; 4096];
        let result = f(&mut buffer[..len]);

        let mut driver = (self.0).0.lock();
        driver.send(&buffer);

        result
    }
}

/// Clear any existing IP addresses & add the new one
pub fn set_ipv4_addr(iface: &mut Interface, cidr: Ipv4Cidr) {
    iface.update_ip_addrs(|addrs| {
        addrs.clear();
        addrs.push(IpCidr::Ipv4(cidr)).unwrap();
    });
}

pub fn get_current_instant() -> Instant {
    //log::info!("{}",framework::drivers::rtc::RtcDateTime::new().to_datetime().unwrap().unix_timestamp());
    Instant::from_secs(
        crate::device::rtc::RtcDateTime::new()
            .to_datetime()
            .unwrap()
            .unix_timestamp(),
    )
}

pub static E1000_DEVICES: Mutex<Vec<E1000Interface>> = Mutex::new(Vec::new());

pub fn init() {
    let devices = get_device_by_class_code(0x02, 0x00);

    let mut drivers = E1000_DEVICES.lock();

    let mut idx = 0;

    for device in devices {
        if let Some(bar) = device.bars[0] {
            let (header, size) = bar.unwrap_mem();

            let paddr = PhysAddr::new(header as u64);
            let vaddr = convert_physical_to_virtual(paddr);

            <MemoryManager>::map_virt_to_phys(
                vaddr.as_u64() as usize,
                paddr.as_u64() as usize,
                size,
                MappingType::KernelData.flags(),
            );

            let mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
            let e1000 = E1000::new(vaddr.as_u64() as usize, size, EthernetAddress(mac));
            let mut net_driver = E1000Driver(Arc::new(Mutex::new(e1000)));

            // let ethernet_addr = EthernetAddress::from_bytes(&mac);
            // let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24)];
            let mut config = Config::new(HardwareAddress::Ethernet(EthernetAddress(mac)));
            config.random_seed = 10;

            let iface = Interface::new(config, &mut net_driver, get_current_instant());
            drivers.push(E1000Interface {
                iface: Mutex::new(iface),
                driver: net_driver,
                name: format!("eth{}", idx),
                irq: None, // TODO: enable interrupt handling for this device
            });

            idx += 1;
        }
    }
}
