use core::fmt;
use core::marker::PhantomData;

use crate::interface::{FileId, FullAddress, Interface};

#[rustfmt::skip]
#[allow(dead_code)]
pub mod regs;

pub enum Error<IF: Interface> {
    Interface(IF::Error),
}

// The derived Debug requires IF to implement Debug as well,
// though only an associated type is actually used.
impl<IF: Interface> fmt::Debug for Error<IF> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Interface(interface) => write!(f, "Error::Interface({:?})", interface),
        }
    }
}

#[cfg(feature = "defmt")]
impl<IF> defmt::Format for Error<IF>
where
    IF: Interface,
    IF::Error: defmt::Format,
{
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::Interface(interface) => defmt::write!(f, "Error::Interface({:?})", interface),
        }
    }
}

pub struct RegisterAccess<'a, IF> {
    interface: &'a mut IF,
}

impl<'a, IF: Interface> RegisterAccess<'a, IF> {
    pub fn new(interface: &'a mut IF) -> Self {
        Self { interface }
    }
}

pub trait Register {
    const ADDRESS: FullAddress;
    const LENGTH: usize;
}

pub trait FromRaw {
    fn from_raw(value: u64) -> Self;
}

pub trait ToRaw {
    fn to_raw(self) -> u64;
}

pub trait FieldAccess: Register {
    type BaseType;
}

pub trait Read {}
pub trait Clear {}
pub trait Write {}

pub struct Accessor<'a, IF, T> {
    interface: &'a mut IF,
    phantom: PhantomData<T>,
}

impl<'a, IF: Interface, T: Register> Accessor<'a, IF, T> {
    pub fn new(interface: &'a mut IF) -> Self {
        Self {
            interface,
            phantom: PhantomData,
        }
    }
}

impl<'a, IF: Interface, T: Register + Read + FromRaw> Accessor<'a, IF, T> {
    pub fn read(&mut self) -> Result<T, Error<IF>> {
        let value = self
            .interface
            .read_register(T::ADDRESS, T::LENGTH)
            .map_err(Error::Interface)?;
        Ok(T::from_raw(value))
    }
}

