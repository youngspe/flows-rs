pub extern crate flows_core;
extern crate flows_macros;
extern crate pin_project_lite;

pub mod flow_impls;
pub mod ops;

pub use convert::{FromFlow, IntoFlow};
pub use flow_impls::{identity, on_each_sync, repeat, repeat_with};
pub use flows_core::Flow;
pub use flows_macros::{flow, flow_of};

pub mod convert {
    pub use flows_core::convert::*;
}
