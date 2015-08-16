// This Source Code Form is subject to the terms of
// the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You
// can obtain one at http://mozilla.org/MPL/2.0/.

//! CBOR ([RFC 7049](http://tools.ietf.org/html/rfc7049))
//! encoder implementation.
//!
//! This module provides an `Encoder` to directly encode Rust types into
//! CBOR, and a `GenericEncoder` which encodes a `Value` into CBOR.
//!
//! # Example 1: Direct encoding
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.u16(1000).unwrap();
//!     assert_eq!("1903e8".from_hex().unwrap(), e.into_writer().into_inner())
//! }
//! ```
//!
//! # Example 2: Direct encoding (indefinite string)
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.text_iter(vec!["strea", "ming"].into_iter()).unwrap();
//!     let output = "7f657374726561646d696e67ff".from_hex().unwrap();
//!     assert_eq!(output, e.into_writer().into_inner())
//! }
//! ```
//!
//! # Example 3: Direct encoding (nested array)
//!
//! ```
//! extern crate cbor;
//! extern crate rustc_serialize;
//!
//! use cbor::Encoder;
//! use rustc_serialize::hex::FromHex;
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut e = Encoder::new(Cursor::new(Vec::new()));
//!     e.array(3)
//!      .and(e.u8(1))
//!      .and(e.array(2)).and(e.u8(2)).and(e.u8(3))
//!      .and(e.array(2)).and(e.u8(4)).and(e.u8(5))
//!      .unwrap();
//!     let output = "8301820203820405".from_hex().unwrap();
//!     assert_eq!(output, e.into_writer().into_inner())
//! }
//! ```

use byteorder::{self, BigEndian, WriteBytesExt};
use std::io;
use std::error::Error;
use std::fmt;
use types::{Simple, Tag, Type};

// Encoder Error Type ///////////////////////////////////////////////////////

pub type EncodeResult = Result<(), EncodeError>;

#[derive(Debug)]
pub enum EncodeError {
    /// Some I/O error
    IoError(io::Error),
    /// The end of file has been encountered unexpectedly
    UnexpectedEOF,
    /// The provided `Simple` value is neither unassigned nor reserved
    InvalidSimpleValue(Simple)
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            EncodeError::IoError(ref e)            => write!(f, "EncodeError: I/O error: {}", *e),
            EncodeError::UnexpectedEOF             => write!(f, "EncodeError: unexpected end-of-file"),
            EncodeError::InvalidSimpleValue(ref s) => write!(f, "EncodeError: invalid simple value {:?}", s)
        }
    }
}

impl Error for EncodeError {
    fn description(&self) -> &str {
        "EncodeError"
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EncodeError::IoError(ref e) => Some(e),
            _                           => None
        }
    }
}

impl From<byteorder::Error> for EncodeError {
    fn from(e: byteorder::Error) -> EncodeError {
        match e {
            byteorder::Error::UnexpectedEOF => EncodeError::UnexpectedEOF,
            byteorder::Error::Io(e)         => EncodeError::IoError(e)
        }
    }
}

impl From<io::Error> for EncodeError {
    fn from(e: io::Error) -> EncodeError {
        EncodeError::IoError(e)
    }
}

// Encoder //////////////////////////////////////////////////////////////////

/// The actual encoder type definition
pub struct Encoder<W> {
    writer: W
}

impl<W: WriteBytesExt> Encoder<W> {
    pub fn new(w: W) -> Encoder<W> {
        Encoder { writer: w }
    }

    pub fn into_writer(self) -> W {
        self.writer
    }

