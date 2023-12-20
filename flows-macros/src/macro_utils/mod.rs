use flows::{
    my_try::MyTry,
    ops::{FlowOp, WrapOp},
    Flow,
};

mod normalize_op_input;
pub struct NotCopy<T>(pub T);

pub fn _flow_op<
    'f,
    Yield,
    Resume,
    Return,
    Fl: Flow<Resume, Yield = Yield, Return = Return>,
    Out,
>(
    f: impl FnOnce(Fl) -> Out + 'f,
) -> WrapOp<impl FlowOp<Fl, Resume, Output = Out> + 'f> {
    WrapOp(f)
}

pub fn _flow_to_flow_op<
    'f,
    Yield1,
    Resume1,
    Return1,
    Yield2,
    Resume2,
    Return2,
    Fl: Flow<Resume1, Yield = Yield1, Return = Return1>,
    Out: Flow<Resume2, Yield = Yield2, Return = Return2>,
>(
    f: impl FnOnce(Fl) -> Out + 'f,
) -> WrapOp<
    impl FlowOp<Fl, Resume1, Output = impl Flow<Resume2, Yield = Yield2, Return = Return2>> + 'f,
> {
    _flow_op(f)
}

pub fn _flow_transform_op<
    'f,
    Yield1,
    Resume,
    Return,
    Yield2,
    Fl: Flow<Resume, Yield = Yield1, Return = Return>,
    Out: Flow<Resume, Yield = Yield2, Return = Return>,
>(
    f: impl FnOnce(Fl) -> Out + 'f,
) -> WrapOp<impl FlowOp<Fl, Resume, Output = impl Flow<Resume, Yield = Yield2, Return = Return>> + 'f>
{
    _flow_op(f)
}

pub fn _flow_try_transform_op<
    'f,
    Yield1,
    Resume,
    Return1,
    Yield2,
    Return2: MyTry<Continue = Return1>,
    Fl: Flow<Resume, Yield = Yield1, Return = Return1>,
    Out: Flow<Resume, Yield = Yield2, Return = Return2>,
>(
    f: impl FnOnce(Fl) -> Out + 'f,
) -> WrapOp<
    impl FlowOp<Fl, Resume, Output = impl Flow<Resume, Yield = Yield2, Return = Return2>> + 'f,
> {
    _flow_op(f)
}

#[doc(hidden)]
#[macro_export]
macro_rules! last_ident {
    ($a:ident) => { $a };
    ($a:ident $b:ident) => { $b };
    ($a:ident $b:ident $c:ident) => { $c };
    ($a:ident $b:ident $c:ident $($d:ident)+) => { $crate::last_ident!($($d)+) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! capture_outer {
    { @capture_assign  [$($cap:tt)*] } => {
        $( $crate::capture_outer! { @capture_assign $cap } )*
    };

    { @capture_assign { outer: $cap:tt } } => {
        $crate::capture_outer! { @capture_assign $cap }
    };

    { @capture_assign { inner: $cap:tt } } => {};

    { @capture_assign ($( $($var:ident)+ ),* $(,)?) } => { $(
        let $crate::last_ident!($($var)+) = $crate::macro_utils::NotCopy(
            $crate::capture_outer!(@capture_outer_val $($var)+)
        );
    )* };

    { @capture_outer_val ref mut $name:ident } => { &mut $name };
    { @capture_outer_val ref $name:ident } => { & $name };
    { @capture_outer_val mut $name:ident } => { $name };
    { @capture_outer_val $name:ident } => { $name };
    { @capture_outer_val $op:ident $name:ident } => { $name . $op () };

    { $cap:tt } => {
        $crate::capture_outer! { @capture_assign $cap }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! capture_inner {
    { @capture_assign [$($cap:tt)*] } => {
        $( $crate::capture_inner! { @capture_assign $cap } )*
    };

    { @capture_assign { inner: $cap:tt } } => {
        $crate::capture_inner! { @capture_assign $cap }
    };
    { @capture_assign { outer: $cap:tt } } => {};

    { @capture_assign ($( $($var:ident)+ ),* $(,)?) } => { $(
        let $crate::last_ident!($($var)+) = $crate::last_ident!($($var)+);
        let $crate::macro_utils::NotCopy(
            $crate::capture_inner!(@capture_inner_pat $($var)+)
            ) = $crate::last_ident!($($var)+);
    )* };

    { @capture_inner_pat mut $name:ident } => { mut $name };
    { @capture_inner_pat mut $op:ident $name:ident } => { mut $name };
    { @capture_inner_pat $name:ident } => { $name };
    { @capture_inner_pat $op:ident $name:ident } => { $name };
    { @capture_inner_pat $op1:ident $op2:ident $name:ident } => { $name };

    { $cap:tt } => {
        $crate::capture_inner! { @capture_assign $cap }
    };
}
