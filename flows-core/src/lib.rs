pub mod async_fn;
pub mod self_ref;
extern crate either;
extern crate futures_util;
extern crate pin_project_lite;

pub mod convert;
pub mod custom_fn;
pub mod my_try;
pub mod ops;
mod utils;

use std::{
    cell::UnsafeCell,
    future::{Future, IntoFuture},
    mem,
    ops::{
        ControlFlow::{self, Break, Continue},
        DerefMut,
    },
    pin::Pin,
    ptr::NonNull,
    task::{self, Context, Poll},
};

use async_fn::AsyncFnOnce2;
use convert::IntoFlow;
use ops::FlowOp;
use self_ref::{SelfRef, WithLifetime};

pub trait Flow<Resume = ()> {
    type Yield;
    type Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Resume>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>>;

    fn can_resume(self: Pin<&mut Self>) -> bool {
        true
    }

    #[must_use]
    fn resume<'flow>(&'flow mut self, value: Resume) -> FlowResume<'flow, Self, Resume>
    where
        Self: Sized + Unpin,
    {
        FlowResume {
            flow: Pin::new(self),
            value,
        }
    }

    #[must_use]
    fn next<'flow>(&'flow mut self) -> FlowResume<'flow, Self, Resume>
    where
        Self: Sized + Unpin,
        Resume: Default,
    {
        self.resume(Default::default())
    }

    fn then<Op: FlowOp<Self, Resume>>(self, op: Op) -> Op::Output
    where
        Self: Sized,
    {
        op.execute(self)
    }
}

impl<F, Resume, P> Flow<Resume> for Pin<P>
where
    F: ?Sized + Flow<Resume>,
    P: DerefMut<Target = F>,
{
    type Yield = F::Yield;
    type Return = F::Return;
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Resume>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        F::poll_resume(utils::pin_as_deref_mut(self), cx, input)
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        F::can_resume(utils::pin_as_deref_mut(self))
    }
}

impl<F, Resume> Flow<Resume> for &mut F
where
    F: ?Sized + Flow<Resume> + Unpin,
{
    type Yield = F::Yield;
    type Return = F::Return;

    fn poll_resume(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Resume>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        F::poll_resume(Pin::new(*self), cx, input)
    }

    fn can_resume(mut self: Pin<&mut Self>) -> bool {
        F::can_resume(Pin::new(*self))
    }
}

pub struct FlowResume<'flow, F, Resume> {
    flow: Pin<&'flow mut F>,
    value: Resume,
}

impl<'flow, F: Flow<Resume>, Resume> IntoFuture for FlowResume<'flow, F, Resume> {
    type Output = ControlFlow<F::Return, F::Yield>;
    type IntoFuture = FlowResumeFuture<'flow, F, Resume>;

    fn into_future(self) -> Self::IntoFuture {
        let Self { flow, value } = self;
        FlowResumeFuture {
            flow,
            value: value.into(),
        }
    }
}

pub struct FlowResumeFuture<'flow, F, Resume> {
    flow: Pin<&'flow mut F>,
    value: Option<Resume>,
}

impl<F, Resume> Unpin for FlowResumeFuture<'_, F, Resume> {}

impl<'flow, F: Flow<Resume>, Resume> Future for FlowResumeFuture<'flow, F, Resume> {
    type Output = ControlFlow<F::Return, F::Yield>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.flow.as_mut().poll_resume(cx, &mut this.value)
    }
}

#[derive(Debug, Default)]
enum FlowState<Yield, Resume = ()> {
    #[default]
    Empty,
    Resume(Resume),
    Yield(Yield),
}

impl<Y, R> Unpin for FlowState<Y, R> {}

#[derive(Debug)]
struct StateRef<'state, Yield, Resume = ()> {
    state: &'state UnsafeCell<FlowState<Yield, Resume>>,
}

#[derive(Debug)]
struct RawStateRef<Yield, Resume = ()> {
    ptr: NonNull<UnsafeCell<FlowState<Yield, Resume>>>,
}

impl<'state, Yield, Resume> StateRef<'state, Yield, Resume> {
    pub unsafe fn new(state: &'state UnsafeCell<FlowState<Yield, Resume>>) -> Self {
        Self { state }
    }