impl<'a, IF: Interface, T: Register + Clear + ToRaw> Accessor<'a, IF, T> {
    pub fn clear_value(&mut self, value: T) -> Result<(), Error<IF>> {
        self.interface
            .write_register(T::ADDRESS, T::LENGTH, value.to_raw())
            .map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: Register + Write + ToRaw> Accessor<'a, IF, T> {
    pub fn write_value(&mut self, value: T) -> Result<(), Error<IF>> {
        self.interface
            .write_register(T::ADDRESS, T::LENGTH, value.to_raw())
            .map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: Register + Write + ToRaw + Default> Accessor<'a, IF, T> {
    pub fn write(&mut self, f: impl FnOnce(&mut T)) -> Result<(), Error<IF>> {
        let mut value = T::default();
        f(&mut value);
        self.interface
            .write_register(T::ADDRESS, T::LENGTH, value.to_raw())
            .map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: Register + Read + Write + FromRaw + ToRaw> Accessor<'a, IF, T> {
    pub fn modify(&mut self, f: impl FnOnce(&mut T)) -> Result<(), Error<IF>> {
        let raw_value = self
            .interface
            .read_register(T::ADDRESS, T::LENGTH)
            .map_err(Error::Interface)?;
        let mut value = T::from_raw(raw_value);
        f(&mut value);
        self.interface
            .write_register(T::ADDRESS, T::LENGTH, value.to_raw())
            .map_err(Error::Interface)
    }
}

pub trait File {
    const FILE_ID: FileId;
    const OFFSET: u8;
    const LENGTH: usize;
}

pub trait ZeroOffsetFile: File {}

pub struct FileAccessor<'a, IF, A> {
    interface: &'a mut IF,
    phantom: PhantomData<A>,
}

impl<'a, IF: Interface, T: File> FileAccessor<'a, IF, T> {
    pub fn new(interface: &'a mut IF) -> Self {
        Self {
            interface,
            phantom: PhantomData,
        }
    }
}

impl<'a, IF: Interface, T: File + Read> FileAccessor<'a, IF, T> {
    pub fn read(&mut self, offset: u8, data: &mut [u8]) -> Result<(), Error<IF>> {
        assert!(data.len() <= T::LENGTH);
        let addr = FullAddress::new(T::FILE_ID, T::OFFSET + offset);
        self.interface.read(addr, data).map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: File + Write> FileAccessor<'a, IF, T> {
    pub fn write(&mut self, offset: u8, data: &[u8]) -> Result<(), Error<IF>> {
        assert!(data.len() <= T::LENGTH);
        let addr = FullAddress::new(T::FILE_ID, T::OFFSET + offset);
        self.interface.write(addr, data).map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: ZeroOffsetFile + Read> FileAccessor<'a, IF, T> {
    pub fn read_fast(&mut self, data: &mut [u8]) -> Result<(), Error<IF>> {
        assert!(data.len() <= T::LENGTH);
        self.interface
            .read_fast(T::FILE_ID, data)
            .map_err(Error::Interface)
    }
}

impl<'a, IF: Interface, T: ZeroOffsetFile + Write> FileAccessor<'a, IF, T> {
    pub fn write_fast(&mut self, data: &[u8]) -> Result<(), Error<IF>> {
        assert!(data.len() <= T::LENGTH);
        self.interface
            .write_fast(T::FILE_ID, data)
            .map_err(Error::Interface)
    }
}

#[rustfmt::skip]
macro_rules! base_type {
    (1) => { u8 };
    (2) => { u16 };
    (3) => { u32 };
    (4) => { u32 };
    (5) => { u64 };
    (6) => { u64 };
    (7) => { u64 };
    (8) => { u64 };
}

macro_rules! access {
    ($name:ident, RO) => {
        impl Read for $name {}
    };
    ($name:ident, CO) => {
        impl Clear for $name {}
    };
    ($name:ident, WO) => {
        impl Write for $name {}
    };
    ($name: ident, RC) => {
        access!($name, RO);
        access!($name, CO);
    };
    ($name: ident, RW) => {
        access!($name, RO);
        access!($name, WO);
    };
}

macro_rules! byte_access {
    ($name:ident, $type:ty, RO) => {
        impl<'a, IF: Interface> Accessor<'a, IF, $name> {
            pub fn read_bytes(&mut self) -> Result<$type, Error<IF>> {
                self.interface
                    .read_register($name::ADDRESS, $name::LENGTH)
                    .map_err(Error::Interface)
                    .map(|val| val as $type)
            }
        }
    };
    ($name:ident, $type:ty, CO) => {
        impl<'a, IF: Interface> Accessor<'a, IF, $name> {
            pub fn clear_bytes(&mut self, value: $type) -> Result<(), Error<IF>> {
                self.interface
                    .write_register($name::ADDRESS, $name::LENGTH, value.into())
                    .map_err(Error::Interface)
            }
        }
    };
    ($name:ident, $type:ty, WO) => {
        impl<'a, IF: Interface> Accessor<'a, IF, $name> {
            pub fn write_bytes(&mut self, value: $type) -> Result<(), Error<IF>> {
                self.interface
                    .write_register($name::ADDRESS, $name::LENGTH, value.into())
                    .map_err(Error::Interface)
            }
        }
    };
    ($name: ident, $type:ty, RC) => {
        byte_access!($name, $type, RO);
        byte_access!($name, $type, CO);
    };
    ($name: ident, $type:ty, RW) => {
        byte_access!($name, $type, RO);
        byte_access!($name, $type, WO);
    };
}

macro_rules! default_value {
    ($name: ident, ()) => {};
    ($name: ident, $por_value:literal) => {
        impl Default for $name {
            fn default() -> Self {
                Self::from_raw($por_value)
            }
        }
    };
}

macro_rules! reg_bytes {
    (
        $file_id:literal,
        $offset:literal,
        $length:tt,
        $name:ident,
        $access:tt,
        (),
        $doc:literal
    ) => {
        #[doc=$doc]
        #[derive(Copy, Clone, Debug)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $name(base_type!($length));

        impl Register for $name {
            const ADDRESS: FullAddress = FullAddress::new(FileId::new($file_id), $offset);
            const LENGTH: usize = $length;
        }

        byte_access!($name, base_type!($length), $access);

        paste! {
            impl<'a, IF: Interface> RegisterAccess<'a, IF> {
                #[doc=$doc]
                pub fn [<$name:snake>](&mut self) -> Accessor<'_, IF, $name> {
                    Accessor::new(&mut self.interface)
                }
            }
        }
    };
}

macro_rules! reg_field {
    (
        $file_id:literal,
        $offset:literal,
        $length:tt,
        $name:ident,
        $access:tt,
        $por_value:tt,
        $doc:literal
    ) => {
        reg_bytes!($file_id, $offset, $length, $name, $access, (), $doc);

        impl FieldAccess for $name {
            type BaseType = base_type!($length);
        }

        impl FromRaw for $name {
            fn from_raw(value: u64) -> Self {
                Self(value as base_type!($length))
            }
        }

        impl ToRaw for $name {
            fn to_raw(self) -> u64 {
                self.0.into()
            }
        }

        access!($name, $access);
        default_value!($name, $por_value);
    };
}

macro_rules! mask {
    (
        $type:ty,
        $offset_start:literal,
        $offset_end:literal
    ) => {
        <$type>::MAX >> (<$type>::BITS - ($offset_end - $offset_start)) << $offset_start
    };
}

macro_rules! field_bool {
    (
        $reg_name:ident,
        $offset:literal,
        $field_name:ident,
        $doc:literal
    ) => {
        paste! {
            impl $reg_name {
                #[doc=$doc]
                pub fn $field_name(&self) -> bool {
                    let value = self.0 >> $offset;
                    (value & 1) != 0
                }

                #[doc=$doc]
                pub fn [<set_ $field_name>](&mut self, value: bool) {
                    type BaseType = <$reg_name as FieldAccess>::BaseType;
                    self.0 &= !((1 as BaseType) << $offset);
                    self.0 |= (value as BaseType) << $offset;
                }
            }
        }
    };
}

macro_rules! field_prim {
    (
        $reg_name:ident,
        $offset_start:literal,
        $offset_end:literal,
        $field_name:ident,
        $type:ty,
        $doc:literal
    ) => {
        paste! {
            impl $reg_name {
                #[doc=$doc]
                pub fn $field_name(&self) -> $type {
                    type BaseType = <$reg_name as FieldAccess>::BaseType;
                    const MASK: BaseType = mask!(BaseType, $offset_start, $offset_end);
                    ((self.0 & MASK) >> $offset_start) as $type
                }

                #[doc=$doc]
                pub fn [<set_ $field_name>](&mut self, value: $type) {
                    type BaseType = <$reg_name as FieldAccess>::BaseType;
                    const MASK: BaseType = mask!(BaseType, $offset_start, $offset_end);
                    self.0 &= !MASK;
                    self.0 |= MASK & (value as BaseType) << $offset_start;
                }
            }
        }
    };
}

macro_rules! field_enum {
    (
        $reg_name:ident,
        $offset_start:literal,
        $offset_end:literal,
        $field_name:ident,
        $repr_type:ty,
        $enum_type:ty,
        $doc:literal
    ) => {
        paste! {
            impl $reg_name {
                #[doc=$doc]
                pub fn $field_name(&self) -> $enum_type {
                    type BaseType = <$reg_name as FieldAccess>::BaseType;
                    const MASK: BaseType = mask!(BaseType, $offset_start, $offset_end);
                    let repr = (self.0 & MASK) >> $offset_start;
                    unsafe { core::mem::transmute(repr as $repr_type) }
                }

                #[doc=$doc]
                pub fn [<set_ $field_name>](&mut self, value: $enum_type) {
                    type BaseType = <$reg_name as FieldAccess>::BaseType;
                    const MASK: BaseType= mask!(BaseType, $offset_start, $offset_end);
                    self.0 &= !MASK;
                    self.0 |= MASK & (value as BaseType) << $offset_start;
                }
            }
        }
    };
}

macro_rules! reg_file {
    (
        $file_id:literal,
        (),
        $length:literal,
        $name:ident,
        $access:tt,
        $doc:literal
    ) => {
        reg_file!($file_id, 0, $length, $name, $access, $doc);

        impl ZeroOffsetFile for $name {}
    };
    (
        $file_id:literal,
        $offset:literal,
        $length:literal,
        $name:ident,
        $access:tt,
        $doc:literal
    ) => {
        #[doc=$doc]
        pub struct $name;

        impl File for $name {
            const FILE_ID: FileId = FileId::new($file_id);
            const OFFSET: u8 = $offset;
            const LENGTH: usize = $length;
        }

        access!($name, $access);

        paste! {
            impl<'a, IF: Interface> RegisterAccess<'a, IF> {
                #[doc=$doc]
                pub fn [<$name:snake>](&mut self) -> FileAccessor<'_, IF, $name> {
                    FileAccessor::new(&mut self.interface)
                }
            }
        }
    };
}

pub(crate) use {
    access, base_type, byte_access, default_value, field_bool, field_enum, field_prim, mask,
    reg_bytes, reg_field, reg_file,
};

#[cfg(test)]
mod test {
    use super::*;
    use paste::paste;

    #[repr(u8)]
    #[derive(Clone, Copy, Eq, PartialEq, Debug)]
    pub enum TestEnum {
        Option0 = 0,
        Option1 = 1,
        Option2 = 2,
        Option3 = 3,
    }

    #[rustfmt::skip]
    reg_field!(0x02, 0x07, 4, TestReg, RW, 0b0110_1011_0101, "Test register");

    field_bool!(TestReg, 1, bool_field1, "Bool test field 1");
    field_bool!(TestReg, 2, bool_field2, "Bool test field 2");
    field_prim!(TestReg, 3, 10, prim_field, u8, "Primitive test field");
    field_enum!(TestReg, 10, 12, enum_field, u8, TestEnum, "Enum test field");

    #[test]
    fn default_value() {
        let value = TestReg::default();
        assert_eq!(value.bool_field1(), false);
        assert_eq!(value.bool_field2(), true);
        assert_eq!(value.prim_field(), 0b10_1011_0);
        assert_eq!(value.enum_field(), TestEnum::Option1);
    }

    #[test]
    fn bool_field() {
        let mut value = TestReg::default();
        value.set_bool_field1(false);
        assert_eq!(value.bool_field1(), false);
        value.set_bool_field1(true);
        assert_eq!(value.bool_field1(), true);
    }

    #[test]
    fn primitive_field() {
        let mut value = TestReg::default();
        value.set_prim_field(0x0f);
        assert_eq!(value.prim_field(), 0x0f);
        value.set_prim_field(0xf0);
        assert_eq!(value.prim_field(), 0x70);
    }

    #[test]
    fn enum_field() {
        let mut value = TestReg::default();
        value.set_enum_field(TestEnum::Option1);
        assert_eq!(value.enum_field(), TestEnum::Option1);
        value.set_enum_field(TestEnum::Option2);
        assert_eq!(value.enum_field(), TestEnum::Option2);
    }
}
