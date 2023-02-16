pub use cgmath::*;

use std::{mem, ops, slice};
use winit::dpi::{PhysicalPosition, PhysicalSize};

#[macro_export(local_inner_macros)]
macro_rules! impl_from_error {
    ($sub:ty, $super:ty, $error:ident) => {
        impl From<$sub> for $super {
            fn from(value: $sub) -> Self {
                <$super>::$error(value)
            }
        }
    };
}

macro_rules! impl_unit {
    ($t:ident,$value:expr) => {
        impl Unit for $t {
            fn one() -> Self {
                $value
            }
        }
    };
}

macro_rules! impl_ops_element {
    ($op_ty:ident, $op_func:ident, $op:tt, $t:ident{$($t_m:ident),*}, $rhs:ident<$($rhs_generics:ident),*>{$($rhs_m:ident),*}) => {
        impl<T: Unit> ops::$op_ty<$rhs<$($rhs_generics,)*>> for $t<T> {
            type Output = Self;

            fn $op_func(self, rhs: $rhs<$($rhs_generics,)*>) -> Self::Output {
                Self {
                    $($t_m: self.$t_m $op rhs.$rhs_m,)*
                }
            }
        }
    };
}

macro_rules! impl_ops_self {
    ($op_ty:ident, $op_func:ident, $op:tt, $t:ident{$($m:ident),*}) => {
        impl_ops_element!($op_ty, $op_func, $op, $t{$($m),*}, $t<T>{$($m),*});
    };
}

macro_rules! impl_ops_scalar {
    ($op_ty:ident<$rhs:ident>, $op_func:ident, $op:tt, $t:ident{$($m:ident),*}) => {
        impl<T: Unit> ops::$op_ty<$rhs> for $t<T> {
            type Output = Self;

            fn $op_func(self, rhs: $rhs) -> Self::Output {
                Self {
                    $($m: self.$m $op rhs,)*
                }
            }
        }
    };
}

