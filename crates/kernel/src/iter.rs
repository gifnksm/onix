pub trait IteratorExt: Iterator {
    fn assume_one(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut iter = self;
        let first = iter.next()?;
        if iter.next().is_some() {
            return None;
        }
        Some(first)
    }
}

impl<T> IteratorExt for T where T: Iterator {}
