use lapic::VirtLocalApic;
use rvm::arch::{VmxExitInfo, VmxExitReason};
use rvm::{RvmError, RvmHal, RvmPerCpu, RvmResult, RvmVcpu};
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags, Efer, EferFlags};
use x86_64::{
    structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame},
    PhysAddr, VirtAddr,
};

use crate::memory::{
    convert_physical_to_virtual, convert_virtual_to_physical, MappingType, MemoryManager,
    FRAME_ALLOCATOR,
};

const CR0: u64 = Cr0Flags::PROTECTED_MODE_ENABLE.bits()
    | Cr0Flags::MONITOR_COPROCESSOR.bits()
    | Cr0Flags::TASK_SWITCHED.bits()
    | Cr0Flags::NUMERIC_ERROR.bits()
    | Cr0Flags::WRITE_PROTECT.bits()
    | Cr0Flags::PAGING.bits();
const CR4: u64 = Cr4Flags::PHYSICAL_ADDRESS_EXTENSION.bits() | Cr4Flags::PAGE_GLOBAL.bits();
const EFER: u64 = EferFlags::LONG_MODE_ENABLE.bits() | EferFlags::NO_EXECUTE_ENABLE.bits();

type Vcpu = RvmVcpu<RvmHalImpl>;

pub struct RvmHalImpl;

impl RvmHal for RvmHalImpl {
    fn alloc_page() -> Option<rvm::HostPhysAddr> {
        let frame = FRAME_ALLOCATOR.lock().allocate_frame();
        if let Some(frame) = frame {
            Some(frame.start_address().as_u64() as rvm::HostPhysAddr)
        } else {
            None
        }
    }

    fn current_time_nanos() -> u64 {
        0
    }

    fn dealloc_page(paddr: rvm::HostPhysAddr) {
        unsafe {
            FRAME_ALLOCATOR
                .lock()
                .deallocate_frame(PhysFrame::containing_address(PhysAddr::new(paddr as u64)));
        }
    }

    fn phys_to_virt(paddr: rvm::HostPhysAddr) -> rvm::HostVirtAddr {
        convert_physical_to_virtual(PhysAddr::new(paddr as u64)).as_u64() as rvm::HostVirtAddr
    }

    fn virt_to_phys(vaddr: rvm::HostVirtAddr) -> rvm::HostPhysAddr {
        convert_virtual_to_physical(VirtAddr::new(vaddr as u64)).as_u64() as rvm::HostPhysAddr
    }

    fn vmexit_handler(vcpu: &mut rvm::RvmVcpu<Self>) {
        let exit_info = vcpu.exit_info().unwrap();
        log::warn!("VM exit: {:#x?}", exit_info);

        if exit_info.entry_failure {
            panic!("VM entry failed: {:#x?}", exit_info);
        }

        let res = match exit_info.exit_reason {
            VmxExitReason::EXTERNAL_INTERRUPT => handle_external_interrupt(vcpu),
            VmxExitReason::INTERRUPT_WINDOW => vcpu.set_interrupt_window(false),
            VmxExitReason::CPUID => handle_cpuid(vcpu),
            VmxExitReason::VMCALL => handle_hypercall(vcpu),
            VmxExitReason::IO_INSTRUCTION => handle_io_instruction(vcpu, &exit_info),
            VmxExitReason::MSR_READ => handle_msr_read(vcpu),
            VmxExitReason::MSR_WRITE => handle_msr_write(vcpu),
            VmxExitReason::EPT_VIOLATION => handle_ept_violation(vcpu, exit_info.guest_rip),
            _ => panic!(
                "Unhandled VM-Exit reason {:?}:\n{:#x?}",
                exit_info.exit_reason, vcpu
            ),
        };

        if res.is_err() {
            panic!(
                "Failed to handle VM-exit {:?}:\n{:#x?}",
                exit_info.exit_reason, vcpu
            );
        }
    }
}

