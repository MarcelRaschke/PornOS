//! This module contains the frame allocator.
//! Each submodule represents a diffrent way how to manage the page frames.
#[cfg(feature = "frame-allocator-array-stack")]
mod array_stack;

#[cfg(feature = "frame-allocator-bitflag")]
mod bitflag;

#[cfg(feature = "frame-allocator-stack")]
mod stack;

pub mod frame;

use spin::Once;
use x86_64::VirtAddr;

use self::{stack::Stack, frame::Frame};
use super::{page_size::PageSize, PhysMemMap, PhysLinearAddr};

pub static FRAME_ALLOCATOR: Once<FrameAllocator> = Once::new();

/// Sets up the frame allocator
pub fn init(phys_mmap: &PhysMemMap) {
    setup_frame_allocator(phys_mmap);
}

/// Each frame manager needs to implement those functions.
pub trait FrameManager: Send + Sync + core::fmt::Debug {
    /// Returns the starting address of a free frame.
    fn get_free_frame(&mut self) -> Option<Frame>;

    /// Marks the given starting address of a frame as free.
    fn free_frame(&mut self, addr: VirtAddr);
}

/// The main frame allocator struct which manages the frames.
#[derive(Debug)]
pub struct FrameAllocator {
    /// this saves the sizes of the pages
    page_size: PageSize,
    /// this stores the datastructure how the frames are stored.
    frame_manager: Stack,
}

impl FrameManager for FrameAllocator {
    /// Returns the starting address of a free frame.
    fn get_free_frame(&mut self) -> Option<Frame> {
        todo!();
    }

    fn free_frame(&mut self, _frame_addr: VirtAddr) {
        todo!()
    }
}

fn setup_frame_allocator(phys_mmap: &PhysMemMap) {
    let page_size = PageSize::Page4KB;

    FRAME_ALLOCATOR.call_once(|| FrameAllocator {
        page_size,
        frame_manager: Stack::new(PhysLinearAddr::new(0), phys_mmap, page_size),
    });
}
