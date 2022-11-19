mod frame_array;
mod frame_index;
mod frame_stack;

use crate::memory::{paging::{PhysLinearAddr, PhysMemMap, PageSize}};

use self::{frame_array::FrameArray, frame_stack::FrameStack};

use super::FrameManager;

pub use frame_index::{FrameIndex, FrameIndexByteIterator};
use x86_64::PhysAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayStack {
    /// stortes the current free frames
    stack: FrameStack,
    /// stores all available frames
    array: FrameArray,
}

impl ArrayStack {
    /// The starting address in the physical linear address space where its components should be
    /// stored.
    const START: PhysLinearAddr = PhysLinearAddr::new(0);
}

impl FrameManager for ArrayStack {
    fn new(phys_mmap: &PhysMemMap, page_size: PageSize) -> Self {
        let stack = FrameStack::new(Self::START, phys_mmap, page_size);
        let stack_capacity = stack.get_capacity();

        Self {
            stack,
            array: FrameArray::new(stack_capacity + 1, phys_mmap),
        }
    }

    fn get_free_frame(&mut self) -> PhysAddr {
        todo!()
    }

    fn free_frame(&mut self, _addr: PhysAddr) {
        todo!()
    }
}