    pub unsafe fn clone_unchecked(&self) -> Self {
        Self { state: self.state }
    }

    pub unsafe fn with_unchecked<R>(
        &self,
        f: impl FnOnce(&mut FlowState<Yield, Resume>) -> R,
    ) -> R {
        unsafe { f(&mut *self.state.get()) }
    }

    #[must_use]
    pub fn replace(&self, new: FlowState<Yield, Resume>) -> FlowState<Yield, Resume> {
        unsafe { self.with_unchecked(|state| mem::replace(state, new)) }
    }

    pub fn set(&self, new: FlowState<Yield, Resume>) {
        let _ = self.replace(new);
    }

    pub fn kind(&self) -> FlowState<()> {
        unsafe {
            self.with_unchecked(|state| match state {
                FlowState::Empty => FlowState::Empty,
                FlowState::Resume(_) => FlowState::Resume(()),
                FlowState::Yield(_) => FlowState::Yield(()),
            })
        }
    }

    pub fn is_yield(&self) -> bool {
        matches!(self.kind(), FlowState::Yield(()))
    }

    pub fn is_resume(&self) -> bool {
        matches!(self.kind(), FlowState::Resume(()))
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.kind(), FlowState::Empty)
    }

    pub fn take(&self) -> FlowState<Yield, Resume> {
        self.replace(FlowState::Empty)
    }

    pub fn take_resumed(&self) -> Option<Resume> {
        if !self.is_resume() {
            return None;
        }
        let FlowState::Resume(resumed) = self.take() else {
            unreachable!()
        };
        Some(resumed)
    }

    pub fn take_yielded(&self) -> Option<Yield> {
        if !self.is_yield() {
            return None;
        }
        let FlowState::Yield(yielded) = self.take() else {
            unreachable!()
        };
        Some(yielded)
    }

    pub fn reborrow_mut<'this>(&'this mut self) -> StateRef<'this, Yield, Resume> {
        StateRef { state: self.state }
    }

    pub fn into_raw(self) -> RawStateRef<Yield, Resume> {
        RawStateRef {
            ptr: self.state.into(),
        }
    }
}

impl<Yield, Resume> RawStateRef<Yield, Resume> {
    pub unsafe fn into_state_ref<'state>(self) -> StateRef<'state, Yield, Resume> {
        StateRef {
            state: self.ptr.as_ref(),
        }
    }
}

impl<Yield, Resume> Clone for RawStateRef<Yield, Resume> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}

unsafe impl<Y: Send, R: Send> Send for StateRef<'_, Y, R> {}
unsafe impl<Y: Send, R: Send> Send for RawStateRef<Y, R> {}

pub struct Sender<'state, Yield, Resume = ()> {
    state_ref: StateRef<'state, Yield, Resume>,
}

pub struct RawSender<Yield, Resume> {
    raw_state_ref: RawStateRef<Yield, Resume>,
}

impl<'state, Yield, Resume> Sender<'state, Yield, Resume> {
    pub fn next(&mut self, value: Yield) -> Next<Yield, Resume> {
        Next {
            state_ref: self.state_ref.reborrow_mut(),
            value: Some(value),
        }
    }

    pub fn next_await<Fut>(&mut self, fut: Fut) -> NextAwait<Yield, Resume, Fut>
    where
        Fut: Future<Output = Yield>,
    {
        NextAwait {
            state_ref: self.state_ref.reborrow_mut(),
            fut: Some(fut),
        }
    }

    pub fn next_from<M, F: Flow<Resume, Yield = Yield>>(
        &mut self,
        src: impl IntoFlow<Resume, M, IntoFlow = F>,
        init: Resume,
    ) -> NextFrom<Yield, Resume, F> {
        NextFrom {
            state_ref: self.state_ref.reborrow_mut(),
            input: Some(init),
            src: src.into_flow(),
        }
    }

    pub fn into_raw(self) -> RawSender<Yield, Resume> {
        RawSender {
            raw_state_ref: self.state_ref.into_raw(),
        }
    }
}

impl<Yield, Resume> RawSender<Yield, Resume> {
    pub unsafe fn into_sender<'state>(self) -> Sender<'state, Yield, Resume> {
        Sender {
            state_ref: self.raw_state_ref.into_state_ref(),
        }
    }

