use crate::{__kernel_base, __stack_top, ld_variable, println};
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

const PAGE_SIZE: usize = 4096;
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
    let kernel_reserved_range = unsafe { ld_variable!(__kernel_base, u8)..ld_variable!(__stack_top, usize) };
    let mut largest_region: Option<Region> = None;
    for region in crate::dtb::fdt().memory().regions() {
        let base = (region.starting_address as usize)
            ..(region.starting_address as usize + region.size.unwrap());
        let regions = exclude_range_from_range(&base, &kernel_reserved_range);
        for region in regions {
            if let Some(region) = region {
                let size = region.end - region.start;
                match largest_region {
                    None => {
                        largest_region = Some(Region {
                            addr: region.start.into(),
                            size: region.end - region.start,
                        })
                    }
                    Some(ref mut largest_region) => {
                        if largest_region.size < size {
                            *largest_region = Region {
                                addr: region.start.into(),
                                size: region.end - region.start,
                            }
                        }
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

/// allocate [n] pages and return the start address
pub fn allocate(n: usize) -> PAddr {
    let addr = unsafe { NEXT_PAGE_ADDR };
    unsafe {
        NEXT_PAGE_ADDR += PAddr(n * PAGE_SIZE);

        #[allow(static_mut_refs)]
        if NEXT_PAGE_ADDR > CURRENT_REGION.clone().unwrap().end() {
            panic!("out of current memory region");
        }

        println!("writing zeros for created page at {addr:#x}");
        ptr::write_bytes(addr.as_mut_ptr(), 0, n * PAGE_SIZE / size_of::<u8>());
    }

    addr
}