    pub fn u8(&mut self, x: u8) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0...23 => w.write_u8(x).map_err(From::from),
            _      => w.write_u8(24).and(w.write_u8(x)).map_err(From::from)
        }
    }

    pub fn u16(&mut self, x: u16) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0...23    => w.write_u8(x as u8).map_err(From::from),
            24...0xFF => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            _         => w.write_u8(25).and(w.write_u16::<BigEndian>(x)).map_err(From::from)
        }
    }

    pub fn u32(&mut self, x: u32) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0...23         => w.write_u8(x as u8).map_err(From::from),
            24...0xFF      => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            0x100...0xFFFF => w.write_u8(25).and(w.write_u16::<BigEndian>(x as u16)).map_err(From::from),
            _              => w.write_u8(26).and(w.write_u32::<BigEndian>(x)).map_err(From::from)
        }
    }

    pub fn u64(&mut self, x: u64) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0...23                => w.write_u8(x as u8).map_err(From::from),
            24...0xFF             => w.write_u8(24).and(w.write_u8(x as u8)).map_err(From::from),
            0x100...0xFFFF        => w.write_u8(25).and(w.write_u16::<BigEndian>(x as u16)).map_err(From::from),
            0x100000...0xFFFFFFFF => w.write_u8(26).and(w.write_u32::<BigEndian>(x as u32)).map_err(From::from),
            _                     => w.write_u8(27).and(w.write_u64::<BigEndian>(x)).map_err(From::from)
        }
    }

    pub fn i8(&mut self, x: i8) -> EncodeResult {
        if x >= 0 {
            self.u8(x as u8)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u8 {
                n @ 0...23 => w.write_u8(0b001_00000 | n).map_err(From::from),
                n          => w.write_u8(0b001_00000 | 24).and(w.write_u8(n)).map_err(From::from)
            }
        }
    }

    pub fn i16(&mut self, x: i16) -> EncodeResult {
        if x >= 0 {
            self.u16(x as u16)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u16 {
                n @ 0...23    => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24...0xFF => w.write_u8(0b001_00000 | 24).and(w.write_u8(n as u8)).map_err(From::from),
                n             => w.write_u8(0b001_00000 | 25).and(w.write_u16::<BigEndian>(n)).map_err(From::from)
            }
        }
    }

    pub fn i32(&mut self, x: i32) -> EncodeResult {
        if x >= 0 {
            self.u32(x as u32)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u32 {
                n @ 0...23         => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24...0xFF      => w.write_u8(0b001_00000 | 24).and(w.write_u8(n as u8)).map_err(From::from),
                n @ 0x100...0xFFFF => w.write_u8(0b001_00000 | 25).and(w.write_u16::<BigEndian>(n as u16)).map_err(From::from),
                n                  => w.write_u8(0b001_00000 | 26).and(w.write_u32::<BigEndian>(n)).map_err(From::from)
            }
        }
    }

    pub fn i64(&mut self, x: i64) -> EncodeResult {
        if x >= 0 {
            self.u64(x as u64)
        } else {
            let ref mut w = self.writer;
            match (-1 - x) as u64 {
                n @ 0...23                => w.write_u8(0b001_00000 | n as u8).map_err(From::from),
                n @ 24...0xFF             => w.write_u8(0b001_00000 | 24).and(w.write_u8(n as u8)).map_err(From::from),
                n @ 0x100...0xFFFF        => w.write_u8(0b001_00000 | 25).and(w.write_u16::<BigEndian>(n as u16)).map_err(From::from),
                n @ 0x100000...0xFFFFFFFF => w.write_u8(0b001_00000 | 26).and(w.write_u32::<BigEndian>(n as u32)).map_err(From::from),
                n                         => w.write_u8(0b001_00000 | 27).and(w.write_u64::<BigEndian>(n)).map_err(From::from)
            }
        }
    }

    pub fn f32(&mut self, x: f32) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 26)
            .and(self.writer.write_f32::<BigEndian>(x))
            .map_err(From::from)
    }

    pub fn f64(&mut self, x: f64) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 27)
            .and(self.writer.write_f64::<BigEndian>(x))
            .map_err(From::from)
    }

    pub fn bool(&mut self, x: bool) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | if x {21} else {20}).map_err(From::from)
    }

    pub fn simple(&mut self, x: Simple) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            Simple::Unassigned(n) => match n {
                0...19 | 28...30 => w.write_u8(0b111_00000 | n).map_err(From::from),
                32...255         => w.write_u8(0b111_00000 | 24).and(w.write_u8(n)).map_err(From::from),
                _                => Err(EncodeError::InvalidSimpleValue(x))
            },
            Simple::Reserved(n) => match n {
                0...31 => w.write_u8(0b111_00000 | 24).and(w.write_u8(n)).map_err(From::from),
                _      => Err(EncodeError::InvalidSimpleValue(x))
            }
        }
    }

    pub fn bytes(&mut self, x: &[u8]) -> EncodeResult {
        self.type_len(Type::Bytes, x.len() as u64)
            .and(self.writer.write_all(x).map_err(From::from))
    }

    /// Indefinite byte string encoding. (RFC 7049 section 2.2.2)
    pub fn bytes_iter<'r, I: Iterator<Item=&'r [u8]>>(&mut self, iter: I) -> EncodeResult {
        try!(self.writer.write_u8(0b010_11111));
        for x in iter {
            try!(self.bytes(x))
        }
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }

    pub fn text(&mut self, x: &str) -> EncodeResult {
        self.type_len(Type::Text, x.len() as u64)
            .and(self.writer.write_all(x.as_bytes()).map_err(From::from))
    }

    /// Indefinite string encoding. (RFC 7049 section 2.2.2)
    pub fn text_iter<'r, I: Iterator<Item=&'r str>>(&mut self, iter: I) -> EncodeResult {
        try!(self.writer.write_u8(0b011_11111));
        for x in iter {
            try!(self.text(x))
        }
        self.writer.write_u8(0b111_11111).map_err(From::from)
    }

    pub fn null(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 22).map_err(From::from)
    }

    pub fn undefined(&mut self) -> EncodeResult {
        self.writer.write_u8(0b111_00000 | 23).map_err(From::from)
    }

    pub fn tag(&mut self, x: Tag) -> EncodeResult {
        self.type_len(Type::Tagged, x.to())
    }

    pub fn array(&mut self, len: usize) -> EncodeResult {
        self.type_len(Type::Array, len as u64)
    }

    /// Indefinite array encoding. (RFC 7049 section 2.2.1)
    pub fn array_begin(&mut self) -> EncodeResult {
        self.writer.write_u8(0b100_11111).map_err(From::from)
    }

    /// End of indefinite array encoding. (RFC 7049 section 2.2.1)
    pub fn array_end(&mut self) -> EncodeResult {
        self.writer.write_u8(0b100_11111).map_err(From::from)
    }

    pub fn object(&mut self, len: usize) -> EncodeResult {
        self.type_len(Type::Object, len as u64)
    }

    /// Indefinite object encoding. (RFC 7049 section 2.2.1)
    pub fn object_begin<F>(&mut self) -> EncodeResult {
        self.writer.write_u8(0b101_11111).map_err(From::from)
    }

    /// End of indefinite object encoding. (RFC 7049 section 2.2.1)
    pub fn object_end<F>(&mut self) -> EncodeResult {
        self.writer.write_u8(0b101_11111).map_err(From::from)
    }

    fn type_len(&mut self, t: Type, x: u64) -> EncodeResult {
        let ref mut w = self.writer;
        match x {
            0...23                => w.write_u8(t.major() << 5 | x as u8).map_err(From::from),
            24...0xFF             => w.write_u8(t.major() << 5 | 24).and(w.write_u8(x as u8)).map_err(From::from),
            0x100...0xFFFF        => w.write_u8(t.major() << 5 | 25).and(w.write_u16::<BigEndian>(x as u16)).map_err(From::from),
            0x100000...0xFFFFFFFF => w.write_u8(t.major() << 5 | 26).and(w.write_u32::<BigEndian>(x as u32)).map_err(From::from),
            _                     => w.write_u8(t.major() << 5 | 27).and(w.write_u64::<BigEndian>(x)).map_err(From::from)
        }
    }
}