    /// Like into_sender, but a little safer as it's bounded by the lifetime of `self`.
    pub unsafe fn as_sender<'this>(&'this mut self) -> Sender<'this, Yield, Resume> {
        self.clone().into_sender()
    }
}

impl<Yield, Resume> Clone for RawSender<Yield, Resume> {
    fn clone(&self) -> Self {
        Self {
            raw_state_ref: self.raw_state_ref.clone(),
        }
    }
}

pub struct Next<'state, Yield, Resume> {
    state_ref: StateRef<'state, Yield, Resume>,
    value: Option<Yield>,
}

impl<Yield, Resume> Unpin for Next<'_, Yield, Resume> {}

impl<'state, Yield, Resume> Future for Next<'state, Yield, Resume> {
    type Output = Resume;

    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        if this.state_ref.is_yield() {
            return Poll::Pending;
        }

        let new_state = this
            .value
            .take()
            .map(FlowState::Yield)
            .unwrap_or(FlowState::Empty);

        match this.state_ref.replace(new_state) {
            FlowState::Resume(resumed) => Poll::Ready(resumed),
            FlowState::Yield(_) => unreachable!(),
            FlowState::Empty => Poll::Pending,
        }
    }
}

pin_project_lite::pin_project!(
    pub struct NextAwait<'state, Yield, Resume, Fut> {
        #[pin]
        fut: Option<Fut>,
        state_ref: StateRef<'state, Yield, Resume>,
    }
);

impl<'state, Yield, Resume, Fut> Future for NextAwait<'state, Yield, Resume, Fut>
where
    Fut: Future<Output = Yield>,
{
    type Output = Resume;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut this = self.project();

        if this.state_ref.is_yield() {
            return Poll::Pending;
        }

        match this.state_ref.replace(FlowState::Empty) {
            FlowState::Resume(resumed) => Poll::Ready(resumed),
            FlowState::Yield(_) => unreachable!(),
            FlowState::Empty => {
                if let Some(Poll::Ready(value)) =
                    this.fut.as_mut().as_pin_mut().map(|fut| fut.poll(cx))
                {
                    this.fut.set(None);
                    this.state_ref.set(FlowState::Yield(value));
                }

                Poll::Pending
            }
        }
    }
}

pin_project_lite::pin_project!(
    #[project = NextFromProj]
    pub struct NextFrom<'state, Yield, Resume, Src> {
        state_ref: StateRef<'state, Yield, Resume>,
        input: Option<Resume>,
        #[pin]
        src: Src,
    }
);

impl<'state, Yield, Resume, Src, Ret> Future for NextFrom<'state, Yield, Resume, Src>
where
    Src: Flow<Resume, Yield = Yield, Return = Ret>,
{
    type Output = Ret;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut this = self.project();

        let poll = if !this.src.as_mut().can_resume() {
            this.src.poll_resume(cx, &mut None)
        } else if this.input.is_some() {
            this.src.poll_resume(cx, this.input)
        } else {
            let mut input = this.state_ref.take_resumed();
            let poll = this.src.poll_resume(cx, &mut input);
            if let Some(resumed) = input.take() {
                this.state_ref.set(FlowState::Resume(resumed))
            }
            poll
        };

        match task::ready!(poll) {
            Continue(yielded) => {
                this.state_ref.set(FlowState::Yield(yielded));
                Poll::Pending
            }
            Break(ret) => Poll::Ready(ret),
        }
    }
}

impl<'state, Yield, Resume> IntoFuture for &'state mut Sender<'_, Yield, Resume>
where
    Yield: Default,
{
    type Output = Resume;
    type IntoFuture = Next<'state, Yield, Resume>;

    fn into_future(self) -> Self::IntoFuture {
        self.next(Yield::default())
    }
}

pub trait FlowFnLt<'state, Yield, Resume, _Sender = Sender<'state, Yield, Resume>>:
    AsyncFnOnce2<Resume, _Sender, Fut = Self::FlowFut, Output = Self::_FlowReturn>
{
    type FlowFut: Future<Output = Self::_FlowReturn>;
    type _FlowReturn;
}

pub trait FlowFn<Yield, Resume>:
    for<'state> FlowFnLt<'state, Yield, Resume, _FlowReturn = Self::FlowReturn>
{
    type FlowReturn;
}