macro_rules! define_vector_struct {
    ($(#[doc=$doc:expr])?  $t:ident{$($m:ident),*}) => {
        $(#[doc=$doc])?
        #[derive(Clone, Copy, Debug)]
        pub struct $t<T: Unit> {
            $(pub $m: T,)*
        }

        impl<T: Unit> $t<T> {
            pub const fn new($($m: T,)*) -> Self {
                Self {
                    $($m,)*
                }
            }
        }

        impl<T: Unit + PartialEq> PartialEq for $t<T> {
            fn eq(&self, other: &Self) -> bool {
                $(self.$m == other.$m)&&*
            }
        }

        impl_ops_self!(Add, add, +, $t{$($m),*});
        impl_ops_self!(Sub, sub, -, $t{$($m),*});
        impl_ops_scalar!(Mul<T>, mul, *, $t{$($m),*});
        impl_ops_scalar!(Div<T>, div, /, $t{$($m),*});
    };
}

macro_rules! define_offset {
    ($(#[doc=$doc:expr])? $t:ident{$($m:ident),*}) => {
        define_vector_struct!($(#[doc=$doc])? $t{$($m),*});

        impl<T: Unit> Default for $t<T> {
            fn default() -> Self {
                Self {
                    $($m: Default::default(),)*
                }
            }
        }

        impl<T: Unit + ops::Neg<Output = T>> ops::Neg for $t<T> {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self {
                    $($m: -self.$m,)*
                }
            }
        }
    };
}

macro_rules! define_extent {
    ($(#[doc=$doc:expr])? $t:ident{$($m:ident),*}) => {
        define_vector_struct!($(#[doc=$doc])? $t{$($m),*});

        impl<T: Unit> $t<T> {
            pub fn one() -> Self {
                Self {
                    $($m: T::one(),)*
                }
            }

            pub fn is_valid(&self) -> bool {
                $(self.$m > T::zero())&&*
            }
        }

        impl<T: Unit> Default for $t<T> {
            fn default() -> Self {
                Self::one()
            }
        }
    };
}

pub trait Unit:
    Copy
    + Default
    + PartialOrd
    + ops::Add<Self, Output = Self>
    + ops::AddAssign<Self>
    + ops::Sub<Self, Output = Self>
    + ops::SubAssign<Self>
    + ops::Mul<Self, Output = Self>
    + ops::MulAssign<Self>
    + ops::Div<Self, Output = Self>
    + ops::DivAssign<Self>
{
    fn one() -> Self;
    fn zero() -> Self {
        Self::default()
    }
}

impl_unit!(i32, 1);
impl_unit!(u32, 1);
impl_unit!(f32, 1.0);
impl_unit!(i64, 1);
impl_unit!(u64, 1);
impl_unit!(f64, 1.0);

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

impl From<PhysicalPosition<i32>> for Offset2d<i32> {
    fn from(value: PhysicalPosition<i32>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<Offset2d<i32>> for PhysicalPosition<i32> {
    fn from(value: Offset2d<i32>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl_ops_element!(Add, add, +, Offset2d { x, y }, Extent2d<T> { width, height });
impl_ops_element!(Sub, sub, -, Offset2d { x, y }, Extent2d<T> { width, height });

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

impl From<Offset3d<u32>> for wgpu::Origin3d {
    fn from(value: Offset3d<u32>) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl_ops_element!(Add, add, +, Offset3d { x, y, z }, Extent3d<T> { width, height, depth });
impl_ops_element!(Sub, sub, -, Offset3d { x, y, z }, Extent3d<T> { width, height, depth });

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

impl From<PhysicalSize<u32>> for Extent2d<u32> {
    fn from(value: PhysicalSize<u32>) -> Self {
        Self::new(value.width, value.height)
    }
}

impl From<Extent2d<u32>> for PhysicalSize<u32> {
    fn from(value: Extent2d<u32>) -> Self {
        Self::new(value.width, value.height)
    }
}

impl_ops_element!(Add, add, +, Extent2d { width, height }, Offset2d<T> { x, y });
impl_ops_element!(Sub, sub, -, Extent2d { width, height }, Offset2d<T> { x, y });

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

impl From<wgpu::Extent3d> for Extent3d<u32> {
    fn from(value: wgpu::Extent3d) -> Self {
        Self {
            width: value.width,
            height: value.height,
            depth: value.depth_or_array_layers,
        }
    }
}

impl From<Extent3d<u32>> for wgpu::Extent3d {
    fn from(value: Extent3d<u32>) -> Self {
        Self {
            width: value.width,
            height: value.height,
            depth_or_array_layers: value.depth,
        }
    }
}

impl_ops_element!(Add, add, +, Extent3d { width, height, depth }, Offset3d<T> { x, y, z });
impl_ops_element!(Sub, sub, -, Extent3d { width, height, depth }, Offset3d<T> { x, y, z });

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Color<T: Unit> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T: Unit> Color<T> {
    pub const fn new(r: T, g: T, b: T, a: T) -> Self {
        Self { r, g, b, a }
    }

    pub fn clear() -> Self {
        Self::new(T::zero(), T::zero(), T::zero(), T::zero())
    }

    pub fn black() -> Self {
        Self::new(T::zero(), T::zero(), T::zero(), T::one())
    }

    pub fn red() -> Self {
        Self::new(T::one(), T::zero(), T::zero(), T::one())
    }

    pub fn green() -> Self {
        Self::new(T::zero(), T::one(), T::zero(), T::one())
    }

    pub fn blue() -> Self {
        Self::new(T::zero(), T::zero(), T::one(), T::one())
    }

    pub fn white() -> Self {
        Self::new(T::one(), T::one(), T::one(), T::one())
    }
}

impl From<wgpu::Color> for Color<f64> {
    fn from(value: wgpu::Color) -> Self {
        Self::new(value.r, value.g, value.b, value.a)
    }
}

impl From<Color<f64>> for wgpu::Color {
    fn from(value: Color<f64>) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

macro_rules! impl_bytes {
    ($t:ident) => {
        unsafe impl Bytes for $t {}
    };
    ($t:ident<$($generic:ident),*>) => {
        unsafe impl<$($generic: Unit,)*> Bytes for $t<$($generic,)*> {}
    };
}

pub unsafe trait Bytes: Copy + Sized {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self as *const Self;
        let data = ptr as *const u8;
        let len = mem::size_of_val(self);
        unsafe { slice::from_raw_parts(data, len) }
    }
}

pub unsafe trait ByteArray<const SIZE: usize>: Bytes {
    fn as_byte_array(&self) -> &[u8; SIZE] {
        unsafe { mem::transmute::<&Self, &[u8; SIZE]>(self) }
    }
}

impl_bytes!(bool);
impl_bytes!(i8);
impl_bytes!(u8);
impl_bytes!(char);
impl_bytes!(i16);
impl_bytes!(u16);
impl_bytes!(i32);
impl_bytes!(u32);
impl_bytes!(f32);
impl_bytes!(i64);
impl_bytes!(u64);
impl_bytes!(f64);
impl_bytes!(i128);
impl_bytes!(u128);
impl_bytes!(isize);
impl_bytes!(usize);
impl_bytes!(Color<T>);
impl_bytes!(Deg<T>);
impl_bytes!(Rad<T>);
impl_bytes!(Vector2<T>);
impl_bytes!(Vector3<T>);
impl_bytes!(Vector4<T>);
impl_bytes!(Quaternion<T>);
impl_bytes!(Matrix4<T>);

unsafe impl<T: Bytes, const SIZE: usize> Bytes for [T; SIZE] {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let data = ptr as *const u8;
        let len = mem::size_of::<T>() * self.len();
        unsafe { slice::from_raw_parts(data, len) }
    }
}

unsafe impl<T: Bytes> Bytes for &[T] {
    fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let data = ptr as *const u8;
        let len = mem::size_of::<T>() * self.len();
        unsafe { slice::from_raw_parts(data, len) }
    }
}

unsafe impl<T: Bytes, const SIZE: usize> ByteArray<SIZE> for [T; SIZE] {}
