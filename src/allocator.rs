use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::{self, Debug},
};

use crate::{memory::Region, println};

#[derive(Debug)]
struct LinkedNode<T = u16> {
    pub head: T,
    pub tail: T,
}

/// Metadata of a buddy block
///
/// # Bits structure
///
/// 1 bit        | 1 bit        | 6 bits
/// is_allocated | is_free_list | pool
///
/// represented in u8
#[derive(Clone, Copy)]
struct Metadata(u8);

impl Metadata {
    pub fn is_allocated(&self) -> bool {
        (self.0 >> 7) == 1
    }

    pub fn is_free_list(&self) -> bool {
        (self.0 >> 6 & 0b1) == 1
    }

    /// index of the pool
    pub fn pool(&self) -> u8 {
        self.0 & 0b111111
    }

    pub fn with_is_allocated(self, is_allocated: bool) -> Self {
        Self::from_value(self.0 & 0b01111111 & (is_allocated as u8))
    }

    pub fn with_is_free_list(self, is_free_list: bool) -> Self {
        Self::from_value(self.0 & 0b10111111 & (is_free_list as u8))
    }

    pub fn with_pool(self, pool: u8) -> Self {
        Self::from_value(self.0 & 0b11110000 & pool)
    }

    pub fn new(is_allocated: bool, is_free_list: bool, pool: u8) -> Self {
        Self((is_allocated as u8) << 7 | (is_free_list as u8) << 6 | pool)
    }

    pub fn from_value(value: u8) -> Self {
        Self(value)
    }
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("is_allocated", &self.is_allocated())
            .field("is_free_list", &self.is_free_list())
            .field("pool", &self.pool())
            .finish()
    }
}

/// Minimum block size of allocation in bytes
const MINIMUM_BLOCK: usize = 4096;
/// Maximum order of buddy
const MAXIMUM_ORDER: usize = 10;
/// Maximum block size of allocation in bytes
const MAXIMUM_BLOCK: usize = MINIMUM_BLOCK * 2usize.pow(MAXIMUM_ORDER as u32);

pub struct BuddyAllocator {
    free_lists: *mut [LinkedNode],
    metadata: *mut [Metadata],
    links: *mut [LinkedNode],
    orders: usize,
    subranges: usize,
}

impl Debug for BuddyAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            f.debug_struct("BuddyAllocator")
                .field("free_lists", &&(*self.free_lists))
                .field("metadata", &self.metadata.len())
                .field("links", &self.links.len())
                .finish()
        }
    }
}

impl BuddyAllocator {
    /// Creates an uninitialized BuddyAllocator
    const unsafe fn null() -> Self {
        Self {
            free_lists: core::ptr::slice_from_raw_parts_mut(core::ptr::null_mut(), 0),
            metadata: core::ptr::slice_from_raw_parts_mut(core::ptr::null_mut(), 0),
            links: core::ptr::slice_from_raw_parts_mut(core::ptr::null_mut(), 0),
            orders: 0,
            subranges: 0,
        }
    }

    pub fn new(region: Region, heap: &mut [u8]) -> Self {
        let required_heap = Self::get_required_heap(region.size);
        if size_of_val(heap) < required_heap {
            panic!("size of supplied heap is not enough")
        }
        let mut heap = heap.as_mut_ptr();
        let orders = MAXIMUM_ORDER;
        let subranges = region.size / MINIMUM_BLOCK;

        unsafe {
            // allocate free lists
            heap = heap.add(heap.align_offset(align_of::<LinkedNode>()));
            let size_of_free_lists = size_of::<LinkedNode>() * orders;
            core::ptr::write_bytes(heap, 0xFF, size_of_free_lists);
            let free_lists = core::slice::from_raw_parts_mut(heap as *mut LinkedNode, orders + 1);
            heap = heap.add(size_of_free_lists);

            heap = heap.add(heap.align_offset(align_of::<Metadata>()));
            let size_of_metadata_array = size_of::<Metadata>() * subranges;
            core::ptr::write_bytes(heap, 0, size_of_metadata_array);
            let metadata = core::slice::from_raw_parts_mut(heap as *mut Metadata, subranges);
            heap = heap.add(size_of_metadata_array);

            heap = heap.add(heap.align_offset(align_of::<LinkedNode>()));
            let size_of_link_array = size_of::<LinkedNode>() * subranges;
            core::ptr::write_bytes(heap, 0xFF, size_of_link_array);
            let links = core::slice::from_raw_parts_mut(heap as *mut LinkedNode, subranges);

            // initialize buddies
            {
                let step = MAXIMUM_BLOCK / MINIMUM_BLOCK;
                let max = subranges - step;
                for i in (0..=max).step_by(step) {
                    links[i].head = if i == 0 { u16::MAX } else { (i - step) as u16 };
                    links[i].tail = if i == max {
                        u16::MAX
                    } else {
                        (i + step) as u16
                    };
                    metadata[i] = Metadata::new(false, true, orders as u8);
                }
                free_lists[orders].head = 0;
                free_lists[orders].tail = max as u16;
            }

            Self {
                free_lists,
                metadata,
                links,
                orders,
                subranges,
            }
        }
    }

    pub fn get_required_heap(region: usize) -> usize {
        let orders = MAXIMUM_ORDER;
        let subranges = region / MINIMUM_BLOCK;
        let mut heap = core::ptr::null::<u8>();

        unsafe {
            heap = heap.add(heap.align_offset(align_of::<LinkedNode>()));
            heap = heap.add(size_of::<LinkedNode>() * orders); // free list array

            heap = heap.add(heap.align_offset(align_of::<Metadata>()));
            heap = heap.add(size_of::<Metadata>() * subranges); // metadata array

            heap = heap.add(heap.align_offset(align_of::<LinkedNode>()));
            heap = heap.add(size_of::<LinkedNode>() * subranges);
        }

        heap.addr()
    }

    pub fn allocate_unchecked(&self, size: usize) -> *mut u8 {
        let desired_order = (size.ilog2() - MINIMUM_BLOCK.ilog2()) as usize;

        // find free block of at least requested size
        let found_order = desired_order;
        let mut block = u16::MAX;
        for pool in desired_order..self.orders {
            let node = unsafe { &(*self.free_lists)[pool] };
            if node.head != u16::MAX {
                block = node.head;
                break;
            }
        }

        // return null if not found
        if block == u16::MAX {
            return core::ptr::null_mut();
        }

        let metadata = unsafe { &mut (*self.metadata)[block as usize] };
        *metadata = metadata.with_is_free_list(false);

        // split unused buddies and add them to free lists
        for pool in (desired_order..found_order).rev() {
            let buddy = block ^ (1 << found_order);
            let buddy_metadata = unsafe { &mut (*self.metadata)[buddy as usize] };
            *buddy_metadata = Metadata::new(false, true, pool as u8);
        }

        unimplemented!()
    }
}

unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}

static mut ALLOCATOR: BuddyAllocator = unsafe { BuddyAllocator::null() };

/// Initialize global allocator
pub unsafe fn initialize_global(region: Region, heap: &mut [u8]) {
    unsafe {
        ALLOCATOR = BuddyAllocator::new(region, heap);
        #[allow(static_mut_refs)]
        let allocator = &ALLOCATOR;
        println!("{allocator:#?}");
    }
}
