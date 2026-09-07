// todo, these should not be used/should be removed

#[macro_export]
macro_rules! array {
    ($ty:expr) => {{
        use $crate::components::Ty;
        Ty::Array(Box::new($ty))
    }};
}

#[macro_export]
macro_rules! option {
    ($ty:expr) => {{
        use $crate::components::Ty;
        Ty::Option(Box::new($ty))
    }};
}
