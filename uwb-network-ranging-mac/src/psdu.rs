use crate::mac::format::serializer::{ReadBuffer, WriteBuffer};
use core::fmt;

/// Object safe wrapper for static buffer
pub trait PsduContainer {
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn set_length(&mut self, length: usize) -> Result<(), ()>;
    fn as_slice(&self) -> &[u8];
    fn as_mut_slice(&mut self) -> &mut [u8];

    fn set_from_slice(&mut self, other: &[u8]) -> Result<(), ()> {
        self.set_length(other.len())?;
        self.as_mut_slice().copy_from_slice(other);
        Ok(())
    }

    fn read_buffer(&self) -> ReadBuffer<'_>;
    fn write_buffer(&mut self) -> WriteBuffer<'_>;
}

pub struct StaticPsdu<const N: usize> {
    data: [u8; N],
    length: usize,
}

impl<const N: usize> Default for StaticPsdu<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> StaticPsdu<N> {
    pub const fn new() -> Self {
        Self {
            data: [0; N],
            length: 0,
        }
    }
}

impl<const N: usize> Clone for StaticPsdu<N> {
    fn clone(&self) -> Self {
        let mut psdu = Self::new();
        unwrap!(psdu.set_from_slice(self.as_slice()));
        psdu
    }
}

impl<const N: usize> fmt::Debug for StaticPsdu<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <[u8] as fmt::Debug>::fmt(self.as_slice(), f)
    }
}

#[cfg(feature = "defmt")]
impl<const N: usize> defmt::Format for StaticPsdu<N> {
    fn format(&self, fmt: defmt::Formatter<'_>) {
        defmt::write!(fmt, "{=[?]}", self.as_slice())
    }
}

impl<const N: usize> PsduContainer for StaticPsdu<N> {
    fn capacity(&self) -> usize {
        self.data.len()
    }

    fn len(&self) -> usize {
        self.length
    }

    fn set_length(&mut self, length: usize) -> Result<(), ()> {
        if length <= self.data.len() {
            self.length = length;
            Ok(())
        } else {
            Err(())
        }
    }

    fn as_slice(&self) -> &[u8] {
        &self.data[..self.length]
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.length]
    }

    fn read_buffer(&self) -> ReadBuffer<'_> {
        ReadBuffer::new(&self.data[..self.length])
    }

    fn write_buffer(&mut self) -> WriteBuffer<'_> {
        WriteBuffer::new(&mut self.data[self.length..], &mut self.length)
    }
}
