use core::num::NonZeroUsize;

use x86_64::PhysAddr;
use xhci::accessor::Mapper;
use xhci::Registers;

use crate::memory::{convert_physical_to_virtual, MappingType, MemoryManager};

use super::pci::get_device_by_class_code;

#[derive(Clone)]
pub struct XHCIMapper;

impl Mapper for XHCIMapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> core::num::NonZeroUsize {
        let physical_address = PhysAddr::new(phys_start as u64);
        let virtual_address = convert_physical_to_virtual(physical_address);

        let size = bytes + 4095;

        <MemoryManager>::map_virt_to_phys(
            virtual_address.as_u64() as usize,
            physical_address.as_u64() as usize,
            size,
            MappingType::KernelData.flags(),
        );

        NonZeroUsize::new(virtual_address.as_u64() as usize).unwrap()
    }

    fn unmap(&mut self, _virt_start: usize, _bytes: usize) {}
}

pub fn get_xhci(mmio_base: usize) -> Registers<XHCIMapper> {
    unsafe { Registers::new(mmio_base, XHCIMapper) }
}

pub fn init() {
    let xhci_devices = get_device_by_class_code(0x0C, 0x03);
    if xhci_devices.len() > 0 {
        let xhci_device = || {
            for device in xhci_devices {
                if device.bars[0].is_none() && device.bars[1].is_none() {
                    continue;
                }

                return Some(device);
            }
            return None;
        };
        let xhci_device = xhci_device();
        if let Some(xhci_device) = xhci_device {
            let bar = if let Some(bar) = xhci_device.bars[0] {
                bar
            } else {
                xhci_device.bars[1].unwrap()
            };
            let (mmio, _size) = bar.unwrap_mem();
            log::info!("MMIO address: {:x}", mmio);
            let mut xhci = get_xhci(mmio as usize);
            let operational = &mut xhci.operational;

            operational.usbcmd.update_volatile(|usb_command_register| {
                usb_command_register.set_run_stop();
            });
            while operational.usbsts.read_volatile().hc_halted() {}

            let num_ports = xhci.capability.hcsparams1.read_volatile().number_of_ports();

            log::info!("XHCI initialized! Ports: {}", num_ports);
        }
    }
}
