use core::{num::NonZeroUsize, sync::atomic::AtomicUsize};

use x86_64::{structures::idt::InterruptStackFrame, PhysAddr};
use xhci::accessor::Mapper;
use xhci::Registers;

use crate::{
    arch::interrupts::{register_irq, InterruptIndex},
    memory::{convert_physical_to_virtual, MappingType, MemoryManager},
};

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

pub static XHCI_PORTS_NUM: AtomicUsize = AtomicUsize::new(0);

pub fn init() {
    let xhci_devices = get_device_by_class_code(0x0C, 0x03);
    if xhci_devices.len() > 0 {
        let xhci_device = || {
            for device in xhci_devices {
                if device.interface != 0x30 {
                    continue;
                }
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
            let mut xhci = unsafe { Registers::new(mmio, XHCIMapper) };

            let operational = &mut xhci.operational;

            operational.usbcmd.update_volatile(|usb_command_register| {
                usb_command_register.set_run_stop();
            });
            while operational.usbsts.read_volatile().hc_halted() {}

            let num_ports = xhci.capability.hcsparams1.read_volatile().number_of_ports();
            log::info!("XHCI Ports: {}", num_ports);
            XHCI_PORTS_NUM.store(num_ports as usize, core::sync::atomic::Ordering::SeqCst);

            operational.usbcmd.update_volatile(|usb_command_register| {
                usb_command_register.set_host_controller_reset();
            });

            // operational.config.update_volatile(|usb_config_register| {
            //     usb_config_register.set_max_device_slots_enabled(255);
            // });

            let interrupter_register_set = &mut xhci.interrupter_register_set;
            let mut interrupter =
                interrupter_register_set.interrupter_mut(InterruptIndex::Xhci as usize);

            interrupter.erstsz.update_volatile(|erstsz| erstsz.set(1));
            interrupter.erdp.update_volatile(|erdp| {
                erdp.set_event_ring_dequeue_pointer(erdp.event_ring_dequeue_pointer())
            });
            interrupter
                .erstba
                .update_volatile(|erstba| erstba.set(erstba.get()));

            interrupter.imod.update_volatile(|i| {
                i.set_interrupt_moderation_interval(0)
                    .set_interrupt_moderation_counter(0);
            });
            register_irq(InterruptIndex::Xhci as usize, xhci_interrupt);
            interrupter.iman.update_volatile(|i| {
                i.set_0_interrupt_pending().set_interrupt_enable();
            });

            operational.usbcmd.update_volatile(|usb_command_register| {
                usb_command_register.set_interrupter_enable();
            });
            operational.usbcmd.update_volatile(|usb_command_register| {
                usb_command_register.set_run_stop();
            });
            while operational.usbsts.read_volatile().hc_halted() {}
        }
    }
}

fn xhci_interrupt(_irq: usize, _frame: InterruptStackFrame) {
    log::info!("xhci interrupt");
}
