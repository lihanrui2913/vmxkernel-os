use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bit_field::BitField;
use core::{mem::size_of, slice};
use spin::{Lazy, Mutex};
use vcell::VolatileCell as Volatile;
use x86_64::{PhysAddr, VirtAddr};

use crate::memory::{convert_physical_to_virtual, MemoryManager};

use super::pci::get_device_by_class_code;

pub static AHCI: Lazy<Mutex<AhciManager>> = Lazy::new(|| Mutex::new(AhciManager::new()));

pub struct AhciManager(Vec<Ahci>);

impl AhciManager {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get_disk(&mut self, index: usize) -> Option<&mut Ahci> {
        self.0.get_mut(index)
    }
}

impl AhciManager {
    pub fn new() -> Self {
        let devices = get_device_by_class_code(0x1, 0x6);

        let mut connections = Vec::new();
        for device in devices {
            if let Some(bar) = device.bars[5] {
                let (address, _size) = bar.unwrap_mem();
                let physical_address = PhysAddr::new(address as u64);
                let virtual_address = convert_physical_to_virtual(physical_address);
                connections.extend(unsafe { Ahci::new(virtual_address) });
            }
        }

        Self(connections)
    }
}

pub struct Ahci {
    cmd_list: &'static [CommandHeader],
    cmd_table: &'static mut CommandTable,
    data: &'static mut [u8],
    port: &'static HbaPort,
}

unsafe impl Send for Ahci {}

impl Ahci {
    pub unsafe fn new(address: VirtAddr) -> Vec<Self> {
        let hba_memory = &*address.as_mut_ptr::<HbaMemory>();

        if !hba_memory.ahci_enabled() {
            return Vec::new();
        }

        (0..hba_memory.support_port_count())
            .filter(|&port_num| hba_memory.port_active(port_num))
            .filter_map(|port_num| hba_memory.get_port(port_num))
            .map(|port| port.init_ahci())
            .collect()
    }

    pub fn get_identity(&mut self) -> DiskIdentify {
        self.execute_command(CMD_IDENTIFY_DEVICE, 0);
        let packet = unsafe { &*(self.data.as_ptr() as *const SataIdentify) };

        DiskIdentify::new(
            packet.serial_number,
            packet.firmware_revision,
            packet.model,
            packet.lba48_sectors,
        )
    }

    pub fn read_block(&mut self, start_sector: u64, buffer: &mut [u8]) {
        self.execute_command(CMD_READ_DMA_EXT, start_sector);
        let length = buffer.len().min(BLOCK_SIZE);
        buffer.copy_from_slice(&self.data.as_ref()[..length]);
    }

    pub fn write_block(&mut self, start_sector: u64, buffer: &[u8]) {
        let length = buffer.len().min(BLOCK_SIZE);
        self.data.as_mut()[..length].copy_from_slice(&buffer[..length]);
        self.execute_command(CMD_WRITE_DMA_EXT, start_sector);
    }

    fn execute_command(&mut self, command: u8, start_sector: u64) {
        let cmd_table = &mut *self.cmd_table;
        let fis = &mut cmd_table.cfis;
        fis.fis_type = FIS_TYPE_REG_H2D;
        fis.cflags = 1 << 7;
        fis.command = command;

        fis.device = match command {
            CMD_READ_DMA_EXT | CMD_WRITE_DMA_EXT => 1 << 6,
            _ => 0,
        };

        fis.sector_count = if command == CMD_IDENTIFY_DEVICE { 0 } else { 1 };
        fis.set_lba(start_sector);

        self.port.command_issue.set(1 << (0 as u32));
        while self.port.command_issue.get().get_bit(0) {}
    }
}

