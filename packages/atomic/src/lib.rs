macro_rules! make {
    ($Atomic:ident, $Storage:ident) => {
        pub struct $Atomic<T>(std::sync::atomic::$Atomic, std::marker::PhantomData<T>)
        where
            T: From<$Storage> + Into<$Storage>;
        impl<T: From<$Storage> + Into<$Storage>> $Atomic<T> {
            pub const fn new(value: $Storage) -> Self {
                Self(
                    std::sync::atomic::$Atomic::new(value),
                    std::marker::PhantomData,
                )
            }
            pub fn get(&self) -> T {
                self.0.load(std::sync::atomic::Ordering::Acquire).into()
            }
            pub fn set(&self, value: T) {
                self.0
                    .store(value.into(), std::sync::atomic::Ordering::Release)
            }
        }
    };
}

make!(AtomicI8, i8);
make!(AtomicI16, i16);
make!(AtomicI32, i32);
make!(AtomicI64, i64);
make!(AtomicU8, u8);
make!(AtomicU16, u16);
make!(AtomicU32, u32);
make!(AtomicU64, u64);
make!(AtomicBool, bool);
make!(AtomicIsize, isize);
make!(AtomicUsize, usize);
