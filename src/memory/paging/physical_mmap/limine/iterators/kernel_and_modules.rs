use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, NonNullPtr};

use super::MemChunkIterator;

pub struct KernelAndModulesIterator(MemChunkIterator);

impl Iterator for KernelAndModulesIterator {
    type Item = &'static NonNullPtr<LimineMemmapEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .by_ref()
            .find(|&mmap| mmap.typ == LimineMemoryMapEntryType::KernelAndModules)
    }
}
