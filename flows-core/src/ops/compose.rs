use std::{
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{Context, Poll},
};

use crate::Flow;

use super::{FlowOp, WrapOp};

pin_project_lite::pin_project!(
    pub struct Compose<B, Src, Dst> {
        #[pin]
        src: Src,
        #[pin]
        dst: Dst,
        b: Option<B>,
    }
);

impl<A, B, C, Ret, Src, Dst> Flow<A> for Compose<B, Src, Dst>
where
    Src: Flow<A, Yield = B, Return = Ret>,
    Dst: Flow<B, Yield = C, Return = Ret>,
{
    type Yield = C;
    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<A>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let this = self.project();

        match this.src.poll_resume(cx, input) {
            Poll::Ready(Continue(yielded)) => {
                *this.b = Some(yielded);
            }
            Poll::Ready(Break(ret)) => return Poll::Ready(Break(ret)),
            Poll::Pending => {}
        }

        this.dst.poll_resume(cx, this.b).map(|x| match x {
            Continue(x) => Continue(x),
            Break(ret) => Break(ret),
        })
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project().src.can_resume()
    }
}

pub struct ComposeOp<Fl2> {
    rhs: Fl2,
}

impl<A, B, Fl1, Fl2> FlowOp<Fl1, A> for ComposeOp<Fl2>
where
    Fl1: Flow<A, Yield = B>,
    Fl2: Flow<B>,
{
    type Output = Compose<B, Fl1, Fl2>;

    fn execute(self, flow: Fl1) -> Self::Output {
        Compose {
            src: flow,
            dst: self.rhs,
            b: None,
        }
    }
}

pub fn compose_with<Fl2>(rhs: Fl2) -> WrapOp<ComposeOp<Fl2>> {
    WrapOp(ComposeOp { rhs })
}
