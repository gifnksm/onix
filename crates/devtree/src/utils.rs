pub(crate) fn slice_split_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().position(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}

pub(crate) fn slice_rsplit_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().rposition(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}
