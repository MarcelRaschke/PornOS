//! Includes the different paging implementation.
mod frame_allocator;
mod physical_mmap;
mod utils;

use core::{arch::asm, marker::PhantomData};

use spin::Once;
use x86_64::{
    structures::paging::{
        page_table::PageTableLevel, FrameAllocator, Page, PageSize, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use self::{
    frame_allocator::FRAME_ALLOCATOR, physical_mmap::KernelAndModulesIterator,
    utils::table_wrapper::TableWrapper,
};

use crate::{memory::HHDM, println};

lazy_static::lazy_static! {
    pub static ref HEAP_START: VirtAddr = VirtAddr::new(0x1000);
}

/// The amount of pages which should be used in the beginning for the stack.
/// == 64KiB
const STACK_INIT_PAGES: u64 = 16;
pub static STACK_START: Once<VirtAddr> = Once::new();

pub fn init() -> ! {
    println!("HHDM: {:x}", HHDM.as_u64());
    let p_configurator = KPagingConfigurator::<Size4KiB>::new();
    p_configurator.map_kernel();
    p_configurator.map_heap();
    p_configurator.map_stack();
    p_configurator.map_frame_allocator();
    p_configurator.switch_paging();

    panic!("Bruh");

    crate::init();
}

#[cfg(feature = "test")]
pub fn tests() {
    frame_allocator::tests();
}

/// The paging configurator which sets up the different paging levels.
///
/// # SAFETY
/// It assumes, that we are still using the paging table of Limine!!!!
#[derive(Debug, Clone)]
pub struct KPagingConfigurator<P: PageSize> {
    size: PhantomData<P>,
    p4_ptr: *mut PageTable,
    p4_phys_addr: PhysAddr,
}

impl<P: PageSize> KPagingConfigurator<P> {
    pub fn new() -> Self {
        let pml4e_addr = FRAME_ALLOCATOR
            .write()
            .allocate_frame()
            .unwrap()
            .start_address();
        let pml4e_virt_addr = *HHDM + pml4e_addr.as_u64();
        let ptr = pml4e_virt_addr.as_mut_ptr() as *mut PageTable;

        Self {
            size: PhantomData,
            p4_phys_addr: pml4e_addr,
            p4_ptr: ptr,
        }
    }

    /// This maps the kernel and its modules to the same virtual address as the given virtual
    /// address of limine.
    ///
    /// SHOULD BE FINE
    pub fn map_kernel(&self) {
        for kmmap in KernelAndModulesIterator::new() {
            for offset in (0..kmmap.len).step_by(P::SIZE.try_into().unwrap()) {
                let page_frame = {
                    let page_frame_phys_addr = PhysAddr::new(kmmap.base + offset);
                    PhysFrame::from_start_address(page_frame_phys_addr).unwrap()
                };
                let page = {
                    let page_frame_virt_addr = *HHDM + page_frame.start_address().as_u64();
                    Page::from_start_address(page_frame_virt_addr).unwrap()
                };

                self.map_page(page, Some(page_frame));
            }
        }
    }

    /// Maps a heap for the kernel.
    ///
    /// SHOULD WORK
    pub fn map_heap(&self) {
        let heap_page = Page::from_start_address(*HEAP_START).unwrap();

        self.map_page(heap_page, None);
    }

    /// Creates a new stack mapping for the kernel.
    pub fn map_stack(&self) {
        // "- P::SIZE" to let the stack start in the allocated frame
        STACK_START.call_once(|| {
            VirtAddr::new((HHDM.as_u64() - 1) & ((1 << 48) - 1)).align_down(4u64)
        });
        let mut addr = *STACK_START.get().unwrap();

        for _page_num in 0..STACK_INIT_PAGES {
            let page = Page::from_start_address(addr.align_down(P::SIZE)).unwrap();

            self.map_page(page, None);

            addr -= P::SIZE;
        }
    }

    pub fn map_frame_allocator(&self) {
        for page_frame in FRAME_ALLOCATOR.read().get_frame_allocator_page_frames() {
            let page: Page = {
                let page_addr = *HHDM + page_frame.start_address().as_u64();
                Page::from_start_address(page_addr).unwrap()
            };

            self.map_page(page, Some(page_frame));
        }
    }
}

impl<P: PageSize> KPagingConfigurator<P> {
    pub fn switch_paging(&self) {
        let p4_phys_addr = self.p4_phys_addr.as_u64();
        let stack_start = STACK_START.get().unwrap().as_u64();
        unsafe {
            asm! {
                "xor r8, r8",
                "mov r9, {1}",
                "mov r8, {0}",
                "mov rsp, r9",
                "mov cr3, r8",
                in(reg) p4_phys_addr,
                in(reg) stack_start,
                inout("r8") 0 => _,
                inout("r9") 0 => _,
            }
        }
    }
}

impl<P: PageSize> KPagingConfigurator<P> {
    /// Maps the given virtual page to the given physical page-frame if it's set.
    /// If `page_frame` is `None` a new page frame will be mapped to the given page.
    pub fn map_page(&self, page: Page, page_frame: Option<PhysFrame>) {
        let p1_index = page.p1_index();
        let mut p1_table = self.get_p1_table(page);

        if let Some(page_frame) = page_frame {
            p1_table.set_page_frame(p1_index, page_frame);
        } else {
            p1_table.create_entry(p1_index);
        }
    }

    /// Returns a table wrapper of the PageTable in the last level according to the given page.
    ///
    /// * `page`: The page where to read the different levels from.
    fn get_p1_table(&self, page: Page) -> TableWrapper {
        let mut table_wrapper = TableWrapper::new(self.p4_ptr);
        let mut level = PageTableLevel::Four;

        while let Some(lower_level) = level.next_lower_level() {
            let entry_index = match lower_level {
                PageTableLevel::Three => page.start_address().p4_index(),
                PageTableLevel::Two => page.start_address().p3_index(),
                PageTableLevel::One => page.start_address().p2_index(),
                _ => unreachable!("Ayo, '{:?}' shouldn't be here <.<", lower_level),
            };
            let table_entry = table_wrapper.get_entry(entry_index);

            let next_table_ptr = {
                let next_table_vtr_ptr = if table_entry.is_unused() {
                    let new_table_entry = table_wrapper.create_entry(entry_index);
                    *HHDM + new_table_entry.addr().as_u64()
                } else {
                    *HHDM + table_entry.addr().as_u64()
                };
                next_table_vtr_ptr.as_mut_ptr() as *mut PageTable
            };

            table_wrapper = TableWrapper::new(next_table_ptr);
            level = lower_level;
        }

        table_wrapper
    }
}