const VM_EXIT_INSTR_LEN_CPUID: u8 = 2;
const VM_EXIT_INSTR_LEN_RDMSR: u8 = 2;
const VM_EXIT_INSTR_LEN_WRMSR: u8 = 2;
const VM_EXIT_INSTR_LEN_VMCALL: u8 = 3;

fn handle_external_interrupt(vcpu: &mut Vcpu) -> RvmResult {
    let int_info = vcpu.interrupt_exit_info()?;
    log::warn!("VM-exit: external interrupt: {:#x?}", int_info);
    assert!(int_info.valid);
    // crate::arch::handle_irq(int_info.vector);
    Ok(())
}

fn handle_cpuid(vcpu: &mut Vcpu) -> RvmResult {
    use raw_cpuid::{cpuid, CpuIdResult};

    const LEAF_FEATURE_INFO: u32 = 0x1;
    const LEAF_HYPERVISOR_INFO: u32 = 0x4000_0000;
    const LEAF_HYPERVISOR_FEATURE: u32 = 0x4000_0001;
    const VENDOR_STR: &[u8; 12] = b"RVMRVMRVMRVM";
    let vendor_regs = unsafe { &*(VENDOR_STR.as_ptr() as *const [u32; 3]) };

    let regs = vcpu.regs_mut();
    let function = regs.rax as u32;
    let res = match function {
        LEAF_FEATURE_INFO => {
            const FEATURE_VMX: u32 = 1 << 5;
            const FEATURE_HYPERVISOR: u32 = 1 << 31;
            let mut res = cpuid!(regs.rax, regs.rcx);
            res.ecx &= !FEATURE_VMX;
            res.ecx |= FEATURE_HYPERVISOR;
            res
        }
        LEAF_HYPERVISOR_INFO => CpuIdResult {
            eax: LEAF_HYPERVISOR_FEATURE,
            ebx: vendor_regs[0],
            ecx: vendor_regs[1],
            edx: vendor_regs[2],
        },
        LEAF_HYPERVISOR_FEATURE => CpuIdResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        },
        _ => cpuid!(regs.rax, regs.rcx),
    };

    log::debug!(
        "VM exit: CPUID({:#x}, {:#x}): {:?}",
        regs.rax,
        regs.rcx,
        res
    );
    regs.rax = res.eax as _;
    regs.rbx = res.ebx as _;
    regs.rcx = res.ecx as _;
    regs.rdx = res.edx as _;
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_CPUID)?;
    Ok(())
}

fn handle_hypercall(vcpu: &mut Vcpu) -> RvmResult {
    let regs = vcpu.regs();
    log::info!(
        "VM exit: VMCALL({:#x}): {:?}",
        regs.rax,
        [regs.rdi, regs.rsi, regs.rdx, regs.rcx]
    );
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_VMCALL)?;
    Ok(())
}

fn handle_io_instruction(vcpu: &mut Vcpu, exit_info: &VmxExitInfo) -> RvmResult {
    let io_info = vcpu.io_exit_info()?;
    log::warn!(
        "VM exit: I/O instruction @ {:#x}: {:#x?}",
        exit_info.guest_rip,
        io_info,
    );
    if io_info.is_string {
        log::error!("INS/OUTS instructions are not supported!");
        return Err(RvmError::Unsupported);
    }
    if io_info.is_repeat {
        log::error!("REP prefixed I/O instructions are not supported!");
        return Err(RvmError::Unsupported);
    }

    if let Some(dev) = device_emu::all_virt_devices().find_port_io_device(io_info.port) {
        if io_info.is_in {
            let value = dev.read(io_info.port, io_info.access_size)?;
            let rax = &mut vcpu.regs_mut().rax;
            // SDM Vol. 1, Section 3.4.1.1:
            // * 32-bit operands generate a 32-bit result, zero-extended to a 64-bit result in the
            //   destination general-purpose register.
            // * 8-bit and 16-bit operands generate an 8-bit or 16-bit result. The upper 56 bits or
            //   48 bits (respectively) of the destination general-purpose register are not modified
            //   by the operation.
            match io_info.access_size {
                1 => *rax = (*rax & !0xff) | (value & 0xff) as u64,
                2 => *rax = (*rax & !0xffff) | (value & 0xffff) as u64,
                4 => *rax = value as u64,
                _ => unreachable!(),
            }
        } else {
            let rax = vcpu.regs().rax;
            let value = match io_info.access_size {
                1 => rax & 0xff,
                2 => rax & 0xffff,
                4 => rax,
                _ => unreachable!(),
            } as u32;
            dev.write(io_info.port, io_info.access_size, value)?;
        }
    } else {
        panic!(
            "Unsupported I/O port {:#x} access: {:#x?}",
            io_info.port, io_info
        )
    }
    vcpu.advance_rip(exit_info.exit_instruction_length as _)?;
    Ok(())
}

