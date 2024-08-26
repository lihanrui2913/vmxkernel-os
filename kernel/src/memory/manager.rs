use core::marker::PhantomData;
use x86_64::instructions::interrupts;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size1GiB, Size2MiB};
use x86_64::structures::paging::{Mapper, OffsetPageTable, PageTableFlags};
use x86_64::structures::paging::{Page, PageSize, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

use super::{BitmapFrameAllocator, FRAME_ALLOCATOR, KERNEL_PAGE_TABLE};

pub enum MappingType {
    UserCode,
    KernelData,
    UserData,
}

#[rustfmt::skip]
impl MappingType {
    pub fn flags(&self) -> PageTableFlags {
        match self {
            Self::UserCode => PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE,
            Self::KernelData => PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_EXECUTE,
            Self::UserData => PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE,
        }
    }
}

pub struct MemoryManager<S: PageSize = Size4KiB> {
    size: PhantomData<S>,
}

impl<S: PageSize> MemoryManager<S> {
    pub fn alloc_range(
        start_address: VirtAddr,
        length: u64,
        flags: PageTableFlags,
        page_table: &mut OffsetPageTable<'static>,
    ) -> Result<(), MapToError<S>>
    where
        OffsetPageTable<'static>: Mapper<S>,
        BitmapFrameAllocator: FrameAllocator<S>,
    {
        interrupts::without_interrupts(|| {
            let page_range = {
                let start_page = Page::containing_address(start_address);
                let end_page = Page::containing_address(start_address + length - 1u64);
                Page::range_inclusive(start_page, end_page)
            };
            let mut frame_allocator = super::FRAME_ALLOCATOR.lock();

            for page in page_range {
                let frame = frame_allocator
                    .allocate_frame()
                    .ok_or(MapToError::FrameAllocationFailed)?;
                unsafe { page_table.map_to(page, frame, flags, &mut *frame_allocator) }
                    .map(|flush| flush.flush())?;
            }

            Ok(())
        })
    }

    pub fn dealloc_range(
        start_address: VirtAddr,
        length: u64,
        page_table: &mut OffsetPageTable<'static>,
    ) {
        interrupts::without_interrupts(|| {
            let page_range = {
                let start_page = Page::containing_address(start_address);
                let end_page = Page::containing_address(start_address + length - 1u64);
                Page::range_inclusive(start_page, end_page)
            };
            let mut frame_allocator = super::FRAME_ALLOCATOR.lock();
            for page in page_range {
                let (frame, mapper_flush) =
                    page_table.unmap(page).expect("Failed to deallocate frame");

                mapper_flush.flush();
                unsafe { frame_allocator.deallocate_frame(frame) };
            }
        })
    }

    /// Allocate memory for the DMA drivers, `cnt` is the number of physical memory frames you need.
    pub fn alloc_for_dma(cnt: usize) -> (PhysAddr, VirtAddr) {
        let phys = FRAME_ALLOCATOR.lock().allocate_frames(cnt).unwrap();
        let phys = PhysAddr::new(phys);
        let virt = crate::memory::convert_physical_to_virtual(phys);
        (phys, virt)
    }

    /// deallocates the physical memory.
    pub fn dealloc_for_dma(virt_addr: VirtAddr, _cnt: usize) {
        let phys = crate::memory::convert_virtual_to_physical(virt_addr);
        unsafe {
            FRAME_ALLOCATOR
                .lock()
                .deallocate_frame(PhysFrame::containing_address(phys));
        }
    }

    pub fn map_virt_to_phys(virt: usize, phys: usize, size: usize, flags: PageTableFlags) {
        for i in 0..(size / 4096) {
            Self::do_map_to(virt + i * 4096, phys + i * 4096, flags);
        }
    }

    pub fn do_map_to(virt: usize, phys: usize, flags: PageTableFlags) {
        let mut kernel_page_table = KERNEL_PAGE_TABLE.lock();

        let result = unsafe {
            kernel_page_table.map_to(
                Page::<Size4KiB>::containing_address(VirtAddr::new(virt as u64)),
                PhysFrame::containing_address(PhysAddr::new(phys as u64)),
                flags,
                &mut *FRAME_ALLOCATOR.lock(),
            )
        };

        match result {
            Err(err) => match err {
                MapToError::FrameAllocationFailed => panic!("Frame allocation failed!!!"),
                MapToError::PageAlreadyMapped(frame) => {
                    log::warn!("Page already mapped: frame: {:?}", frame);
                    kernel_page_table
                        .unmap(Page::<Size4KiB>::containing_address(VirtAddr::new(
                            virt as u64,
                        )))
                        .expect("Cannot unmap to")
                        .1
                        .flush();
                    Self::do_map_to(virt, phys, flags);
                }
                MapToError::ParentEntryHugePage => {
                    log::warn!("Parent entry huge page");
                    let result = kernel_page_table.unmap(Page::<Size2MiB>::containing_address(
                        VirtAddr::new(virt as u64),
                    ));

                    if let Ok((_frame, flusher)) = result {
                        flusher.flush();
                    } else {
                        let result = kernel_page_table.unmap(Page::<Size1GiB>::containing_address(
                            VirtAddr::new(virt as u64),
                        ));
                        if let Ok((_frame, flusher)) = result {
                            flusher.flush();
                        } else {
                            panic!("Cannot unmap huge page");
                        }
                    }
                    Self::do_map_to(virt, phys, flags);
                }
            },
            Ok(flusher) => flusher.flush(),
        }
    }
}