impl<'state, F, Yield, Resume, Return> FlowFnLt<'state, Yield, Resume> for F
where
    F: ?Sized + AsyncFnOnce2<Resume, Sender<'state, Yield, Resume>, Output = Return>,
{
    type FlowFut = F::Fut;
    type _FlowReturn = Return;
}

impl<F, Yield, Resume, Return> FlowFn<Yield, Resume> for F
where
    F: ?Sized + for<'state> FlowFnLt<'state, Yield, Resume, _FlowReturn = Return>,
{
    type FlowReturn = Return;
}

type FlowFromFnSelfRef<'x, Yield, Resume, Fun> = SelfRef<
    'x,
    UnsafeCell<FlowState<Yield, Resume>>,
    dyn for<'state> WithLifetime<'state, WithLt = FlowFromFnInner<'state, Yield, Resume, Fun>> + 'x,
>;

pin_project_lite::pin_project!(
    struct FlowFromFn<'x, Yield, Resume, Fun: FlowFn<Yield, Resume>> {
        #[pin]
        inner: FlowFromFnSelfRef<'x, Yield, Resume, Fun>,
    }
);

pin_project_lite::pin_project!(
    #[project = FlowFromFnInnerProj]
    enum FlowFromFnInner<'state, Yield, Resume, Fun: FlowFn<Yield, Resume>> {
        Fun {
            fun: Option<Fun>,
        },
        Fut {
            #[pin]
            fut: <Fun as FlowFnLt<'state, Yield, Resume>>::FlowFut,
        },
    }
);

impl<'x, Fun, Yield, Resume> FlowFromFn<'x, Yield, Resume, Fun>
where
    Fun: FlowFn<Yield, Resume>,
{
    fn with_mut<R>(
        self: Pin<&mut Self>,
        f: impl for<'state> FnOnce(
            StateRef<'state, Yield, Resume>,
            Pin<&mut FlowFromFnInner<'state, Yield, Resume, Fun>>,
        ) -> R,
    ) -> R {
        self.project().inner.with_pin_mut(|state, inner| {
            let state_ref = unsafe { StateRef::new(state) };
            f(state_ref, inner)
        })
    }
}

impl<'x, Fun, Yield, Resume, Ret> Flow<Resume> for FlowFromFn<'x, Yield, Resume, Fun>
where
    Fun: FlowFn<Yield, Resume, FlowReturn = Ret>,
{
    type Yield = Yield;
    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Resume>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        self.with_mut(|state_ref, mut inner| {
            if let Some(yielded) = state_ref.take_yielded() {
                return Poll::Ready(Continue(yielded));
            }

            if let FlowFromFnInnerProj::Fun { fun } = inner.as_mut().project() {
                let Some(resumed) = input.take() else {
                    return Poll::Pending;
                };

                let fun = fun.take().unwrap();
                let fut = unsafe {
                    fun.call_once(
                        resumed,
                        Sender {
                            state_ref: state_ref.clone_unchecked(),
                        },
                    )
                };
                inner.set(FlowFromFnInner::Fut { fut })
            } else if let Some(resumed) = input.take() {
                state_ref.set(FlowState::Resume(resumed));
            }

            let FlowFromFnInnerProj::Fut { fut } = inner.project() else {
                unreachable!()
            };

            let poll = fut.poll(cx);
            *input = state_ref.take_resumed();

            match poll {
                Poll::Ready(ret) => Poll::Ready(Break(ret)),
                Poll::Pending => state_ref
                    .take_yielded()
                    .map(Continue)
                    .map(Poll::Ready)
                    .unwrap_or(Poll::Pending),
            }
        })
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.with_mut(|state_ref, _| state_ref.is_empty())
    }
}

pub fn flow_from_fn<'x, Yield, Resume, Return>(
    fun: impl FlowFn<Yield, Resume, FlowReturn = Return> + 'x,
) -> impl Flow<Resume, Yield = Yield, Return = Return> + 'x
where
    Yield: 'x,
    Resume: 'x,
{
    FlowFromFn {
        inner: SelfRef::new_with(FlowState::Empty.into(), |_| FlowFromFnInner::Fun {
            fun: fun.into(),
        }),
    }
}
