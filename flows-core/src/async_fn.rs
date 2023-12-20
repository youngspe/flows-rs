use std::future::Future;

pub trait AsyncFnOnce0 {
    type Fut: Future<Output = Self::Output>;
    type Output;
    fn call_once(self) -> Self::Fut;
}

pub trait AsyncFnOnce1<Arg0> {
    type Fut: Future<Output = Self::Output>;
    type Output;
    fn call_once(self, arg0: Arg0) -> Self::Fut;
}

pub trait AsyncFnOnce2<Arg0, Arg1> {
    type Fut: Future<Output = Self::Output>;
    type Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1) -> Self::Fut;
}

pub trait AsyncFnOnce3<Arg0, Arg1, Arg2> {
    type Fut: Future<Output = Self::Output>;
    type Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1, arg2: Arg2) -> Self::Fut;
}

impl<F, Fut> AsyncFnOnce0 for F
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self) -> Self::Fut {
        self()
    }
}

impl<F, Fut, A> AsyncFnOnce0 for (F, A)
where
    F: FnOnce(A) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self) -> Self::Fut {
        self.0(self.1)
    }
}

impl<F, Fut, Arg0> AsyncFnOnce1<Arg0> for F
where
    F: FnOnce(Arg0) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0) -> Self::Fut {
        self(arg0)
    }
}

impl<F, Fut, Arg0, A> AsyncFnOnce1<Arg0> for (F, A)
where
    F: FnOnce(Arg0, A) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0) -> Self::Fut {
        self.0(arg0, self.1)
    }
}

impl<F, Fut, Arg0, Arg1> AsyncFnOnce2<Arg0, Arg1> for F
where
    F: FnOnce(Arg0, Arg1) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1) -> Self::Fut {
        self(arg0, arg1)
    }
}

impl<F, Fut, Arg0, Arg1, A> AsyncFnOnce2<Arg0, Arg1> for (F, A)
where
    F: FnOnce(Arg0, Arg1, A) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1) -> Self::Fut {
        self.0(arg0, arg1, self.1)
    }
}

impl<F, Fut, Arg0, Arg1, Arg2> AsyncFnOnce3<Arg0, Arg1, Arg2> for F
where
    F: FnOnce(Arg0, Arg1, Arg2) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1, arg2: Arg2) -> Self::Fut {
        self(arg0, arg1, arg2)
    }
}

impl<F, Fut, Arg0, Arg1, Arg2, A> AsyncFnOnce3<Arg0, Arg1, Arg2> for (F, A)
where
    F: FnOnce(Arg0, Arg1, Arg2, A) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
    fn call_once(self, arg0: Arg0, arg1: Arg1, arg2: Arg2) -> Self::Fut {
        self.0(arg0, arg1, arg2, self.1)
    }
}
