use std::fmt::Debug;

pub trait DestructureTupleExt<A, B> {
    fn destructure(self) -> (Option<A>, Option<B>);
}

impl<A, B> DestructureTupleExt<A, B> for Option<(A, B)> {
    fn destructure(self) -> (Option<A>, Option<B>) {
        self.map(|(a, b)| (Some(a), Some(b)))
            .unwrap_or((None, None))
    }
}

impl<A, B, E> DestructureTupleExt<A, B> for Result<Option<(A, B)>, E>
where
    E: Debug,
{
    fn destructure(self) -> (Option<A>, Option<B>) {
        match self {
            Ok(Some((a, b))) => (Some(a), Some(b)),
            Ok(None) => (None, None),
            Err(_) => (None, None),
        }
    }
}
