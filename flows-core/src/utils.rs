use std::{
    future::Future,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

use crate::custom_fn::MapFnOnce;

pub(crate) fn pin_as_deref_mut<'ptr, P: DerefMut>(
    pin: Pin<&'ptr mut Pin<P>>,
) -> Pin<&'ptr mut P::Target> {
    unsafe { pin.get_unchecked_mut() }.as_mut()
}

pin_project_lite::pin_project!(
    pub struct MapFuture<Fut, Fun> {
        #[pin]
        fut: Fut,
        fun: Option<Fun>,
    }
);

impl<Fut, Fun> Future for MapFuture<Fut, Fun>
where
    Fut: Future,
    Fun: MapFnOnce<Fut::Output>,
{
    type Output = Fun::Out;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.fut
            .poll(cx)
            .map(|x| this.fun.take().unwrap().map_exec_once(x))
    }
}

pub fn map_future<Fut, Fun>(fut: Fut, fun: Fun) -> MapFuture<Fut, Fun>
where
    Fut: Future,
    Fun: MapFnOnce<Fut::Output>,
{
    MapFuture {
        fut,
        fun: Some(fun),
    }
}
