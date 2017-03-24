#![feature(specialization)]

extern crate tar;

use tar::{Header};

use std::io::{Read, Seek, SeekFrom, Cursor};
use std::mem::{size_of};
use std::path::{PathBuf};
use std::slice::{from_raw_parts};

pub const BLOCK_SZ: usize = 512;

pub fn cast_bytes_as_u64_slice(buf: &[u8]) -> &[u64] {
  assert_eq!(0, buf.len() % 8);
  unsafe { from_raw_parts(buf.as_ptr() as *const u64, buf.len() / 8) }
}

pub fn cast_bytes_as_header(buf: &[u8]) -> &Header {
  assert_eq!(size_of::<Header>(), buf.len());
  unsafe { &*(buf.as_ptr() as *const Header) }
}

pub trait TarBufferExt {
  fn raw_header(&mut self, pos: u64) -> &[u8];

  fn raw_entries<'a>(&'a mut self) -> Result<TarRawEntries<'a>, ()> where Self: Sized {
    Ok(TarRawEntries{
      buffer:   self,
      pos:      0,
      //idx:      0,
      closed:   false,
    })
  }
}

pub struct TarBuffer<A> {
  inner:    A,
  blockbuf: Option<Vec<u8>>,
}

impl<A> TarBuffer<A> where A: Read + Seek {
  pub fn new(inner: A) -> Self {
    TarBuffer{
      inner:    inner,
      blockbuf: None,
    }
  }
}

impl<A> TarBufferExt for TarBuffer<A> where A: Read + Seek {
  default fn raw_header(&mut self, pos: u64) -> &[u8] {
    if self.blockbuf.is_none() {
      let mut h = Vec::with_capacity(BLOCK_SZ);
      h.resize(BLOCK_SZ, 0);
      self.blockbuf = Some(h);
    }
    self.inner.seek(SeekFrom::Start(pos)).unwrap();
    self.inner.read_exact(self.blockbuf.as_mut().unwrap()).unwrap();
    self.blockbuf.as_ref().unwrap()
  }
}

impl<A> TarBufferExt for TarBuffer<Cursor<A>> where A: AsRef<[u8]> {
  fn raw_header(&mut self, pos: u64) -> &[u8] {
    let offset = pos as usize;
    &self.inner.get_ref().as_ref()[offset .. offset + BLOCK_SZ]
  }
}

pub struct TarRawEntry {
  pub header_pos:   u64,
  pub path:         PathBuf,
  pub entry_pos:    u64,
  pub entry_sz:     u64,
}

impl TarRawEntry {
  pub fn raw_file_position(&self) -> u64 {
    self.entry_pos
  }

  pub fn file_size(&self) -> u64 {
    self.entry_sz
  }
}

pub struct TarRawEntries<'a> {
  buffer:   &'a mut TarBufferExt,
  pos:      u64,
  //idx:      usize,
  closed:   bool,
}

impl<'a> Iterator for TarRawEntries<'a> {
  type Item = Result<TarRawEntry, ()>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.closed {
      return None;
    }
    let header_pos = self.pos;
    let entry_pos = header_pos + BLOCK_SZ as u64;
    let mut path = None;
    let mut entry_sz = 0;
    let mut eof = true;
    {
      let raw_header = self.buffer.raw_header(header_pos);
      for &w in cast_bytes_as_u64_slice(raw_header).iter() {
        if w != 0 {
          eof = false;
          break;
        }
      }
      if !eof {
        let header = cast_bytes_as_header(raw_header);
        path = Some((*header.path().unwrap()).into());
        entry_sz = header.entry_size().unwrap();
        assert_eq!(entry_sz, header.size().unwrap());
      }
    }
    if eof {
      let raw_header2 = self.buffer.raw_header(header_pos + BLOCK_SZ as u64);
      let mut eof2 = true;
      for &w in cast_bytes_as_u64_slice(raw_header2).iter() {
        if w != 0 {
          eof2 = false;
          break;
        }
      }
      assert!(eof2);
      self.pos = header_pos + 2 * BLOCK_SZ as u64;
      self.closed = true;
      None
    } else {
      let next_pos = entry_pos + ((entry_sz as usize + BLOCK_SZ - 1) / BLOCK_SZ * BLOCK_SZ) as u64;
      self.pos = next_pos;
      //self.idx += 1;
      Some(Ok(TarRawEntry{
        header_pos:   header_pos,
        path:         path.unwrap(),
        entry_pos:    entry_pos,
        entry_sz:     entry_sz,
      }))
    }
  }
}
