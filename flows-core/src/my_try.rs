use std::ops::ControlFlow::{self, Break, Continue};

pub trait MyTry: Sized {
    type Break;
    type Continue;
    type Mapped<C>: MyTry<Break = Self::Break, Continue = C, Mapped<Self::Continue> = Self>;
    type AsRef<'this>: MyTry<Continue = &'this Self::Continue>
    where
        Self: 'this;
    type AsMut<'this>: MyTry<Continue = &'this mut Self::Continue>
    where
        Self: 'this;

    fn from_control_flow(ctrl: ControlFlow<Self::Break, Self::Continue>) -> Self;
    fn into_control_flow(self) -> ControlFlow<Self::Break, Self::Continue>;

    fn as_ref<'this>(&'this self) -> Self::AsRef<'this>;
    fn as_mut<'this>(&'this mut self) -> Self::AsMut<'this>;

    fn is_continue(&self) -> bool {
        matches!(self.as_ref().into_control_flow(), Continue(_))
    }

    fn is_break(&self) -> bool {
        !self.is_continue()
    }

    fn from_continue(value: Self::Continue) -> Self {
        Self::from_control_flow(Continue(value))
    }

    fn from_break(value: Self::Break) -> Self {
        Self::from_control_flow(Break(value))
    }

    fn into_continue(self) -> Option<Self::Continue> {
        match self.into_control_flow() {
            Continue(x) => Some(x),
            Break(_) => None,
        }
    }

    fn into_break(self) -> Option<Self::Break> {
        match self.into_control_flow() {
            Break(x) => Some(x),
            Continue(_) => None,
        }
    }

    fn map<U>(self, f: impl FnOnce(Self::Continue) -> U) -> Self::Mapped<U> {
        match self.into_control_flow() {
            Continue(x) => MyTry::from_continue(f(x)),
            Break(x) => MyTry::from_break(x),
        }
    }

    fn flatten(self) -> Self::Continue
    where
        Self::Continue: MyTry<Break = Self::Break>,
    {
        match self.into_control_flow() {
            Continue(x) => x,
            Break(x) => MyTry::from_break(x),
        }
    }

    fn flat_map<U>(self, f: impl FnOnce(Self::Continue) -> U) -> U
    where
        U: MyTry<Break = Self::Break>,
    {
        self.map(f).flatten()
    }

    fn transpose(
        self,
    ) -> <Self::Continue as MyTry>::Mapped<Self::Mapped<<Self::Continue as MyTry>::Continue>>
    where
        Self::Continue: MyTry,
    {
        match self.map(MyTry::into_control_flow).into_control_flow() {
            Continue(Continue(x)) => MyTry::from_continue(MyTry::from_continue(x)),
            Continue(Break(x)) => MyTry::from_break(x),
            Break(x) => MyTry::from_continue(MyTry::from_break(x)),
        }
    }
}

pub trait MapBreak<B>: MyTry {
    type BreakMapped: MapBreak<
        Self::Break,
        Break = B,
        Continue = Self::Continue,
        BreakMapped = Self,
    >;
}

impl<B, C> MyTry for ControlFlow<B, C> {
    type Break = B;
    type Continue = C;
    type Mapped<_C> = ControlFlow<B, _C>;
    type AsRef<'this> = ControlFlow<&'this B, &'this C> where Self: 'this;
    type AsMut<'this> = ControlFlow<&'this mut B, &'this mut C> where Self: 'this;

    fn from_control_flow(ctrl: Self) -> Self {
        ctrl
    }

    fn into_control_flow(self) -> Self {
        self
    }

    fn as_ref(&self) -> Self::AsRef<'_> {
        match self {
            Continue(x) => Continue(x),
            Break(x) => Break(x),
        }
    }

    fn as_mut(&mut self) -> Self::AsMut<'_> {
        match self {
            Continue(x) => Continue(x),
            Break(x) => Break(x),
        }
    }
}

impl<B1, C, B2> MapBreak<B2> for ControlFlow<B1, C> {
    type BreakMapped = ControlFlow<B2, C>;
}

impl<T, E> MyTry for Result<T, E> {
    type Break = E;
    type Continue = T;
    type Mapped<_T> = Result<_T, E>;
    type AsRef<'this> = Result<&'this T, &'this E> where Self: 'this;
    type AsMut<'this> = Result<&'this mut T, &'this mut E> where Self: 'this;

    fn from_control_flow(ctrl: ControlFlow<Self::Break, Self::Continue>) -> Self {
        match ctrl {
            Continue(x) => Ok(x),
            Break(x) => Err(x),
        }
    }

    fn into_control_flow(self) -> ControlFlow<Self::Break, Self::Continue> {
        match self {
            Ok(x) => Continue(x),
            Err(x) => Break(x),
        }
    }
    fn as_ref(&self) -> Self::AsRef<'_> {
        self.as_ref()
    }
    fn as_mut(&mut self) -> Self::AsMut<'_> {
        self.as_mut()
    }
}

impl<T, E1, E2> MapBreak<E2> for Result<T, E1> {
    type BreakMapped = Result<T, E2>;
}

impl<T> MyTry for Option<T> {
    type Break = ();
    type Continue = T;
    type Mapped<_T> = Option<_T>;
    type AsRef<'this> = Option<&'this T> where Self: 'this;
    type AsMut<'this> = Option<&'this mut T> where Self: 'this;

    fn from_control_flow(ctrl: ControlFlow<Self::Break, Self::Continue>) -> Self {
        match ctrl {
            Continue(x) => Some(x),
            Break(()) => None,
        }
    }

    fn into_control_flow(self) -> ControlFlow<Self::Break, Self::Continue> {
        match self {
            Some(x) => Continue(x),
            None => Break(()),
        }
    }
    fn as_ref(&self) -> Self::AsRef<'_> {
        self.as_ref()
    }
    fn as_mut(&mut self) -> Self::AsMut<'_> {
        self.as_mut()
    }
}

impl<T> MapBreak<()> for Option<T> {
    type BreakMapped = Self;
}
