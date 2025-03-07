//! `std`-like traits, helpers, and type definitions for core I/O functionality.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::fmt;

mod buffered;
mod error;
mod impls;

pub mod prelude;

pub use self::buffered::BufReader;
pub use self::error::{Error, Result};

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

use axerrno::ax_err;

/// The `Read` trait allows for reading bytes from a source.
pub trait Read {
    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Read all bytes until EOF in this source, placing them into `buf`.
    #[cfg(feature = "alloc")]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let start_len = buf.len();
        let mut probe = [0u8; 32];
        loop {
            match self.read(&mut probe) {
                Ok(0) => return Ok(buf.len() - start_len),
                Ok(n) => buf.extend_from_slice(&probe[..n]),
                Err(e) => return Err(e),
            }
        }
    }

    /// Read all bytes until EOF in this source, appending them to `buf`.
    #[cfg(feature = "alloc")]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        unsafe { append_to_string(buf, |b| self.read_to_end(b)) }
    }

    /// Read the exact number of bytes required to fill `buf`.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            ax_err!(UnexpectedEof, "failed to fill whole buffer")
        } else {
            Ok(())
        }
    }
}

/// A trait for objects which are byte-oriented sinks.
pub trait Write {
    /// Write a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    fn flush(&mut self) -> Result;

    /// Attempts to write an entire buffer into this writer.
    fn write_all(&mut self, mut buf: &[u8]) -> Result {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return ax_err!(WriteZero, "failed to write whole buffer"),
                Ok(n) => buf = &buf[n..],
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes a formatted string into this writer, returning any error
    /// encountered.
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<()> {
        // Create a shim which translates a Write to a fmt::Write and saves
        // off I/O errors. instead of discarding them
        struct Adapter<'a, T: ?Sized + 'a> {
            inner: &'a mut T,
            error: Result<()>,
        }

        impl<T: Write + ?Sized> fmt::Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(fmt::Error)
                    }
                }
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(()),
        };
        match fmt::write(&mut output, fmt) {
            Ok(()) => Ok(()),
            Err(..) => {
                // check if the error came from the underlying `Write` or not
                if output.error.is_err() {
                    output.error
                } else {
                    ax_err!(InvalidData, "formatter error")
                }
            }
        }
    }
}

/// A `BufRead` is a type of `Read`er which has an internal buffer, allowing it
/// to perform extra ways of reading.
pub trait BufRead: Read {
    /// Returns the contents of the internal buffer, filling it with more data
    /// from the inner reader if it is empty.
    fn fill_buf(&mut self) -> Result<&[u8]>;

    /// Tells this buffer that `amt` bytes have been consumed from the buffer,
    /// so they should no longer be returned in calls to `read`.
    fn consume(&mut self, amt: usize);

    /// Check if the underlying `Read` has any data left to be read.
    fn has_data_left(&mut self) -> Result<bool> {
        self.fill_buf().map(|b| !b.is_empty())
    }

    /// Read all bytes into `buf` until the delimiter `byte` or EOF is reached.
    #[cfg(feature = "alloc")]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.fill_buf() {
                    Ok(n) => n,
                    Err(Error::Again) => continue,
                    Err(e) => return Err(e),
                };
                match available.iter().position(|&b| b == byte) {
                    Some(i) => {
                        buf.extend_from_slice(&available[..=i]);
                        (true, i + 1)
                    }
                    None => {
                        buf.extend_from_slice(available);
                        (false, available.len())
                    }
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                return Ok(read);
            }
        }
    }

    /// Read all bytes until a newline (the `0xA` byte) is reached, and append
    /// them to the provided `String` buffer.
    #[cfg(feature = "alloc")]
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        unsafe { append_to_string(buf, |b| self.read_until(b'\n', b)) }
    }
}

#[cfg(feature = "alloc")]
unsafe fn append_to_string<F>(buf: &mut String, f: F) -> Result<usize>
where
    F: FnOnce(&mut Vec<u8>) -> Result<usize>,
{
    let old_len = buf.len();
    let buf = unsafe { buf.as_mut_vec() };
    let ret = f(buf)?;
    if core::str::from_utf8(&buf[old_len..]).is_err() {
        ax_err!(InvalidData, "stream did not contain valid UTF-8")
    } else {
        Ok(ret)
    }
}
