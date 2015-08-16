// This Source Code Form is subject to the terms of
// the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You
// can obtain one at http://mozilla.org/MPL/2.0/.

//! CBOR (RFC 7049) encoder and decoder implementations.

extern crate byteorder;
extern crate libc;

#[cfg(test)]
extern crate rustc_serialize;

pub mod types;
pub mod decoder;
pub mod encoder;
pub mod skip;

pub use decoder::{Config, Decoder, DecodeError, DecodeResult};
pub use decoder::{opt, maybe, or_break};
pub use encoder::{Encoder, EncodeError, EncodeResult};
