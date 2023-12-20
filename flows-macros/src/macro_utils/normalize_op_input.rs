#[doc(hidden)]
#[macro_export]
macro_rules! normalize_op_input {
    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt : $arg0_ty:ty),* } | -> $return_ty:ty { $($body:tt)* } $($rest:tt)* } => {
        $crate::normalize_op_input! {
            $out { $($pre)* | $( $arg0: $arg0_ty ),* | -> $return_ty {$($body)*} }
            $($rest)*
        }
    };
    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt : $arg0_ty:ty),* } | -> $return_ty:ty $body:block $($rest:tt)* } => {
        $crate::normalize_op_input! {
            $out { $($pre)* | $( $arg0: $arg0_ty ),* | -> $return_ty { $body } }
            $($rest)*
        }
    };
    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt : $arg0_ty:ty),* } | { $($body:tt)* } $($rest:tt)* } => {
        $crate::normalize_op_input! {
            $out { $($pre)* | $( $arg0: $arg0_ty ),* | -> _ {$($body)*} }
            $($rest)*
        }
    };
    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt)* } | $body:expr $(,$($rest:tt)*)? } => {
        $crate::normalize_op_input! {
            $out { $($pre)* | $($arg0)* | -> _ { $body } }
            $(,$($rest)*)?
        }
    };
    { @fn $out:tt { $($pre:tt)* } | $arg0:tt, | $($rest:tt)* } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            | $($rest)*
        }
    };

    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt : $arg0_ty:ty),* } $arg:tt : $arg_ty:ty $(, $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | { $( $arg0: $arg0_ty, )* $arg: $arg_ty }
            $($($rest)*)?
        }
    };
    { @fn $out:tt { $($pre:tt)* } | { $($arg0:tt : $arg0_ty:ty),* } $($arg:tt : $arg_ty:ty),+  $(| $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | { $( $arg0: $arg0_ty, )* $($arg : $arg_ty),* }
            $(| $($rest)*)?
        }
    };

    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $($arg:ident)+ $(: $arg_ty:ty)? $(, $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            ( $($arg)+ ) $(: $arg_ty)?
            $(, $($rest)*)?
        }
    };
    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $($($arg:ident)+ $(: $arg_ty:ty)?),+  $(| $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            $( ( $($arg)+ ) $(: $arg_ty)? )+
            $(| $($rest)*)?
        }
    };

    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $arg:tt $(, $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            $arg: _
            $(, $($rest)*)?
        }
    };
    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $($arg:tt),+  $(| $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            $($arg : _),*
            $(| $($rest)*)?
        }
    };

    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $arg:pat_param $(, $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            ( $arg )
            $(, $($rest)*)?
        }
    };
    { @fn $out:tt { $($pre:tt)* } | $arg0:tt $($arg:pat_param),+  $(| $($rest:tt)*)? } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* } | $arg0
            $( ( $arg ) ),+
            $(| $($rest)*)?
        }
    };

    { ($($out:tt)*) $pre:tt $(,)? } => {
         $($out)*! $pre
    };

    { $out:tt { $($pre:tt)* } $($x:ident)* || $($rest:tt)* } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* $($x)* } | {  }
            | $($rest)*
        }
    };
    { $out:tt { $($pre:tt)* } $($x:ident)* | $($rest:tt)* } => {
        $crate::normalize_op_input! { @fn
            $out { $($pre)* $($x)* } | {}
            $($rest)*
        }
    };

    { $out:tt { $($pre:tt)* } #[$($attr:tt)*] $($rest:tt)* } => {
        $crate::normalize_op_input! {
            $out { $($pre)* #[$($attr)*] }
            $($rest)*
        }
    };

    { $out:tt { $($pre:tt)* } $($x:ident)+ $(, $($rest:tt)*)? } => {
        $crate::normalize_op_input! {
            $out { $($pre)* $($x)* }
            $(, $($rest)*)?
        }
    };
    { $out:tt { $($pre:tt)* } $x:tt $($rest:tt)* } => {
        $crate::normalize_op_input! {
            $out { $($pre)* $x }
            $($rest)*
        }
    };
    { $out:tt { $($pre:tt)* } , $($rest:tt)+} => {
        $crate::normalize_op_input! { @inner
            @out { $($pre)* , }
            $($rest)*
        }
    };
}
