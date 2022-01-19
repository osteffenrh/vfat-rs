/// A const time assertion.
#[macro_export]
macro_rules! const_assert {
    ($cond:expr) => {
        // Causes overflow if condition is false
        let _ = [(); 0 - (!($cond) as usize)];
    };
    ($($xs:expr),+) => {
        $crate::const_assert!($($xs)&&+);
    };
    ($($xs:expr);+ $(;)*) => {
        $crate::const_assert!($($xs),+);
    };
}

/// A compile time equality assertion
#[macro_export]
macro_rules! const_assert_eq {
    ($x:expr, $($xs:expr),+) => {
        const _: () = { $crate::const_assert!($($x == $xs),+);};
    }
}

/// A compile time size assertion. Can be used to check the actual struct size.
#[macro_export]
macro_rules! const_assert_size {
    ($struct:ident, $size:expr) => {
        $crate::const_assert_eq!(core::mem::size_of::<$struct>(), ($size));
    };
}
