/// An atomic wrapper with an underlying atomic storage and conversion to
/// a type T
#[derive(Debug, Default)]
pub struct Atomic<S, T>(S::Type, std::marker::PhantomData<T>)
where
    S: AtomicType,
    T: From<S> + Into<S>;
/// Marker type to associate primitive with their atomic versions
pub trait AtomicType {
    type Type;
}
macro_rules! impl_atomic_type {
    ($($t:ident => $Atomic:ident, $newfn:ident),* $(,)?) => { $(
    impl AtomicType for $t {
        type Type = std::sync::atomic::$Atomic;
    }
    impl<T: From<$t> + Into<$t>> Atomic<$t, T> {
        pub const fn $newfn(value: $t) -> Self {
            Self(std::sync::atomic::$Atomic::new(value), std::marker::PhantomData)
        }
        pub fn get(&self) -> T {
            self.0.load(std::sync::atomic::Ordering::Acquire).into()
        }
        pub fn set(&self, value: T) {
            self.0.store(value.into(), std::sync::atomic::Ordering::Release)
        }
    }
    )* }
}
impl_atomic_type! {
    i8 => AtomicI8, new_i8,
    i16 => AtomicI16, new_i16,
    i32 => AtomicI32, new_i32,
    i64 => AtomicI64, new_i64,
    u8 => AtomicU8, new_u8,
    u16 => AtomicU16, new_u16,
    u32 => AtomicU32, new_u32,
    u64 => AtomicU64, new_u64,
    bool => AtomicBool, new_bool,
    isize => AtomicIsize, new_isize,
    usize => AtomicUsize, new_usize,
}
