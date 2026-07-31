use core::marker::PhantomData;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NoSpaceLeft;

pub trait FixSerializationLength {
    const SER_LEN: usize;
}

pub trait FixSerializable: FixSerializationLength {
    // The buf size must be equal Self::SER_LEN
    fn serialize(&self, buf: &mut [u8]);
}

pub trait FixDeserializable: FixSerializationLength {
    // The buf size must be equal Self::SER_LEN
    fn deserialize(buf: &[u8]) -> Self;
}

pub trait FixMaybeDeserializable: FixSerializationLength + Sized {
    // The buf size must be equal Self::SER_LEN
    fn try_deserialize(buf: &[u8]) -> Option<Self>;
}

pub struct Placeholder<'a, T> {
    offset: usize,
    _phantom: PhantomData<&'a mut T>,
}

pub struct WriteBuffer<'a> {
    slice: &'a mut [u8],
    parent_len: Option<&'a mut usize>,
    len: usize,
}

impl<'a> WriteBuffer<'a> {
    pub fn new_empty() -> Self {
        Self {
            slice: Default::default(),
            parent_len: None,
            len: 0,
        }
    }

    pub fn new(slice: &'a mut [u8], parent_len: &'a mut usize) -> Self {
        Self {
            slice,
            parent_len: Some(parent_len),
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slice.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn limit_capacity(&mut self, value: usize) {
        _ = self.slice.split_off_mut(value..);
    }

    pub fn append_placeholder<T: FixSerializable>(
        &mut self,
    ) -> Result<Placeholder<'a, T>, NoSpaceLeft> {
        if self.slice.len() - self.len >= T::SER_LEN {
            let offset = self.len;
            self.len += T::SER_LEN;

            Ok(Placeholder {
                offset,
                _phantom: PhantomData,
            })
        } else {
            Err(NoSpaceLeft)
        }
    }

    pub fn write_placeholder<T: FixSerializable>(
        &mut self,
        placeholder: &mut Placeholder<T>,
        field: T,
    ) {
        field.serialize(&mut self.slice[placeholder.offset..placeholder.offset + T::SER_LEN]);
    }

    pub fn append_field<T: FixSerializable>(&mut self, field: T) -> Result<(), NoSpaceLeft> {
        if self.slice.len() - self.len >= T::SER_LEN {
            field.serialize(&mut self.slice[self.len..self.len + T::SER_LEN]);
            self.len += T::SER_LEN;
            Ok(())
        } else {
            Err(NoSpaceLeft)
        }
    }

    pub fn nested_buffer(&mut self, len_limit: usize) -> WriteBuffer<'_> {
        let len = core::cmp::min(self.capacity() - self.len(), len_limit);
        WriteBuffer::new(&mut self.slice[self.len..self.len + len], &mut self.len)
    }

    pub fn commit(self) {
        if let Some(parent_len) = self.parent_len {
            *parent_len += self.len;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReadBuffer<'a> {
    slice: &'a [u8],
}

impl<'a> ReadBuffer<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    pub fn limit_len(&mut self, value: usize) {
        _ = self.slice.split_off(value..);
    }

    pub fn clear(&mut self) {
        self.limit_len(0);
    }

    pub fn pop_field<T: FixDeserializable>(&mut self) -> Result<T, NoSpaceLeft> {
        let prefix = self.slice.split_off(..T::SER_LEN).ok_or(NoSpaceLeft)?;
        Ok(T::deserialize(prefix))
    }

    pub fn try_pop_field<T: FixMaybeDeserializable>(&mut self) -> Result<Option<T>, NoSpaceLeft> {
        if self.slice.len() >= T::SER_LEN {
            let val = T::try_deserialize(&self.slice[..T::SER_LEN]);
            if val.is_some() {
                self.slice = &self.slice[T::SER_LEN..];
            }
            Ok(val)
        } else {
            Err(NoSpaceLeft)
        }
    }

    pub fn pop_buffer(&mut self, len: usize) -> Result<ReadBuffer<'a>, NoSpaceLeft> {
        let prefix = self.slice.split_off(..len).ok_or(NoSpaceLeft)?;
        Ok(ReadBuffer::new(prefix))
    }
}
