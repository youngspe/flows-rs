pub trait MapFnOnce<In> {
    type Out;

    fn map_exec_once(self, value: In) -> Self::Out;
}

pub trait MapFn<In>: MapFnOnce<In> {
    fn map_exec(&mut self, value: In) -> Self::Out;
}

impl<In, Out, F> MapFnOnce<In> for F
where
    F: FnOnce(In) -> Out,
{
    type Out = Out;

    fn map_exec_once(self, value: In) -> Self::Out {
        self(value)
    }
}

impl<In, Out, F> MapFn<In> for F
where
    F: FnMut(In) -> Out,
{
    fn map_exec(&mut self, value: In) -> Self::Out {
        self(value)
    }
}

pub trait NewFnOnce {
    type Out;

    fn new_exec_once(self) -> Self::Out;
}

pub trait NewFn: NewFnOnce {
    fn new_exec(&mut self) -> Self::Out;
}

impl<Out, F> NewFnOnce for F
where
    F: FnOnce() -> Out,
{
    type Out = Out;

    fn new_exec_once(self) -> Self::Out {
        self()
    }
}

impl<Out, F> NewFn for F
where
    F: FnMut() -> Out,
{
    fn new_exec(&mut self) -> Self::Out {
        self()
    }
}

pub struct CloneFn<T>(pub T);

impl<T: Clone> NewFnOnce for CloneFn<T> {
    type Out = T;

    fn new_exec_once(mut self) -> Self::Out {
        self.new_exec()
    }
}

impl<T: Clone> NewFn for CloneFn<T> {
    fn new_exec(&mut self) -> Self::Out {
        self.0.clone()
    }
}
