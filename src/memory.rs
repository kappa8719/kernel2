use crate::{__kernel_base, __stack_top, ld_variable};
use bitflags::bitflags;
use core::fmt::{Formatter, LowerHex, UpperHex};
use core::ops::{Add, AddAssign, Range};
use core::ptr;

macro_rules! impl_addr {
    ($ty: ty) => {
        impl $ty {
            pub fn addr(self) -> usize {
                self.0
            }

            pub unsafe fn as_ptr(self) -> *const u8 {
                self.0 as *const u8
            }

            pub unsafe fn as_mut_ptr(self) -> *mut u8 {
                self.0 as *mut u8
            }

            pub const fn zero() -> Self {
                Self(0)
            }
        }

        impl UpperHex for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                UpperHex::fmt(&self.0, f)
            }
        }

        impl LowerHex for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                LowerHex::fmt(&self.0, f)
            }
        }

        impl From<usize> for $ty {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }

        impl From<$ty> for usize {
            fn from(value: $ty) -> Self {
                value.0
            }
        }

        impl Add for $ty {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl AddAssign for $ty {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }
    };
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
pub struct PAddr(pub usize);
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
pub struct VAddr(pub usize);

impl_addr!(PAddr);
impl_addr!(VAddr);

#[derive(Clone, Debug)]
pub struct Region {
    pub addr: PAddr,
    pub size: usize,
}

impl Region {
    pub fn end(&self) -> PAddr {
        self.addr + self.size.into()
    }
}

pub const PAGE_SIZE: usize = 4096;
static mut CURRENT_REGION: Option<Region> = None;
static mut NEXT_PAGE_ADDR: PAddr = PAddr::zero();

fn exclude_range_from_range<T: PartialOrd + Copy>(
    base: &Range<T>,
    other: &Range<T>,
) -> [Option<Range<T>>; 2] {
    let mut ranges: [_; 2] = [None, None];

    // No overlap or other range is before/after this range
    if base.end <= other.start || other.end <= base.start {
        return [Some(base.clone()), None];
    }

    // Left part of self (if any)
    if base.start < other.start {
        ranges[0] = Some(base.start..other.start);
    }

    // Right part of self (if any)
    if base.end > other.end {
        ranges[1] = Some(other.end..base.end);
    }

    ranges
}

/// select memory region from loaded fdt and set as global
///
/// currently this function simply selects the largest region
pub fn set_region_from_fdt() {
    // let region = unsafe {
    //     Region {
    //         addr: PAddr(ld_variable!(__free_ram, u8)),
    //         size: ld_variable!(__free_ram_end, u8) - ld_variable!(__free_ram, u8),
    //     }
    // };
    //
    // unsafe {
    //     NEXT_PAGE_ADDR = region.addr;
    //     CURRENT_REGION = Some(region);
    // };
    //
    // return;

    let kernel_reserved_range =
        unsafe { ld_variable!(__kernel_base, u8)..ld_variable!(__stack_top, usize) };
    let mut largest_region: Option<Region> = None;
    for region in crate::dtb::fdt().memory().regions() {
        let base = (region.starting_address as usize)
            ..(region.starting_address as usize + region.size.unwrap());
        let base = ((base.start + 4096 - 1) & !(4096 - 1))..((base.end + 4096 - 1) & !(4096 - 1));
        let regions = exclude_range_from_range(&base, &kernel_reserved_range);
        for region in regions.into_iter().flatten() {
            let size = region.end - region.start;
            let region = Region {
                addr: PAddr(region.start + PAGE_SIZE),
                size: region.end - region.start,
            };

            match largest_region {
                None => largest_region = Some(region),
                Some(ref mut largest_region) => {
                    if largest_region.size < size {
                        *largest_region = region
                    }
                }
            }
        }
    }

    let region = largest_region.unwrap();

    unsafe {
        NEXT_PAGE_ADDR = region.addr;
        CURRENT_REGION = Some(region);
    };
}

pub fn get_region() -> &'static Region {
    #[allow(static_mut_refs)]
    unsafe {
        CURRENT_REGION.as_ref().unwrap()
    }
}

/// allocate [n] pages and return the start address
pub fn allocate(n: usize) -> PAddr {
    let addr = unsafe { NEXT_PAGE_ADDR };
    unsafe {
        NEXT_PAGE_ADDR += PAddr(n * PAGE_SIZE);

        #[allow(static_mut_refs)]
        if NEXT_PAGE_ADDR > CURRENT_REGION.clone().unwrap().end() {
            panic!("out of current memory region");
        }

        ptr::write_bytes(addr.as_mut_ptr(), 0, n * PAGE_SIZE / size_of::<u8>());
    }

    addr
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct PageFlag: usize {
        const Dirty = 1 << 7;
        const Accessed = 1 << 6;
        const Global = 1 << 5;
        const User = 1 << 4;
        const Execute = 1 << 3;
        const Write = 1 << 2;
        const Read = 1 << 1;
        const Valid = 1 << 0;

        const ReadWriteExecute = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
    }
}

/// Sv32: VPN1(10bits) + VPN2(10bits) + Offset(12bits)
pub fn map_page_sv32(table1: PAddr, vaddr: VAddr, paddr: PAddr, flags: PageFlag) {
    unsafe {
        if !vaddr.as_ptr().is_aligned() {
            panic!("unaligned vaddr {vaddr:#x}");
        };
        if !paddr.as_ptr().is_aligned() {
            panic!("unaligned paddr {paddr:#x}");
        };

        let table1 = table1.addr() as *mut usize;

        let vpn1 = ((vaddr.addr() >> 22) & 0x3ff) as isize;
        if (table1.offset(vpn1).read() & PageFlag::Valid.bits()) == 0 {
            // create second level virtual page table
            let page_table = allocate(1);
            table1
                .offset(vpn1)
                .write(((page_table.addr() / PAGE_SIZE) << 10) | PageFlag::Valid.bits());
        }

        let vpn0 = ((vaddr.addr() >> 12) & 0x3ff) as isize;
        let table0 = ((*table1.offset(vpn1) >> 10) * PAGE_SIZE) as *mut usize;
        table0
            .offset(vpn0)
            .write(((paddr.addr() / PAGE_SIZE) << 10) | (flags | PageFlag::Valid).bits())
    }
}

pub fn map_page_to_heap(table: PAddr, flags: PageFlag, mapper: fn(PAddr, VAddr, PAddr, PageFlag)) {
    #[allow(static_mut_refs)]
    let region = unsafe { CURRENT_REGION.as_ref().unwrap() };
    for paddr in (region.addr.addr()..(region.addr.addr() + region.size)).step_by(PAGE_SIZE) {
        // println!("[map_page_to_heap#] map {paddr:#x}");
        mapper(table, VAddr(paddr), PAddr(paddr), flags)
    }
}
