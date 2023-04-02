use core::{mem::MaybeUninit, num::NonZeroUsize, slice::Iter};
///Abstraction for iterating through an entire gif source
#[derive(Clone)]
pub struct ByteIterator<S: SeekableIter> {
    iterator: S,
    offset: usize,
}
use smallvec::SmallVec;

use crate::common::ParseError;
use core::mem::size_of;

pub trait SeekableIter: Iterator<Item = u8> + Clone {
    /// Goes to absolute byte `offset` in the stream of iteration
    fn seek(&mut self, offset: usize) -> Result<(), usize>;
    /// Moves forward by `len` bytes
    fn move_by(&mut self, len: usize) -> Result<(), usize>;
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
        self.move_by(offset)

        //let mut lv = Some(0);
        //for i in 0..offset {
        //    lv = self.next();
        //}
        //lv.map(|_| ()).ok_or(offset.into())
    }
    fn move_by(&mut self, len: usize) -> Result<(), usize> {
        for i in 0..len {
            self.1.next().ok_or(i)?;
        }
        Ok(())
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
        let mut holder: [u8; 2] = [0, 0];
        holder[0] = self.iterator.next().ok_or(ParseError::UnepectedEOF)?;
        holder[1] = self.iterator.next().ok_or(ParseError::UnepectedEOF)?;

        Ok(u16::from_le_bytes(holder))
    }

    pub(crate) fn take_byte(&mut self) -> Result<u8, ParseError> {
        self.offset += size_of::<u8>();
        let res = self.iterator.next().ok_or(ParseError::UnepectedEOF);

        res
    }

    #[inline]
    pub(crate) fn take_arr<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        self.offset += size_of::<[u8; N]>();

        let mut rv: [u8; N] = unsafe { MaybeUninit::uninit().assume_init() };
        for idx in 0..N {
            rv[idx] = self.iterator.next().ok_or(ParseError::UnepectedEOF)?;
        }
        Ok(rv)

        //self.iterator
        //    .next_chunk()
        //    .map_err(|_| ParseError::UnepectedEOF)
    }
    pub(crate) fn get_offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn seek_by(&mut self, len: usize) -> Result<(), ParseError> {
        self.offset += len;
        self.iterator
            .move_by(len)
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
                .move_by(adv_offset)
                .map_err(|_| ParseError::UnepectedEOF)
        }
    }
}

pub struct ImageIterator<'header, S: SeekableIter> {
    bytes: &'header mut ByteIterator<S>,
    locations: &'header [usize],
    location_offset: usize,
}
