use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::paging::{
    page_table::PageTableLevel, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size4KiB,
};

use crate::memory::{types::Bytes, HHDM};

use super::{utils::table_wrapper::TableWrapper, PML4E_ADDR};

lazy_static! {
    /// **S**uper **I**mpressive **M**a**p**per
    ///
    /// Yes, this is the mapper which maps page to page frames or in other
    /// words: The ultimate ***SIMP***
    pub static ref SIMP: Mutex<Mapper> = Mutex::new(Mapper::new());
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mapper {
    p4_ptr: *mut PageTable,
}

unsafe impl Send for Mapper {}

impl Mapper {
    fn new() -> Self {
        Self {
            p4_ptr: PML4E_ADDR.get().unwrap().as_u64() as *mut PageTable,
        }
    }
}

unsafe impl VMMMapper<Size4KiB> for Mapper {
    unsafe fn map_page(&self, page: Page, page_frame: Option<PhysFrame>, flags: PageTableFlags) {
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
                    let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;
                    table_wrapper.set_page_frame(entry_index, None, flags);
                    *HHDM + table_wrapper.get_entry(entry_index).addr().as_u64()
                } else {
                    *HHDM + table_entry.addr().as_u64()
                };
                next_table_vtr_ptr.as_mut_ptr() as *mut PageTable
            };

            table_wrapper = TableWrapper::new(next_table_ptr);
            level = lower_level;
        }

        table_wrapper.set_page_frame(page.p1_index(), page_frame, flags);
    }

    unsafe fn unmap_page(&self, page: Page) -> Result<PhysFrame, ()> {
        todo!()
    }

    /// Maps a range of pages in a romw.
    ///
    /// * `page`: The starting page which should be mapped.
    /// * `page_frame`: The starting page frame (if available) which should be mapped.
    ///                 If it's `None`, random page-frames are picked up then.
    /// * `len`: The amount of bytes which should be mapped in a row.
    /// * `flags`: The flags for each page.
    ///
    /// # Note
    /// If `page_frame` is `Some(...)`, then you **have to** make sure that, the range, starting
    /// from the given page frame until `start + len` is **a valid Physicall address range**!!!
    unsafe fn map_page_range(
        &self,
        page: Page,
        page_frame: Option<PhysFrame>,
        len: Bytes,
        flags: PageTableFlags,
    ) {
        for offset in (0..len.as_u64()).step_by(Size4KiB::SIZE.try_into().unwrap()) {
            let page = {
                let addr = (page.start_address() + offset).align_down(Size4KiB::SIZE);
                Page::from_start_address(addr).unwrap()
            };

            let page_frame = page_frame.map(|frame| {
                let addr = (frame.start_address() + offset).align_down(Size4KiB::SIZE);
                PhysFrame::from_start_address(addr).unwrap()
            });

            self.map_page(page, page_frame, flags);
        }
    }
}

pub unsafe trait VMMMapper<P: PageSize> {
    /// Maps a page to the given page_frame (if available) with the given flags.
    ///
    /// * `page`: The page to be mapped.
    /// * `page_frame`: If it's `Some`, then the page will be mapped to the given page frame,
    ///                 otherwise a new page frame will ba allocated.
    /// * `flags`: The flags for the given mapping.
    unsafe fn map_page(&self, page: Page, page_frame: Option<PhysFrame>, flags: PageTableFlags);

    /// Maps a range of pages in a romw.
    ///
    /// * `page`: The starting page which should be mapped.
    /// * `page_frame`: The starting page frame (if available) which should be mapped.
    ///                 If it's `None`, random page-frames are picked up then.
    /// * `len`: The amount of bytes which should be mapped in a row.
    /// * `flags`: The flags for each page.
    ///
    /// # Note
    /// If `page_frame` is `Some(...)`, then you **have to** make sure that, the range, starting
    /// from the given page frame until `start + len` is **a valid Physicall address range**!!!
    unsafe fn map_page_range(
        &self,
        page: Page,
        page_frame: Option<PhysFrame>,
        len: Bytes,
        flags: PageTableFlags,
    );

    /// Unmpas the given page and returns the unmapped page frame if everything
    /// works fine.
    ///
    /// * `page`: The page which should be unmapped.
    unsafe fn unmap_page(&self, page: Page) -> Result<PhysFrame, ()>;
}
