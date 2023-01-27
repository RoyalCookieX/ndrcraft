use winit::dpi::{PhysicalPosition, PhysicalSize, Position, Size};

macro_rules! impl_unit {
    ($t:ident,$value:expr) => {
        impl Unit for $t {
            fn one() -> Self {
                $value
            }
        }
    };
}

macro_rules! define_vector_struct {
    ($(#[doc=$doc:expr])? $(#[derive($($der:ident),*)])? $t:ident{$($m:ident),*}) => {
        $(#[doc=$doc])?
        $(#[derive($($der),*)])?
        pub struct $t<T: Unit> {
            $(pub $m: T,)*
        }
    };
}

macro_rules! define_vector_fn_new {
    ($t:ident{$($m:ident),*}) => {
        pub const fn new($($m: T,)*) -> Self {
            Self {
                $($m,)*
            }
        }
    };
}

macro_rules! define_vector_fn_one {
    ($t:ident{$($m:ident),*}) => {
        pub fn one() -> Self {
            Self {
                $($m: T::one(),)*
            }
        }
    };
}

macro_rules! define_offset {
    ($(#[doc=$doc:expr])? $t:ident{$($m:ident),*}) => {
        define_vector_struct!($(#[doc=$doc])? #[derive(Clone, Copy, Debug, Default)] $t{$($m),*});

        impl<T: Unit> $t<T> {
            define_vector_fn_new!($t{$($m),*});
        }
    };
}

macro_rules! define_extent {
    ($(#[doc=$doc:expr])? $t:ident{$($m:ident),*}) => {
        define_vector_struct!($(#[doc=$doc])? #[derive(Clone, Copy, Debug)] $t{$($m),*});

        impl<T: Unit> $t<T> {
            define_vector_fn_new!($t{$($m),*});
            define_vector_fn_one!($t{$($m),*});
        }

        impl<T: Unit> Default for $t<T> {
            fn default() -> Self {
                Self::one()
            }
        }
    };
}

pub trait Unit: Copy + Default {
    fn one() -> Self;
}

impl_unit!(i32, 0);
impl_unit!(u32, 0);
impl_unit!(f32, 0.0);

define_offset!(
    #[doc = "A 2d offset."]
    Offset2d { x, y }
);

impl<T: Unit> From<Extent2d<T>> for Offset2d<T> {
    fn from(value: Extent2d<T>) -> Self {
        Self {
            x: value.width,
            y: value.height,
        }
    }
}

impl From<Offset2d<i32>> for Position {
    fn from(value: Offset2d<i32>) -> Self {
        Self::Physical(PhysicalPosition::new(value.x, value.y))
    }
}

define_offset!(
    #[doc = "A 3d offset."]
    Offset3d { x, y, z }
);

impl<T: Unit> From<Extent3d<T>> for Offset3d<T> {
    fn from(value: Extent3d<T>) -> Self {
        Self {
            x: value.width,
            y: value.height,
            z: value.depth,
        }
    }
}

define_extent!(
    #[doc = "A 2d extent."]
    Extent2d { width, height }
);

impl<T: Unit> From<Offset2d<T>> for Extent2d<T> {
    fn from(value: Offset2d<T>) -> Self {
        Self {
            width: value.x,
            height: value.y,
        }
    }
}

impl From<Extent2d<u32>> for Size {
    fn from(value: Extent2d<u32>) -> Self {
        Self::Physical(PhysicalSize::new(value.width, value.height))
    }
}

define_extent!(
    #[doc = "A 3d extent."]
    Extent3d {
        width,
        height,
        depth
    }
);

impl<T: Unit> From<Offset3d<T>> for Extent3d<T> {
    fn from(value: Offset3d<T>) -> Self {
        Self {
            width: value.x,
            height: value.y,
            depth: value.z,
        }
    }
}