impl Drop for Ahci {
    fn drop(&mut self) {
        <MemoryManager>::dealloc_for_dma(VirtAddr::from_ptr(self.cmd_list.as_ptr()), 0);
        <MemoryManager>::dealloc_for_dma(VirtAddr::from_ptr(self.cmd_table as *const _), 0);
        <MemoryManager>::dealloc_for_dma(VirtAddr::from_ptr(self.data.as_ptr()), 0);
    }
}

#[repr(C)]
struct HbaMemory {
    capability: Volatile<u32>,
    global_host_control: Volatile<u32>,
    interrupt_status: Volatile<u32>,
    port_implemented: Volatile<u32>,
    version: Volatile<u32>,
    ccc_control: Volatile<u32>,
    ccc_ports: Volatile<u32>,
    em_location: Volatile<u32>,
    em_control: Volatile<u32>,
    capabilities2: Volatile<u32>,
    bios_os_handoff_control: Volatile<u32>,
}

impl HbaMemory {
    fn ahci_enabled(&self) -> bool {
        self.global_host_control.get().get_bit(31)
    }

    fn port_active(&self, port_num: usize) -> bool {
        self.port_implemented.get().get_bit(port_num)
    }

    fn support_port_count(&self) -> usize {
        self.capability.get().get_bits(0..5) as usize + 1
    }
}

impl HbaMemory {
    pub fn get_port(&self, port_num: usize) -> Option<&HbaPort> {
        let hba_ptr = self as *const _ as usize;
        let port_address = hba_ptr + 0x100 + 0x80 * port_num;

        let port = unsafe { &*(port_address as *const HbaPort) };
        match (port.device_connected(), port.is_sata_device()) {
            (true, true) => Some(port),
            _ => None,
        }
    }
}

#[repr(C)]
struct HbaPort {
    command_list_base_address: Volatile<u64>,
    fis_base_address: Volatile<u64>,
    interrupt_status: Volatile<u32>,
    interrupt_enable: Volatile<u32>,
    command: Volatile<u32>,
    reserved: Volatile<u32>,
    task_file_data: Volatile<u32>,
    signature: Volatile<u32>,
    sata_status: Volatile<u32>,
    sata_control: Volatile<u32>,
    sata_error: Volatile<u32>,
    sata_active: Volatile<u32>,
    command_issue: Volatile<u32>,
    sata_notification: Volatile<u32>,
    fis_based_switch_control: Volatile<u32>,
}

const SATA_SIG_ATAPI: u32 = 0xeb140101;
const SATA_SIG_SEMB: u32 = 0xc33c0101;
const SATA_SIG_PM: u32 = 0x96690101;

impl HbaPort {
    unsafe fn init_ahci(&'static self) -> Ahci {
        self.stop_cmd();

        let (cmd_list_pa, cmd_list_va) = <MemoryManager>::alloc_for_dma(1);
        let (cmd_table_pa, cmd_table_va) = <MemoryManager>::alloc_for_dma(1);
        let (data_pa, data_va) = <MemoryManager>::alloc_for_dma(1);

        self.command_list_base_address.set(cmd_list_pa.as_u64());

        let cmd_list_size = 4096 / size_of::<CommandHeader>();
        let cmd_list_ptr = cmd_list_va.as_mut_ptr::<CommandHeader>();
        let cmd_list = slice::from_raw_parts_mut(cmd_list_ptr, cmd_list_size);

        let cmd_header = &mut cmd_list[0];
        cmd_header.command_table_base_address = cmd_table_pa.as_u64();
        cmd_header.prdt_length = 1;
        cmd_header.flags = 4;

        let cmd_table = &mut *cmd_table_va.as_mut_ptr::<CommandTable>();
        let prdt = &mut cmd_table.prdt[0];
        prdt.data_base_address = data_pa.as_u64();
        prdt.byte_count_i = (BLOCK_SIZE - 1) as u32;

        self.start_cmd();

        let data = slice::from_raw_parts_mut(data_va.as_mut_ptr(), BLOCK_SIZE);

        Ahci {
            cmd_list,
            cmd_table,
            data,
            port: &self,
        }
    }

