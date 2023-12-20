use std::{
    future::Future,
    mem,
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{self, Context, Poll},
};

use either::Either;
use futures_util::{self, Sink, Stream};

use super::Flow;
use crate::my_try::MyTry;

pub trait IntoFlow<Resume, M = ()> {
    type Yield;
    type Return;
    type IntoFlow: Flow<Resume, Yield = Self::Yield, Return = Self::Return>;

    fn into_flow(self) -> Self::IntoFlow;
}

impl<Fl, R> IntoFlow<R> for Fl
where
    Fl: Flow<R>,
{
    type Yield = Fl::Yield;
    type Return = Fl::Return;

    type IntoFlow = Fl;

    fn into_flow(self) -> Self::IntoFlow {
        self
    }
}

pin_project_lite::pin_project!(
    pub struct StreamFlow<St: ?Sized> {
        #[pin]
        inner: St,
    }
);

impl<St> Flow<()> for StreamFlow<St>
where
    St: ?Sized + Stream,
{
    type Yield = St::Item;
    type Return = ();

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<()>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        if input.is_some() {
            self.project().inner.poll_next(cx).map(|x| {
                *input = None;
                x.map(Continue).unwrap_or(Break(()))
            })
        } else {
            Poll::Pending
        }
    }
}

impl<St> IntoFlow<(), StreamFlow<()>> for St
where
    St: futures_util::stream::TryStream,
{
    type Yield = St::Item;
    type Return = ();
    type IntoFlow = StreamFlow<St>;

    fn into_flow(self) -> Self::IntoFlow {
        StreamFlow { inner: self }
    }
}

pub struct IterFlow<It: ?Sized> {
    pub(crate) iter: It,
}

impl<It: ?Sized> Unpin for IterFlow<It> {}

impl<It> Flow for IterFlow<It>
where
    It: ?Sized + Iterator,
{
    type Yield = It::Item;
    type Return = ();

    fn poll_resume(
        self: Pin<&mut Self>,
        _: &mut Context,
        input: &mut Option<()>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        if input.take().is_some() {
            Poll::Ready(
                self.get_mut()
                    .iter
                    .next()
                    .map(Continue)
                    .unwrap_or(Break(())),
            )
        } else {
            Poll::Pending
        }
    }
}

impl<It> IntoFlow<(), IterFlow<()>> for It
where
    It: IntoIterator,
{
    type Yield = It::Item;
    type Return = ();
    type IntoFlow = IterFlow<It::IntoIter>;

    fn into_flow(self) -> Self::IntoFlow {
        IterFlow {
            iter: self.into_iter(),
        }
    }
}

pub(crate) enum SinkFlowState {
    Ready,
    Pending,
    Closing,
}

pin_project_lite::pin_project!(
    pub struct SinkFlow<S> {
        item_sent: bool,
        state: SinkFlowState,
        #[pin]
        dest: S,
    }
);

impl<Res, S> Flow<ControlFlow<(), Res>> for SinkFlow<S>
where
    S: Sink<Res>,
{
    type Yield = ();
    type Return = S::Error;

    fn poll_resume(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<ControlFlow<(), Res>>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        (move || loop {
            let mut this = self.as_mut().project();
            match this.state {
                SinkFlowState::Ready => match input.take() {
                    Some(Continue(input)) => {
                        this.dest.as_mut().start_send(input)?;
                        *this.item_sent = true;
                        *this.state = SinkFlowState::Pending;
                    }
                    Some(Break(())) => {
                        *this.state = SinkFlowState::Closing;
                    }
                    None => {
                        return match mem::replace(this.item_sent, false) {
                            true => Poll::Ready(Ok(())),
                            false => Poll::Pending,
                        }
                    }
                },
                SinkFlowState::Pending => {
                    () = task::ready!(this.dest.poll_ready(cx))?;
                    *this.state = SinkFlowState::Ready;
                }
                SinkFlowState::Closing => return this.dest.poll_close(cx),
            }
        })()
        .map(MyTry::into_control_flow)
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        matches!(self.state, SinkFlowState::Ready)
    }
}

pub trait FromFlow<Item, Resume = (), M = ()> {
    type FromFlowFuture<Fl: Flow<Resume, Yield = Item>>: Future<Output = Self>;
    fn from_flow<Fl: Flow<Resume, Yield = Item>>(src: Fl) -> Self::FromFlowFuture<Fl>;
}

pin_project_lite::pin_project!(
    pub struct ExtendFromFlow<Dest = (), Src = (), Res = ()> {
        #[pin]
        src: Src,
        dest: Option<Dest>,
        input: Option<Res>,
    }
);

impl<T, Src, Dest, Res> Future for ExtendFromFlow<Dest, Src, Res>
where
    Src: Flow<Res, Yield = T>,
    Dest: Extend<T>,
    Res: Default,
{
    type Output = Dest;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match task::ready!(this.src.poll_resume(cx, this.input)) {
                Continue(item) => {
                    this.dest.as_mut().unwrap().extend([item]);
                    this.input.get_or_insert_with(Res::default);
                }
                Break(_) => return Poll::Ready(this.dest.take().unwrap()),
            }
        }
    }
}

impl<T, Res, Dest> FromFlow<T, Res, ExtendFromFlow> for Dest
where
    Dest: Default + Extend<T>,
    Res: Default,
{
    type FromFlowFuture<Fl: Flow<Res, Yield = T>> = ExtendFromFlow<Dest, Fl, Res>;

    fn from_flow<Fl: Flow<Res, Yield = T>>(src: Fl) -> Self::FromFlowFuture<Fl> {
        ExtendFromFlow {
            src,
            dest: Some(Dest::default()),
            input: Some(Res::default()),
        }
    }
}

pin_project_lite::pin_project!(
    pub struct EitherFlow<L, R> {
        #[pin]
        inner: Either<L, R>,
    }
);

impl<Y, Res, Ret, L, R, ML, MR> IntoFlow<Res, Either<ML, MR>> for Either<L, R>
where
    L: IntoFlow<Res, ML, Yield = Y, Return = Ret>,
    R: IntoFlow<Res, MR, Yield = Y, Return = Ret>,
{
    type Yield = Y;
    type Return = Ret;

    type IntoFlow = EitherFlow<L::IntoFlow, R::IntoFlow>;

    fn into_flow(self) -> Self::IntoFlow {
        EitherFlow {
            inner: self.map_either(L::into_flow, R::into_flow),
        }
    }
}

impl<Y, Res, Ret, L, R> Flow<Res> for EitherFlow<L, R>
where
    L: Flow<Res, Yield = Y, Return = Ret>,
    R: Flow<Res, Yield = Y, Return = Ret>,
{
    type Yield = Y;

    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        match self.project().inner.as_pin_mut() {
            Either::Left(x) => x.poll_resume(cx, input),
            Either::Right(x) => x.poll_resume(cx, input),
        }
    }
}
