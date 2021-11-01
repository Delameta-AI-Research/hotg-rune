/// A version of [`anyhow::ensure!()`] which returns a `*mut Error`.
macro_rules! ensure {
    ($cond:expr $(,)?) => {
        ensure!(
            $cond,
            concat!("Condition failed: `", stringify!($cond), "`"),
        )
    };
    ($cond:expr, $($rest:tt)*) => {
        if !$cond {
            return $crate::Error::new_ptr(anyhow::anyhow!($($rest)*));
        }
    };
}
