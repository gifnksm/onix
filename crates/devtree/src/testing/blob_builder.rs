extern crate alloc;

use alloc::vec::Vec;

use dataview::DataView;

use crate::{
    blob::{DEVICETREE_ALIGNMENT, Header, LAST_COMPATIBLE_VERSION, MAGIC, ReserveEntry, VERSION},
    util::AlignedByteBuffer,
};

#[derive(Debug, Clone)]
pub struct BlobBuilder {
    magic: u32,
    version: u32,
    last_compatible_version: u32,
    boot_cpuid_phys: u32,
    mem_rsvmap: Vec<ReserveEntry>,
    struct_block: Vec<u8>,
    strings_block: Vec<u8>,
}

impl Default for BlobBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            last_compatible_version: LAST_COMPATIBLE_VERSION,
            boot_cpuid_phys: 0,
            mem_rsvmap: Vec::new(),
            struct_block: Vec::new(),
            strings_block: Vec::new(),
        }
    }

    pub fn magic(&mut self, magic: u32) -> &mut Self {
        self.magic = magic;
        self
    }

    pub fn version(&mut self, version: u32) -> &mut Self {
        self.version = version;
        self
    }

    pub fn last_compatible_version(&mut self, last_compatible_version: u32) -> &mut Self {
        self.last_compatible_version = last_compatible_version;
        self
    }

    pub fn boot_cpuid_phys(&mut self, boot_cpuid_phys: u32) -> &mut Self {
        self.boot_cpuid_phys = boot_cpuid_phys;
        self
    }

    pub fn extend_mem_rsvmap<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = ReserveEntry>,
    {
        self.mem_rsvmap.extend(iter);
        self
    }

    pub fn extend_mem_rsvmap_from_slice(&mut self, slice: &[ReserveEntry]) -> &mut Self {
        self.mem_rsvmap.extend_from_slice(slice);
        self
    }

    pub fn extend_struct_block<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.struct_block.extend(iter);
        self
    }

    pub fn extend_struct_block_from_slice(&mut self, slice: &[u8]) -> &mut Self {
        self.struct_block.extend_from_slice(slice);
        self
    }

    pub fn extend_strings_block<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.strings_block.extend(iter);
        self
    }

    pub fn extend_strings_block_from_slice(&mut self, slice: &[u8]) -> &mut Self {
        self.strings_block.extend_from_slice(slice);
        self
    }

    #[must_use]
    pub fn build(&self) -> AlignedByteBuffer<DEVICETREE_ALIGNMENT> {
        let header = Header::new_for_test(
            self.magic,
            self.version,
            self.last_compatible_version,
            self.boot_cpuid_phys,
            &self.mem_rsvmap,
            &self.struct_block,
            &self.strings_block,
        );

        let mut blob = AlignedByteBuffer::<DEVICETREE_ALIGNMENT>::new_zeroed(header.total_size());
        let data = DataView::from_mut(&mut blob[..]);

        data.write(0, &header);
        data.slice_mut(
            header.memory_reservation_block_offset(),
            self.mem_rsvmap.len(),
        )
        .copy_from_slice(&self.mem_rsvmap);
        data.slice_mut(header.struct_block_offset(), header.struct_block_size())
            .copy_from_slice(&self.struct_block);
        data.slice_mut(header.strings_block_offset(), header.strings_block_size())
            .copy_from_slice(&self.strings_block);

        blob
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_default_builder_values() {
        let builder = BlobBuilder::default();
        assert_eq!(builder.magic, MAGIC);
        assert_eq!(builder.version, VERSION);
        assert_eq!(builder.last_compatible_version, LAST_COMPATIBLE_VERSION);
        assert_eq!(builder.boot_cpuid_phys, 0);
        assert!(builder.mem_rsvmap.is_empty());
        assert!(builder.struct_block.is_empty());
        assert!(builder.strings_block.is_empty());
    }

    #[test]
    fn test_setters_and_extenders() {
        let mut builder = BlobBuilder::new();
        builder
            .magic(0x1234_5678)
            .version(2)
            .last_compatible_version(1)
            .boot_cpuid_phys(42);

        assert_eq!(builder.magic, 0x1234_5678);
        assert_eq!(builder.version, 2);
        assert_eq!(builder.last_compatible_version, 1);
        assert_eq!(builder.boot_cpuid_phys, 42);

        let rsvmap = [ReserveEntry::new(1, 2), ReserveEntry::new(3, 4)];
        builder.extend_mem_rsvmap(rsvmap);
        assert_eq!(builder.mem_rsvmap, rsvmap);

        let more_rsvmap = [ReserveEntry::new(5, 6), ReserveEntry::new(7, 8)];
        builder.extend_mem_rsvmap_from_slice(&more_rsvmap);
        assert_eq!(builder.mem_rsvmap.len(), 4);

        builder.extend_struct_block([10, 11, 12]);
        assert_eq!(builder.struct_block, [10, 11, 12]);
        builder.extend_struct_block_from_slice(&[13, 14]);
        assert_eq!(builder.struct_block, [10, 11, 12, 13, 14]);

        builder.extend_strings_block([20, 21]);
        assert_eq!(builder.strings_block, [20, 21]);
        builder.extend_strings_block_from_slice(&[22]);
        assert_eq!(builder.strings_block, [20, 21, 22]);
    }

    #[test]
    fn test_build_blob_size_and_alignment() {
        let mut builder = BlobBuilder::new();
        builder
            .extend_mem_rsvmap([ReserveEntry::new(1, 2)])
            .extend_struct_block([1, 2, 3, 4])
            .extend_strings_block([5, 6, 7, 8]);

        let blob = builder.build();
        assert!(
            blob.as_slice()
                .as_ptr()
                .addr()
                .is_multiple_of(DEVICETREE_ALIGNMENT)
        );
        assert!(blob.len() >= 16); // Should be at least header size
    }

    #[test]
    fn test_empty_blob_build() {
        let builder = BlobBuilder::new();
        let blob = builder.build();
        assert!(
            blob.as_slice()
                .as_ptr()
                .addr()
                .is_multiple_of(DEVICETREE_ALIGNMENT)
        );
        assert_eq!(&blob[0..4], MAGIC.to_be_bytes());
    }
}
