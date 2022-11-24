use core::slice::Iter;
///Abstraction for iterating through an entire gif source
#[derive(Clone)]
pub struct ByteIterator<S: SeekableIter> {
    iterator: S,
    offset: usize,
}
use crate::common::ParseError;
use core::mem::size_of;

pub trait SeekableIter: Iterator<Item = u8> + Clone {
    /// Goes to byte `offset` in the stream of iteration
    fn seek(&mut self, offset: usize) -> Result<(), usize>;
}

impl<'a> Clone for SeekableSliceIter<'a> {
    fn clone(&self) -> Self {
        Self(&self.0, self.1.clone())
    }
}

pub struct SeekableSliceIter<'a>(&'a [u8], Iter<'a, u8>);

impl Iterator for SeekableSliceIter<'_> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().copied()
    }
}

impl SeekableIter for SeekableSliceIter<'_> {
    fn seek(&mut self, offset: usize) -> Result<(), usize> {
        self.1 = self.0.iter();
        self.1.advance_by(offset)
    }
}

impl<'a> SeekableSliceIter<'a> {
    fn new(slice: &'a [u8]) -> Self {
        Self(slice, slice.iter())
    }
}

impl<'a> ByteIterator<SeekableSliceIter<'a>> {
    pub fn from_slice(iterator: &'a [u8]) -> ByteIterator<SeekableSliceIter> {
        Self {
            iterator: SeekableSliceIter::new(iterator),
            offset: 0,
        }
    }
}

impl<S: SeekableIter> ByteIterator<S> {
    pub(crate) fn take_u16_le(&mut self) -> Result<u16, ParseError> {
        self.offset += size_of::<u16>();
        self.iterator
            .next_chunk()
            .map(|val| u16::from_le_bytes(val))
            .map_err(|_| ParseError::UnepectedEOF)
    }

    pub(crate) fn take_byte(&mut self) -> Result<u8, ParseError> {
        self.offset += size_of::<u8>();
        let res = self.iterator.next().ok_or(ParseError::UnepectedEOF);

        res
    }

    #[inline]
    pub(crate) fn take_arr<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        self.offset += size_of::<[u8; N]>();
        self.iterator
            .next_chunk()
            .map_err(|_| ParseError::UnepectedEOF)
    }
    pub(crate) fn get_offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn seek_by(&mut self, len: usize) -> Result<(), ParseError> {
        self.offset += len;
        self.iterator
            .advance_by(len)
            .map_err(|_| ParseError::UnepectedEOF)
    }

    pub(crate) fn seek_to(&mut self, offset: usize) -> Result<(), ParseError> {
        if self.offset > offset {
            self.iterator.seek(offset);
            self.offset = offset;
            Ok(())
        } else {
            let adv_offset = offset - self.offset;
            self.offset = offset;
            self.iterator
                .advance_by(adv_offset)
                .map_err(|_| ParseError::UnepectedEOF)
        }
    }
}

pub struct ImageIterator<'header, S: SeekableIter> {
    bytes: &'header mut ByteIterator<S>,
    locations: &'header [usize],
    location_offset: usize,
}
