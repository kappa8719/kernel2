use crate::memory::{PAddr, PageFlag, VAddr};

pub struct PageTable {
    entries: [PageTableEntry; 1024],
}

#[derive(Copy, Clone)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    #[inline(always)]
    pub fn ppn1(self) -> usize {
        self.0 >> 20
    }

    #[inline(always)]
    pub fn ppn0(self) -> usize {
        // mask upper bits (which is ppn1)
        let mask = (1 << 10) - 1;
        self.0 >> 10 & mask
    }

    pub fn flags(self) -> PageFlag {
        let mask = (1 << 8) - 1;
        PageFlag::from_bits_truncate(self.0 & mask)
    }

    pub fn value(self) -> usize {
        self.0
    }

    pub fn with_flags(self, flags: PageFlag) -> Self {
        Self::from_value(self.0 >> 8 << 8 | flags.bits())
    }

    pub fn new(ppn1: usize, ppn0: usize, flags: PageFlag) -> Self {
        Self::from_value(ppn1 << 20 | ppn0 << 10 | flags.bits())
    }

    pub fn from_value(value: usize) -> Self {
        Self(value)
    }
}

pub fn map(root: &mut PageTable, vaddr: VAddr, paddr: PAddr, flags: PageFlag) {
    let vpn1 = (vaddr.addr() >> 22) & ((1 << 10) - 1);
    let vpn0 = (vaddr.addr() >> 12) & ((1 << 10) - 1);

    if root.entries[vpn1].value() & PageFlag::Valid.bits() == 0 {}
}
