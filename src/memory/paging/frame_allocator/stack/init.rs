use crate::memory::paging::frame_allocator::stack::POINTER_SIZE;
use crate::memory::paging::physical_mmap::{self, UseableMemChunkIterator};
use crate::println;
use crate::{memory::paging::PhysMemMap, print};
use x86_64::structures::paging::{PageSize, Size4KiB};
use x86_64::PhysAddr;

use super::{Stack, StackIndex};

impl Stack {
    /// Creates a new frame-stack with the given arguments.
    pub fn new() -> Self {
        print!("Using Frame-Allocator-Stack ... ");
        let amount_page_frames = physical_mmap::get_amount_page_frames::<Size4KiB>();
        let stack_start = get_start_addr();
        let capacity = amount_page_frames;

        let mut stack = Self {
            start: stack_start,
            len: capacity,
            capacity,
            ..Self::default()
        };

        stack.add_entries();
        stack.swap_stack_frames();

        println!("OK");
        stack
    }

    /// Fills the stack with pointers to the page frames.
    fn add_entries(&self) {
        let mut entry_addr = self.start.as_u64();
        for mmap in UseableMemChunkIterator::new() {
            for readed_bytes in (0..mmap.len).step_by(Self::PAGE_SIZE) {
                let frame_addr = mmap.base + readed_bytes;
                let ptr = entry_addr as *mut u64;
                unsafe {
                    *ptr = frame_addr;
                }
                entry_addr += *POINTER_SIZE;
            }
        }
    }

    /// Moves the frames which the stack uses to the top of the stack.
    /// Then the stack reduces it's capacity to the first real free frame.
    ///
    /// This makes it possible to get the physical addresses of the stack-frames without the
    /// conflict of popping or pushing.
    fn swap_stack_frames(&mut self) {
        if let Some(stack_frame_index) = self.get_stack_frame_index() {
            let used_frames = self.get_used_frames();
            for index in stack_frame_index..stack_frame_index + used_frames {
                let used_frame_addr: *mut u64 = {
                    let addr = self.start.as_u64() + (POINTER_SIZE * index).as_u64();
                    addr as *mut u64
                };

                let free_frame_addr: *mut u64 = {
                    let addr =
                        self.start.as_u64() + (POINTER_SIZE * (index + used_frames)).as_u64();
                    addr as *mut u64
                };

                unsafe {
                    core::ptr::swap(used_frame_addr, free_frame_addr);
                }
            }

            self.capacity = physical_mmap::get_amount_page_frames::<Size4KiB>() - used_frames;
            self.len = self.capacity;
        }
    }

    /// Returns the stack index which holds the frame where the stack starts.
    ///
    /// # Return
    /// - `Some<StackIndex>`: If the given frame could be found.
    /// - `None`: If the frame isn't in the stack anymore.
    fn get_stack_frame_index(&self) -> Option<StackIndex> {
        for stack_index in 0..self.len {
            let frame_addr = self.get_entry(stack_index).unwrap();
            if frame_addr == self.start {
                return Some(stack_index);
            }
        }
        None
    }

    /// # Return
    /// The amount of frames which the stack uses.
    fn get_used_frames(&self) -> u64 {
        self.capacity.div_ceil(Size4KiB::SIZE)
    }
}

// FUTURE: It could happen, that we'll get the last frame because the other frames might
// be too small....
fn get_start_addr() -> PhysAddr {
    let amount_page_frames = physical_mmap::get_amount_page_frames::<Size4KiB>();
    let needed_free_space = POINTER_SIZE * amount_page_frames;

    for mmap in UseableMemChunkIterator::new() {
        let has_enough_space = mmap.len >= needed_free_space.as_u64();
        if has_enough_space {
            return PhysAddr::new(mmap.base);
        }
    }

    unreachable!("Bro, download some RAM: http://downloadramdownloadramdownloadram.com");
}
