use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{self, Context, Poll},
};

use crate::{
    custom_fn::{MapFn, MapFnOnce},
    my_try::MyTry,
    utils::{map_future, MapFuture},
    Flow, convert::IntoFlow,
};

use super::{map_sync, FlowOp, MapSync, WrapOp, MapReturnSync, Compose, compose_with, map_return_sync};

pin_project_lite::pin_project!(
    pub struct TryFeedback<Res, Fl> {
        #[pin]
        inner: Fl,
        input: Option<Res>,
    }
);

impl<Res, Ret, Y, Fl> Future for TryFeedback<Res, Fl>
where
    Fl: Flow<Res, Yield = Y, Return = Ret>,
    Y: MyTry<Continue = Res>,
{
    type Output = Y::Mapped<Fl::Return>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(loop {
            let this = self.as_mut().project();

            let out = task::ready!(this.inner.poll_resume(cx, this.input));

            match MyTry::map(out, MyTry::into_control_flow) {
                Continue(Continue(x)) => *this.input = Some(x),
                Continue(Break(brk)) => break MyTry::from_break(brk),
                Break(ret) => break MyTry::from_continue(ret),
            }
        })
    }
}

pub struct TryFeedbackOp<Res> {
    init: Res,
}

impl<Res, Fl> FlowOp<Fl, Res> for TryFeedbackOp<Res>
where
    Fl: Flow<Res>,
    Fl::Yield: MyTry<Continue = Res>,
{
    type Output = TryFeedback<Res, Fl>;

    fn execute(self, flow: Fl) -> Self::Output {
        TryFeedback {
            inner: flow,
            input: Some(self.init),
        }
    }
}

pub fn try_feedback<Res>(init: Res) -> WrapOp<TryFeedbackOp<Res>> {
    WrapOp(TryFeedbackOp { init })
}

pub struct NeverToAny<T>(PhantomData<T>);

impl<T> MapFnOnce<Infallible> for NeverToAny<T> {
    type Out = T;

    fn map_exec_once(mut self, value: Infallible) -> Self::Out {
        self.map_exec(value)
    }
}

impl<T> MapFn<Infallible> for NeverToAny<T> {
    fn map_exec(&mut self, value: Infallible) -> Self::Out {
        match value {}
    }
}

pub struct WrapOk<E>(PhantomData<E>);

impl<T, E> MapFnOnce<T> for WrapOk<E> {
    type Out = Result<T, E>;

    fn map_exec_once(mut self, value: T) -> Self::Out {
        self.map_exec(value)
    }
}

impl<T, E> MapFn<T> for WrapOk<E> {
    fn map_exec(&mut self, value: T) -> Self::Out {
        Ok(value)
    }
}

pub struct UnwrapInfallible;

impl<T> MapFnOnce<Result<T, Infallible>> for UnwrapInfallible {
    type Out = T;

    fn map_exec_once(mut self, value: Result<T, Infallible>) -> Self::Out {
        self.map_exec(value)
    }
}

impl<T> MapFn<Result<T, Infallible>> for UnwrapInfallible {
    fn map_exec(&mut self, value: Result<T, Infallible>) -> Self::Out {
        value.unwrap_or_else(|e| match e {})
    }
}

pub type Feedback<Res, Fl> =
    MapFuture<TryFeedback<Res, MapSync<Fl, WrapOk<Infallible>>>, UnwrapInfallible>;

pub struct FeedbackOp<Res> {
    init: Res,
}

impl<Res, Fl> FlowOp<Fl, Res> for FeedbackOp<Res>
where
    Fl: Flow<Res, Yield = Res>,
{
    type Output = Feedback<Res, Fl>;

    fn execute(self, flow: Fl) -> Self::Output {
        map_future(
            flow.then(map_sync(WrapOk(PhantomData)))
                .then(try_feedback(self.init)),
            UnwrapInfallible,
        )
    }
}

pub fn feedback<Res>(init: Res) -> WrapOp<FeedbackOp<Res>> {
    WrapOp(FeedbackOp { init })
}

pub type TryForEach<Yield, Resume, Src, Dst> = TryFeedback<
    Resume,
    Compose<Yield, Src, MapReturnSync<Dst, NeverToAny<<Src as Flow<Resume>>::Return>>>,
>;

pub type ForEach<Yield, Resume, Src, Dst> = Feedback<
    Resume,
    Compose<Yield, Src, MapReturnSync<Dst, NeverToAny<<Src as Flow<Resume>>::Return>>>,
>;

