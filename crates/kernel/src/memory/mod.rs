use core::ops::Range;

pub mod allocator;
pub mod kernel_space;
pub mod layout;

pub const PAGE_SIZE: usize = sv39::PAGE_SIZE;

pub trait Align: Sized {
    fn align_up(&self, align: usize) -> Self;
    fn align_down(&self, align: usize) -> Self;
    fn is_aligned(&self, align: usize) -> bool;

    fn page_align_up(&self) -> Self {
        self.align_up(PAGE_SIZE)
    }
    fn page_align_down(&self) -> Self {
        self.align_down(PAGE_SIZE)
    }

    fn is_page_aligned(&self) -> bool {
        self.is_aligned(PAGE_SIZE)
    }
}

impl Align for usize {
    fn align_up(&self, align: usize) -> Self {
        self.next_multiple_of(align)
    }

    fn align_down(&self, align: usize) -> Self {
        self / align * align
    }

    fn is_aligned(&self, align: usize) -> bool {
        self.is_multiple_of(align)
    }
}

pub fn expand_to_page_boundaries<T>(range: Range<T>) -> Range<T>
where
    T: Align,
{
    range.start.page_align_down()..range.end.page_align_up()
}