    fn start_cmd(&self) {
        let command = &self.command;
        while command.get().get_bit(15) {}
        command.set(*command.get().set_bit(4, true));
        command.set(*command.get().set_bit(0, true));
    }

    fn stop_cmd(&self) {
        let command = &self.command;
        command.set(*command.get().set_bit(0, false));
        command.set(*command.get().set_bit(4, false));
        while command.get().get_bit(15) || command.get().get_bit(14) {}
    }

    fn is_sata_device(&self) -> bool {
        match self.signature.get() {
            SATA_SIG_ATAPI | SATA_SIG_SEMB | SATA_SIG_PM => false,
            _ => true,
        }
    }

    fn device_connected(&self) -> bool {
        let sata_status = &self.sata_status;
        let ipm_active = sata_status.get().get_bits(8..12) == 1;
        let det_present = sata_status.get().get_bits(0..4) == 3;
        ipm_active && det_present
    }
}

#[repr(C)]
struct CommandHeader {
    flags: u16,
    prdt_length: u16,
    prd_byte_count: Volatile<u32>,
    command_table_base_address: u64,
    reserved: [u32; 4],
}

#[repr(C)]
struct CommandTable {
    cfis: FisRegH2D,
    acmd: [u8; 16],
    reserved: [u8; 48],
    prdt: [PrdtEntry; 1],
}

#[repr(C)]
struct PrdtEntry {
    data_base_address: u64,
    reserved: u32,
    byte_count_i: u32,
}

#[repr(C)]
struct FisRegH2D {
    fis_type: u8,
    cflags: u8,
    command: u8,
    feature_lo: u8,
    lba_0: u8,
    lba_1: u8,
    lba_2: u8,
    device: u8,
    lba_3: u8,
    lba_4: u8,
    lba_5: u8,
    feature_hi: u8,
    sector_count: u16,
    command_completion: u8,
    control: u8,
    _padding: [u8; 48],
}

impl FisRegH2D {
    fn set_lba(&mut self, lba: u64) {
        self.lba_0 = (lba >> 0) as u8;
        self.lba_1 = (lba >> 8) as u8;
        self.lba_2 = (lba >> 16) as u8;
        self.lba_3 = (lba >> 24) as u8;
        self.lba_4 = (lba >> 32) as u8;
        self.lba_5 = (lba >> 40) as u8;
    }
}

const BLOCK_SIZE: usize = 512;
const FIS_TYPE_REG_H2D: u8 = 0x27;
const CMD_READ_DMA_EXT: u8 = 0x25;
const CMD_WRITE_DMA_EXT: u8 = 0x35;
const CMD_IDENTIFY_DEVICE: u8 = 0xec;

#[repr(C)]
struct SataIdentify {
    _1: [u16; 10],
    serial_number: [u8; 20],
    _2: [u16; 3],
    firmware_revision: [u8; 8],
    model: [u8; 40],
    _3: [u16; 53],
    lba48_sectors: u64,
}

#[derive(Debug, Default, Clone)]
pub struct DiskIdentify {
    pub serial_number: String,
    pub firmware_revision: String,
    pub model: String,
    pub lba48_sectors: u64,
}

impl DiskIdentify {
    pub fn new(
        serial_number: [u8; 20],
        firmware_revision: [u8; 8],
        model: [u8; 40],
        lba48_sectors: u64,
    ) -> Self {
        let parse_string = |input: &[u8]| -> String {
            let corrected = input
                .chunks(2)
                .flat_map(|chunk| chunk.iter().rev())
                .copied()
                .collect::<Vec<u8>>();

            String::from_utf8_lossy(&corrected).trim_end().to_string()
        };

        Self {
            serial_number: parse_string(&serial_number),
            firmware_revision: parse_string(&firmware_revision),
            model: parse_string(&model),
            lba48_sectors,
        }
    }
}
