use std::{cell::RefCell, rc::Rc};

// For convenience, when trying to create
// mutable shared pointers, for single-threaded use.
// Mainly to avoid the nested ::new.
pub type RcRefCell<T> = Rc<RefCell<T>>;

// This is just defined as a namespace, so we can _kind of_
// add extra functions for our type alias.
// Using a mod would also work, but the linter wants them to
// be snake_case, which doesn't match the type name very well.
pub struct RcRefCellT(());

impl RcRefCellT {
    pub fn new<T>(t: T) -> RcRefCell<T> {
        Rc::new(RefCell::new(t))
    }
}
