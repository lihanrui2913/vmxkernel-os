use acpi::PciConfigRegions;
use alloc::alloc::Global;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use capability::PciCapability;
use core::fmt::Display;
use core::{fmt, ptr};
use device_type::DeviceType;
use pci_types::*;
use spin::Lazy;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::{PhysAddr, VirtAddr};

use crate::{arch::acpi::ACPI, memory::convert_physical_to_virtual};

pub static PCI_ACCESS: Lazy<PciAccess> = Lazy::new(|| PciAccess::new(&ACPI.pci_regions));

pub static PCI_DEVICES: Lazy<BTreeMap<PciAddress, PciDevice>> =
    Lazy::new(|| PciResolver::resolve(PCI_ACCESS.clone()));

pub fn get_device_by_class_code(class: BaseClass, sub_class: SubClass) -> Vec<&'static PciDevice> {
    let devices = PCI_DEVICES.iter();
    let mut result = Vec::new();
    for (_, device) in devices {
        if device.class == class && device.sub_class == sub_class {
            result.push(device);
        }
    }
    result
}

pub fn init() {
    PCI_DEVICES
        .iter()
        .for_each(|(_, device)| log::info!("{}", device));

    // now enable SSE and the like
    let mut cr0 = Cr0::read();
    cr0.set(Cr0Flags::EMULATE_COPROCESSOR, false);
    cr0.set(Cr0Flags::MONITOR_COPROCESSOR, true);
    unsafe { Cr0::write(cr0) };

    let mut cr4 = Cr4::read();
    cr4.set(Cr4Flags::OSFXSR, true);
    cr4.set(Cr4Flags::OSXMMEXCPT_ENABLE, true);
    unsafe { Cr4::write(cr4) };

    log::info!("PCI initialized successfully!");
}

#[derive(Clone, Copy)]
pub struct PciAccess<'a>(&'a PciConfigRegions<'a, Global>);

impl<'a> PciAccess<'a> {
    pub fn new(regions: &'a PciConfigRegions<'a, Global>) -> Self {
        Self(regions)
    }

    pub fn mmio_address(&self, address: PciAddress, offset: u16) -> VirtAddr {
        let (segment, bus, device, function) = (
            address.segment(),
            address.bus(),
            address.device(),
            address.function(),
        );

        let physical_address = self
            .0
            .physical_address(segment, bus, device, function)
            .expect("Invalid PCI address");

        convert_physical_to_virtual(PhysAddr::new(physical_address)) + offset as u64
    }
}

impl<'a> ConfigRegionAccess for PciAccess<'a> {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let address = self.mmio_address(address, offset);
        ptr::read_volatile(address.as_ptr())
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let address = self.mmio_address(address, offset);
        ptr::write_volatile(address.as_mut_ptr(), value);
    }
}

#[derive(Debug)]
pub struct PciDevice {
    pub address: PciAddress,
    pub vendor_id: VendorId,
    pub device_id: DeviceId,
    pub revision: DeviceRevision,
    pub class: BaseClass,
    pub sub_class: SubClass,
    pub interface: Interface,
    pub bars: [Option<Bar>; MAX_BARS],
}

impl Display for PciDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{}.{}: {:?} [{:04x}:{:04x}] (rev: {:02x})",
            self.address.bus(),
            self.address.device(),
            self.address.function(),
            DeviceType::from((self.class, self.sub_class)),
            self.vendor_id,
            self.device_id,
            self.revision,
        )
    }
}

pub struct PciResolver<'a> {
    access: PciAccess<'a>,
    devices: BTreeMap<PciAddress, PciDevice>,
}

impl<'a> PciResolver<'a> {
    fn resolve(access: PciAccess<'a>) -> BTreeMap<PciAddress, PciDevice> {
        let mut resolver = Self {
            access,
            devices: BTreeMap::new(),
        };

        for region in resolver.access.0.iter() {
            resolver.scan_segment(region.segment_group);
        }

        resolver.devices
    }

    fn scan_segment(&mut self, segment: u16) {
        self.scan_bus(segment, 0);

        let address = PciAddress::new(segment, 0, 0, 0);
        if PciHeader::new(address).has_multiple_functions(&self.access) {
            (1..8).for_each(|i| self.scan_bus(segment, i));
        }
    }

    fn scan_bus(&mut self, segment: u16, bus: u8) {
        (0..32).for_each(|device| {
            let address = PciAddress::new(segment, bus, device, 0);
            self.scan_function(segment, bus, device, 0);

            let header = PciHeader::new(address);
            if header.has_multiple_functions(&self.access) {
                (1..8).for_each(|function| {
                    self.scan_function(segment, bus, device, function);
                });
            }
        });
    }

    fn scan_function(&mut self, segment: u16, bus: u8, device: u8, function: u8) {
        let address = PciAddress::new(segment, bus, device, function);
        let header = PciHeader::new(address);

        let (vendor_id, device_id) = header.id(&self.access);
        let (revision, class, sub_class, interface) = header.revision_and_class(&self.access);

        if vendor_id == 0xffff {
            return;
        }

        let handle_bar = |header: &EndpointHeader| {
            let mut bars = [None; 6];
            let mut skip_next = false;

            for (index, bar_slot) in bars.iter_mut().enumerate() {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                let bar = header.bar(index as u8, &self.access);
                if let Some(Bar::Memory64 { .. }) = bar {
                    skip_next = true;
                }
                *bar_slot = bar;
            }

            bars
        };

        match header.header_type(&self.access) {
            HeaderType::Endpoint => {
                let endpoint_header = EndpointHeader::from_header(header, &self.access)
                    .expect("Invalid endpoint header");

                let bars = handle_bar(&endpoint_header);

                endpoint_header.capabilities(&self.access).for_each(
                    |capability| match capability {
                        PciCapability::Msi(msi) => {
                            msi.set_enabled(true, PCI_ACCESS.clone());
                        }
                        PciCapability::MsiX(mut msix) => {
                            msix.set_enabled(true, PCI_ACCESS.clone());
                            // let table_bar = bars[msix.table_bar() as usize].unwrap();
                        }
                        _ => {}
                    },
                );

                let device = PciDevice {
                    address,
                    class,
                    sub_class,
                    interface,
                    vendor_id,
                    device_id,
                    revision,
                    bars,
                };

                self.devices.insert(address, device);
            }
            HeaderType::PciPciBridge => {
                let bridge_header = PciPciBridgeHeader::from_header(header, &self.access)
                    .expect("Invalid PCI-PCI bridge header");

                let start_bus = bridge_header.secondary_bus_number(&self.access);
                let end_bus = bridge_header.subordinate_bus_number(&self.access);

                for bus_id in start_bus..=end_bus {
                    self.scan_bus(segment, bus_id);
                }
            }
            _ => {}
        }
    }
}
