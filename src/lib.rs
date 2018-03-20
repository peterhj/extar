#![feature(align_offset)]
#![feature(specialization)]

use std::ffi::{CStr};
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::mem::{align_of, size_of};
use std::path::{PathBuf};
use std::slice::{from_raw_parts};
use std::str::{from_utf8};

pub const BLOCK_SZ: u64 = 512;

pub fn cast_bytes_as_u32_slice(buf: &[u8]) -> &[u32] {
  assert_eq!(0, buf.as_ptr().align_offset(align_of::<u32>()));
  assert_eq!(0, buf.len() % size_of::<u32>());
  unsafe { from_raw_parts(buf.as_ptr() as *const u32, buf.len() / size_of::<u32>()) }
}

pub trait RawBufferedTarExt {
  fn raw_header(&mut self, pos: u64) -> &[u8];

  fn raw_entries<'a>(&'a mut self) -> RawTarEntries<'a> where Self: Sized {
    RawTarEntries{
      buffer:   self,
      pos:      0,
      closed:   false,
    }
  }
}

pub struct BufferedTarFile<A> {
  inner:    A,
  blockbuf: Option<Vec<u8>>,
}

impl<A> BufferedTarFile<A> where A: Read + Seek {
  pub fn new(inner: A) -> Self {
    BufferedTarFile{
      inner:    inner,
      blockbuf: None,
    }
  }
}

impl<A> RawBufferedTarExt for BufferedTarFile<A> where A: Read + Seek {
  default fn raw_header(&mut self, pos: u64) -> &[u8] {
    if self.blockbuf.is_none() {
      let mut h = Vec::with_capacity(BLOCK_SZ as usize);
      h.resize(BLOCK_SZ as usize, 0);
      self.blockbuf = Some(h);
    }
    self.inner.seek(SeekFrom::Start(pos)).unwrap();
    self.inner.read_exact(self.blockbuf.as_mut().unwrap()).unwrap();
    self.blockbuf.as_ref().unwrap()
  }
}

impl<A> RawBufferedTarExt for BufferedTarFile<Cursor<A>> where A: AsRef<[u8]> {
  fn raw_header(&mut self, pos: u64) -> &[u8] {
    let offset = pos as usize;
    &self.inner.get_ref().as_ref()[offset .. offset + BLOCK_SZ as usize]
  }
}

pub struct RawTarEntry {
  pub header_pos:   u64,
  pub entry_pos:    u64,
  pub entry_sz:     u64,
  pub is_file:      bool,
  pub path:         PathBuf,
}

impl RawTarEntry {
  pub fn raw_file_position(&self) -> u64 {
    self.entry_pos
  }

  pub fn size(&self) -> u64 {
    self.entry_sz
  }
}

pub struct RawTarEntries<'a> {
  buffer:   &'a mut RawBufferedTarExt,
  pos:      u64,
  closed:   bool,
}

impl<'a> Iterator for RawTarEntries<'a> {
  type Item = Result<RawTarEntry, ()>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.closed {
      return None;
    }
    let header_pos = self.pos;
    let entry_pos = header_pos + BLOCK_SZ;
    let mut eof = true;
    {
      let header_buf = self.buffer.raw_header(header_pos);
      for &w in cast_bytes_as_u32_slice(header_buf).iter() {
        if w != 0 {
          eof = false;
          break;
        }
      }
      if !eof {
        let mut path_len = 0;
        for k in 0 .. 100 {
          if header_buf[k] == 0 {
            path_len = k;
            break;
          }
        }
        let path_cstr = match CStr::from_bytes_with_nul(&header_buf[ .. path_len + 1]) {
          Err(e) => panic!("failed to parse file path ({:?}): {:?}", e, &header_buf[ .. 100]),
          Ok(s) => s,
        };
        let path = PathBuf::from(path_cstr.to_str().unwrap());
        let entry_sz_str = match from_utf8(&header_buf[124 .. 136 - 1]) {
          Err(e) => panic!("failed to parse entry sz ({:?}): {:?}", e, &header_buf[124 .. 136]),
          Ok(s) => s,
        };
        let entry_sz = u64::from_str_radix(entry_sz_str, 8).unwrap();
        let typeflag = header_buf[156];
        let is_file = match typeflag {
          b'0' | b'\0' => true,
          _ => false,
        };
        self.pos = entry_pos + (entry_sz + BLOCK_SZ - 1) / BLOCK_SZ * BLOCK_SZ;
        return Some(Ok(RawTarEntry{
          header_pos:   header_pos,
          entry_pos:    entry_pos,
          entry_sz:     entry_sz,
          is_file:      is_file,
          path:         path,
        }));
      }
    }
    let mut eof2 = true;
    {
      let header_buf2 = self.buffer.raw_header(header_pos + BLOCK_SZ);
      for &w in cast_bytes_as_u32_slice(header_buf2).iter() {
        if w != 0 {
          eof2 = false;
          break;
        }
      }
      assert!(eof2, "tar file is missing a terminal block");
      self.pos = header_pos + 2 * BLOCK_SZ;
      self.closed = true;
    }
    None
  }
}