pub fn try_for_each_init<Yield, Res, Out, Return, Src, Dst, M>(
    src: impl IntoFlow<Res, M, IntoFlow = Src>,
    collector: Dst,
    init: Res,
) -> TryForEach<Yield, Res, Src, Dst>
where
    Src: Flow<Res, Yield = Yield, Return = Return>,
    Dst: Flow<Yield, Yield = Out, Return = Infallible>,
    Out: MyTry<Continue = Res>,
{
    src.into_flow()
        .then(compose_with(
            collector.then(map_return_sync(NeverToAny(PhantomData))),
        ))
        .then(try_feedback(init))
}

pub fn try_for_each<Yield, Res, Out, Return, Src, Dst, M>(
    src: impl IntoFlow<Res, M, IntoFlow = Src>,
    collector: Dst,
) -> TryForEach<Yield, Res, Src, Dst>
where
    Res: Default,
    Src: Flow<Res, Yield = Yield, Return = Return>,
    Dst: Flow<Yield, Yield = Out, Return = Infallible>,
    Out: MyTry<Continue = Res>,
{
    try_for_each_init(src, collector, Default::default())
}

pub fn for_each_init<Yield, Res, Return, Src, Dst, M>(
    src: impl IntoFlow<Res, M, IntoFlow = Src>,
    collector: Dst,
    init: Res,
) -> ForEach<Yield, Res, Src, Dst>
where
    Src: Flow<Res, Yield = Yield, Return = Return>,
    Dst: Flow<Yield, Yield = Res, Return = Infallible>,
{
    src.into_flow()
        .then(compose_with(
            collector.then(map_return_sync(NeverToAny(PhantomData))),
        ))
        .then(feedback(init))
}

pub fn for_each<Yield, Res, Return, Src, Dst, M>(
    src: impl IntoFlow<Res, M, IntoFlow = Src>,
    collector: Dst,
) -> ForEach<Yield, Res, Src, Dst>
where
    Res: Default,
    Src: Flow<Res, Yield = Yield, Return = Return>,
    Dst: Flow<Yield, Yield = Res, Return = Infallible>,
{
    for_each_init(src, collector, Default::default())
}

pin_project_lite::pin_project!(
    struct MergeAll<Fl1, Fl2, Ret, M> {
        #[pin]
        main_flow: Fl1,
        sub_flows: Vec<Pin<Box<Fl2>>>,
        ret: Option<Ret>,
        _m: PhantomData<M>,
    }
);

impl<Fl1, Fl2, Res, Ret, M> Flow<Res> for MergeAll<Fl1, Fl2, Ret, M>
where
    Fl1: Flow<Res, Return = Ret>,
    Fl1::Yield: IntoFlow<Res, M, IntoFlow = Fl2>,
    Fl2: Flow<Res>,
{
    type Yield = Fl2::Yield;
    type Return = Ret;

    fn poll_resume(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let mut no_input = None;
        let mut this = self.as_mut().project();

        if this.ret.is_none() {
            let can_resume = input.is_some() && this.main_flow.as_mut().can_resume();

            match task::ready!(this
                .main_flow
                .poll_resume(cx, if can_resume { input } else { &mut no_input }))
            {
                Continue(sub_flow) => {
                    this.sub_flows.push(Box::pin(sub_flow.into_flow()));
                }
                Break(returned) => return Poll::Ready(Break(returned)),
            }
        }

        {
            let mut i = 0;
            while i < this.sub_flows.len() {
                let sub_flow = &mut this.sub_flows[i];
                let can_resume = input.is_some() && sub_flow.as_mut().can_resume();

                match sub_flow
                    .as_mut()
                    .poll_resume(cx, if can_resume { input } else { &mut no_input })
                {
                    Poll::Ready(Continue(item)) => return Poll::Ready(Continue(item)),
                    Poll::Ready(Break(_)) => {
                        this.sub_flows.swap_remove(i);
                    }
                    Poll::Pending => {
                        i += 1;
                    }
                }
            }
        }

        this.sub_flows
            .is_empty()
            .then(|| this.ret.take())
            .flatten()
            .map(Break)
            .map(Poll::Ready)
            .unwrap_or(Poll::Pending)
    }
}

pub fn merge_all<'f, Y, Res, Ret, M, Fl1, Fl2>(
) -> WrapOp<impl FlowOp<Fl1, Res, Output = impl Flow<Res, Yield = Y, Return = Ret> + 'f> + 'f>
where
    Fl1: 'f + Flow<Res, Return = Ret>,
    Fl1::Yield: IntoFlow<Res, M, Yield = Y, IntoFlow = Fl2>,
    Fl2: 'f + Flow<Res, Yield = Y>,
    Ret: 'f,
    M: 'f,
{
    WrapOp(|main_flow: Fl1| MergeAll::<Fl1, Fl2, Ret, M> {
        main_flow,
        sub_flows: Vec::new(),
        ret: None,
        _m: PhantomData,
    })
}