// Tests ////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use rustc_serialize::hex::FromHex;
    use std::{f32, f64};
    use std::io::Cursor;
    use super::*;
    use types::{Simple, Tag};

    #[test]
    fn unsigned() {
        encoded("00", |mut e| e.u8(0));
        encoded("01", |mut e| e.u8(1));
        encoded("0a", |mut e| e.u8(10));
        encoded("17", |mut e| e.u8(23));
        encoded("1818", |mut e| e.u8(24));
        encoded("1819", |mut e| e.u8(25));
        encoded("1864", |mut e| e.u8(100));
        encoded("1903e8", |mut e| e.u16(1000));
        encoded("1a000f4240", |mut e| e.u32(1000000));
        encoded("1b000000e8d4a51000", |mut e| e.u64(1000000000000));
        encoded("1bffffffffffffffff", |mut e| e.u64(18446744073709551615))
    }

    #[test]
    fn signed() {
        encoded("20", |mut e| e.i8(-1));
        encoded("29", |mut e| e.i8(-10));
        encoded("3863", |mut e| e.i8(-100));
        encoded("3901f3", |mut e| e.i16(-500));
        encoded("3903e7", |mut e| e.i16(-1000));
        encoded("3a00053d89", |mut e| e.i32(-343434));
        encoded("3b000000058879da85", |mut e| e.i64(-23764523654))
    }

    #[test]
    fn bool() {
        encoded("f4", |mut e| e.bool(false));
        encoded("f5", |mut e| e.bool(true))
    }

    #[test]
    fn simple() {
        encoded("f0", |mut e| e.simple(Simple::Unassigned(16)));
        encoded("f818", |mut e| e.simple(Simple::Reserved(24)));
        encoded("f8ff", |mut e| e.simple(Simple::Unassigned(255)))
    }

    #[test]
    fn float() {
        encoded("fa47c35000", |mut e| e.f32(100000.0));
        encoded("fa7f7fffff", |mut e| e.f32(3.4028234663852886e+38));
        encoded("fbc010666666666666", |mut e| e.f64(-4.1));

        encoded("fa7f800000", |mut e| e.f32(f32::INFINITY));
        encoded("faff800000", |mut e| e.f32(-f32::INFINITY));
        encoded("fa7fc00000", |mut e| e.f32(f32::NAN));

        encoded("fb7ff0000000000000", |mut e| e.f64(f64::INFINITY));
        encoded("fbfff0000000000000", |mut e| e.f64(-f64::INFINITY));
        encoded("fb7ff8000000000000", |mut e| e.f64(f64::NAN));
    }

    #[test]
    fn bytes() {
        encoded("4401020304", |mut e| e.bytes(&vec![1,2,3,4][..]));
    }

    #[test]
    fn text() {
        encoded("62c3bc", |mut e| e.text("\u{00fc}"));
        encoded("781f64667364667364660d0a7364660d0a68656c6c6f0d0a736466736673646673", |mut e| {
            e.text("dfsdfsdf\r\nsdf\r\nhello\r\nsdfsfsdfs")
        });
    }

    #[test]
    fn indefinite_text() {
        encoded("7f657374726561646d696e67ff", |mut e| {
            e.text_iter(vec!["strea", "ming"].into_iter())
        })
    }

    #[test]
    fn indefinite_bytes() {
        encoded("5f457374726561446d696e67ff", |mut e| {
            e.bytes_iter(vec!["strea".as_bytes(), "ming".as_bytes()].into_iter())
        })
    }

    #[test]
    fn option() {
        encoded("f6", |mut e| e.null())
    }

    #[test]
    fn tagged() {
        encoded("c11a514b67b0", |mut e| {
            try!(e.tag(Tag::Timestamp));
            e.u32(1363896240)
        })
    }

    #[test]
    fn array() {
        encoded("83010203", |mut e| {
            try!(e.array(3));
            try!(e.u32(1));
            try!(e.u32(2));
            e.u32(3)
        });
        encoded("8301820203820405", |mut e| {
            e.array(3)
             .and(e.u8(1))
             .and(e.array(2))
                .and(e.u8(2))
                .and(e.u8(3))
             .and(e.array(2))
                .and(e.u8(4))
                .and(e.u8(5))
        })
    }

    #[test]
    fn object() {
        encoded("a26161016162820203", |mut e| {
            try!(e.object(2));
            try!(e.text("a").and(e.u8(1)));
            e.text("b").and(e.array(2)).and(e.u8(2)).and(e.u8(3))
        })
    }

    fn encoded<F>(expected: &str, mut f: F)
    where F: FnMut(Encoder<Cursor<&mut [u8]>>) -> EncodeResult
    {
        let mut buffer = vec![0u8; 128];
        assert!(f(Encoder::new(Cursor::new(&mut buffer[..]))).is_ok());
        assert_eq!(&expected.from_hex().unwrap()[..], &buffer[0 .. expected.len() / 2])
    }
}
