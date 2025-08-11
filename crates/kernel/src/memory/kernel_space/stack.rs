use alloc::vec::Vec;
use core::ops::Range;

use spin::{Once, mutex::SpinMutex};

const STACK_SIZE: usize = 128 * 1024;
const STACK_PADDING_SIZE: usize = 128 * 1024;
const STACK_ARENA_SIZE: usize = 1024 * 1024 * 1024;
const NUM_STACK_SLOTS: usize = STACK_ARENA_SIZE / (STACK_SIZE + STACK_PADDING_SIZE);
const STACK_ARENA_START: usize = 0xffff_ffc0_0000_0000;
const STACK_ARENA_END: usize = STACK_ARENA_START + STACK_ARENA_SIZE;

static STACK_SLOT_ALLOCATOR: Once<SpinMutex<StackSlotAllocator>> = Once::new();

type AllocatorChunk = u128;
const CHUNK_BITS: usize = 128;

pub fn init() {
    STACK_SLOT_ALLOCATOR.call_once(|| SpinMutex::new(StackSlotAllocator::new()));
}

struct StackSlotAllocator {
    allocated_slots: Vec<AllocatorChunk>,
    next_search_slot: usize,
}

impl StackSlotAllocator {
    #[must_use]
    fn new() -> Self {
        let allocated_slots = alloc::vec![0; NUM_STACK_SLOTS.div_ceil(CHUNK_BITS)];
        Self {
            allocated_slots,
            next_search_slot: 0,
        }
    }

    fn slot_bit(&self, slot: usize) -> bool {
        let (chunk, bit) = (slot / CHUNK_BITS, slot % CHUNK_BITS);
        self.allocated_slots[chunk] & (1 << bit) != 0
    }

    fn set_slot_bit(&mut self, slot: usize) {
        let (chunk, bit) = (slot / CHUNK_BITS, slot % CHUNK_BITS);
        self.allocated_slots[chunk] |= 1 << bit;
    }

    fn clear_slot_bit(&mut self, slot: usize) {
        let (chunk, bit) = (slot / CHUNK_BITS, slot % CHUNK_BITS);
        self.allocated_slots[chunk] &= !(1 << bit);
    }

    fn find_free_slot(&self) -> Option<usize> {
        (self.next_search_slot..NUM_STACK_SLOTS)
            .chain(0..self.next_search_slot)
            .find(|i| !self.slot_bit(*i))
    }

    fn allocate_slot(&mut self) -> Option<usize> {
        let slot = self.find_free_slot()?;
        self.set_slot_bit(slot);
        self.next_search_slot = (slot + 1) % NUM_STACK_SLOTS;
        Some(slot)
    }

    fn free_slot(&mut self, slot: usize) {
        assert!(slot < NUM_STACK_SLOTS);
        assert!(self.slot_bit(slot));
        self.clear_slot_bit(slot);
    }
}

#[derive(Debug)]
pub(super) struct StackSlot {
    slot: usize,
}

impl StackSlot {
    pub(super) fn allocate() -> Option<Self> {
        let mut allocator = STACK_SLOT_ALLOCATOR.get().unwrap().lock();
        let slot = allocator.allocate_slot()?;
        Some(Self { slot })
    }

    pub fn range(&self) -> Range<usize> {
        let end = self.top();
        end - STACK_SIZE..end
    }

    pub fn top(&self) -> usize {
        assert!(self.slot < NUM_STACK_SLOTS);
        STACK_ARENA_END - (STACK_SIZE + STACK_PADDING_SIZE) * self.slot
    }
}

impl Drop for StackSlot {
    fn drop(&mut self) {
        let mut allocator = STACK_SLOT_ALLOCATOR.get().unwrap().lock();
        allocator.free_slot(self.slot);
    }
}
