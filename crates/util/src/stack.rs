const RED_ZONE: usize = 100 * 1024;
const STACK_SIZE: usize = 1024 * 1024;

#[inline]
pub fn with_sufficient_stack<R, F>(f: F) -> R
where
    F: FnOnce() -> R,
{
    stacker::maybe_grow(RED_ZONE, STACK_SIZE, f)
}