fn handle_msr_read(vcpu: &mut Vcpu) -> RvmResult {
    let msr = vcpu.regs().rcx as u32;

    use x86::msr::*;
    let res = if msr == IA32_APIC_BASE {
        let mut apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
        apic_base |= 1 << 11 | 1 << 10; // enable xAPIC and x2APIC
        Ok(apic_base)
    } else if VirtLocalApic::msr_range().contains(&msr) {
        VirtLocalApic::rdmsr(vcpu, msr)
    } else {
        Err(RvmError::Unsupported)
    };

    if let Ok(value) = res {
        log::debug!("VM exit: RDMSR({:#x}) -> {:#x}", msr, value);
        vcpu.regs_mut().rax = value & 0xffff_ffff;
        vcpu.regs_mut().rdx = value >> 32;
    } else {
        panic!("Failed to handle RDMSR({:#x}): {:?}", msr, res);
    }
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_RDMSR)?;
    Ok(())
}

fn handle_msr_write(vcpu: &mut Vcpu) -> RvmResult {
    let msr = vcpu.regs().rcx as u32;
    let value = (vcpu.regs().rax & 0xffff_ffff) | (vcpu.regs().rdx << 32);
    log::debug!("VM exit: WRMSR({:#x}) <- {:#x}", msr, value);

    use x86::msr::*;
    let res = if msr == IA32_APIC_BASE {
        Ok(()) // ignore
    } else if VirtLocalApic::msr_range().contains(&msr) {
        VirtLocalApic::wrmsr(vcpu, msr, value)
    } else {
        Err(RvmError::Unsupported)
    };

    if res.is_err() {
        panic!(
            "Failed to handle WRMSR({:#x}) <- {:#x}: {:?}",
            msr, value, res
        );
    }
    vcpu.advance_rip(VM_EXIT_INSTR_LEN_WRMSR)?;
    Ok(())
}

fn handle_ept_violation(vcpu: &Vcpu, guest_rip: usize) -> RvmResult {
    let fault_info = vcpu.nested_page_fault_info()?;
    panic!(
        "VM exit: EPT violation @ {:#x}, fault_paddr={:#x}, access_flags=({:?})",
        guest_rip, fault_info.fault_guest_paddr, fault_info.access_flags
    );
}

pub mod device_emu;
pub mod lapic;

pub fn init() {
    unsafe {
        Cr0::write_raw(CR0);
        Cr4::write_raw(CR4);
        Efer::write_raw(EFER);
    }
}

pub fn run_vm(entry_address: usize) -> ! {
    let mut percpu = RvmPerCpu::<RvmHalImpl>::new(0);
    percpu.hardware_enable().unwrap();

    let new_page_table_address = FRAME_ALLOCATOR
        .lock()
        .allocate_frame()
        .unwrap()
        .start_address()
        .as_u64() as usize;

    <MemoryManager>::map_phys(entry_address, 0x1000, MappingType::KernelData.flags());

    let mut vcpu = percpu
        .create_vcpu(entry_address, new_page_table_address)
        .unwrap();

    log::info!("Running guest");

    vcpu.run();
}
