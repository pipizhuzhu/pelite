
use std::{io, ptr, slice};
use std::ffi::OsStr;
use std::path::Path;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::RawHandle;
use std::os::raw::c_void;

const INVALID_HANDLE_VALUE: RawHandle = !0 as RawHandle;
const NULL: RawHandle = 0 as RawHandle;
const FALSE: i32 = 0;

extern "system" {
	fn CreateFileW(
		lpFileName: *const u16,
		dwDesiredAccess: u32,
		dwShareMode: u32,
		lpSecurityAttributes: *const c_void,
		dwCreationDisposition: u32,
		dwFlagsAndAttributes: u32,
		hTemplateFile: RawHandle,
	) -> RawHandle;
	fn CreateFileMappingW(
		hFile: RawHandle,
		lpAttributes: *const c_void,
		flProtect: u32,
		dwMaximumSizeHigh: u32,
		dwMaximumSizeLow: u32,
		lpName: *const u16,
	) -> RawHandle;
	fn MapViewOfFile(
		hFileMappingObject: RawHandle,
		dwDesiredAccess: u32,
		dwFileOffsetHigh: u32,
		dwFileOffsetLow: u32,
		dwNumberOfBytesToMap: usize,
	) -> *const c_void;
	fn UnmapViewOfFile(
		lpBaseAddress: *const c_void,
	) -> i32;
	fn GetFileSizeEx(
		hFile: RawHandle,
		lpFileSize: *mut u64,
	) -> i32;
	fn CloseHandle(
		hObject: RawHandle,
	) -> i32;
	fn VirtualAlloc(
		lpAddress: *const c_void,
		dwSize: usize,
		flAllocationType: u32,
		flProtect: u32,
	) -> *mut c_void;
	fn LoadLibraryW(
		lpFileName: *const u16,
	) -> *const c_void;
}

macro_rules! close_handle {
	($e:expr) => {
		let e = CloseHandle($e);
		debug_assert!(e != FALSE, "CloseHandle failed with {:?}", io::Error::last_os_error());
	}
}

//----------------------------------------------------------------

/// Memory-mapped image.
pub struct ImageMap {
	map: RawHandle,
	view: *const c_void,
}
impl ImageMap {
	/// Maps the executable image into memory with correctly aligned sections.
	pub fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<ImageMap> {
		Self::_open(path.as_ref())
	}
	fn _open(path: &Path) -> io::Result<ImageMap> { unsafe {
		// Get its file handle
		let file = {
			// Get the path as a nul terminated wide string
			let path: &OsStr = path.as_ref();
			let mut wpath: Vec<u16> = path.encode_wide().collect();
			wpath.push(0);
			CreateFileW(wpath.as_ptr(), /*GENERIC_READ*/0x80000000, /*FILE_SHARE_READ*/0x00000001, ptr::null(), /*OPEN_EXISTING*/3, /*FILE_ATTRIBUTE_NORMAL*/0x00000080, NULL)
		};
		if file != INVALID_HANDLE_VALUE {
			// Create the image file mapping, `SEC_IMAGE` does its magic thing
			let map = CreateFileMappingW(file, ptr::null(), /*PAGE_READONLY*/0x02 | /*SEC_IMAGE*/0x1000000, 0, 0, ptr::null());
			close_handle!(file);
			if map != NULL {
				// Map view of the file
				let view = MapViewOfFile(map, /*FILE_MAP_READ*/0x0004, 0, 0, 0);
				if view != ptr::null() {
					// All good! Trust the OS with correctly mapping the image...
					return Ok(ImageMap { map, view });
				}
				let err = io::Error::last_os_error();
				close_handle!(map);
				return Err(err);
			}
		}
		Err(io::Error::last_os_error())
	}}
}
impl AsRef<[u8]> for ImageMap {
	fn as_ref(&self) -> &[u8] {
		unimplemented!()
	}
}
impl Drop for ImageMap {
	fn drop(&mut self) { unsafe {
		UnmapViewOfFile(self.view);
		close_handle!(self.map);
	}}
}

//----------------------------------------------------------------

/// Memory-mapped file.
pub struct FileMap {
	map: RawHandle,
	view: *const c_void,
	size: usize,
}
impl FileMap {
	/// Maps the whole file into memory.
	pub fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<FileMap> {
		Self::_open(path.as_ref())
	}
	fn _open(path: &Path) -> io::Result<FileMap> { unsafe {
		// Get its file handle
		let file = {
			// Get the path as a nul terminated wide string
			let path: &OsStr = path.as_ref();
			let mut wpath: Vec<u16> = path.encode_wide().collect();
			wpath.push(0);
			CreateFileW(wpath.as_ptr(), /*GENERIC_READ*/0x80000000, /*FILE_SHARE_READ*/0x00000001, ptr::null(), /*OPEN_EXISTING*/3, /*FILE_ATTRIBUTE_NORMAL*/0x00000080, NULL)
		};
		if file == INVALID_HANDLE_VALUE {
			return Err(io::Error::last_os_error());
		}
		// Get the file size as we'll be mapping it wholesome
		let mut file_size = 0u64;
		let e = GetFileSizeEx(file, &mut file_size);
		let size = file_size as usize;
		if e == FALSE {
			let err = io::Error::last_os_error();
			close_handle!(file);
			return Err(err);
		}
		// Create the memory file mapping
		let map = CreateFileMappingW(file, ptr::null(), /*PAGE_READONLY*/0x02, 0, 0, ptr::null());
		close_handle!(file);
		if map == NULL {
			return Err(io::Error::last_os_error());
		}
		// Map view of the file
		let view = MapViewOfFile(map, /*FILE_MAP_READ*/0x0004, 0, 0, 0);
		if view == ptr::null() {
			let err = io::Error::last_os_error();
			close_handle!(map);
			return Err(err);
		}
		Ok(FileMap { map, view, size })
	}}
}
impl AsRef<[u8]> for FileMap {
	fn as_ref(&self) -> &[u8] { unsafe {
		slice::from_raw_parts(self.view as *const u8, self.size)
	}}
}
impl Drop for FileMap {
	fn drop(&mut self) { unsafe {
		UnmapViewOfFile(self.view);
		close_handle!(self.map);
	}}
}

//----------------------------------------------------------------

use pe::{Pe, PeView, Va};
use pe::imports::Desc;
use pe::exports::Export;

pub trait ManualMap {
	unsafe fn mmap(self) -> *mut u8;
}
impl<'a, P: Pe<'a> + Copy> ManualMap for P {
	unsafe fn mmap(self) -> *mut u8 {
		let v = mm_alloc(self);
		mm_copy(self, v);
		mm_rebase(self, v, v as usize);
		mm_deps(self, v, mm_deps_load);
		mm_tls(self, v);
		mm_protect(self, v);
		return v;
	}
}

pub unsafe fn mm_alloc<'a, P: Pe<'a> + Copy>(pe: P) -> *mut u8 {
	let image_size = pe.optional_header().SizeOfImage as usize;
	let vbase = VirtualAlloc(ptr::null_mut(), image_size, /*MEM_COMMIT|MEM_RESERVE*/0x00003000, /*PAGE_READWRITE*/0x04);
	vbase as *mut u8
}

pub unsafe fn mm_copy<'a, P: Pe<'a> + Copy>(pe: P, image: *mut u8) {
	let src = pe.image().as_ptr();

	// write PE header
	let size_of_headers = pe.optional_header().SizeOfHeaders as usize;
	ptr::copy_nonoverlapping(src, image, size_of_headers);

	for sec in pe.section_headers() {
		// write section data
		ptr::copy_nonoverlapping(
			src.offset(sec.PointerToRawData as isize),
			image.offset(sec.VirtualAddress as isize),
			sec.SizeOfRawData as usize
		);
	}
}

pub unsafe fn mm_rebase<'a, P: Pe<'a> + Copy>(pe: P, image: *mut u8, virt_base: usize) {
	let delta = {
		let image_base = pe.optional_header().ImageBase as usize;
		virt_base.wrapping_sub(image_base)
	};

	let base_relocs = pe.base_relocs().unwrap();
	for rva in base_relocs.into_iter().flat_map(|relocs| relocs) {
		let p = image.offset(rva as isize) as *mut usize;
		let fixed_addr = ptr::read_unaligned(p).wrapping_add(delta);
		ptr::write_unaligned(p, fixed_addr);
	}
}

pub unsafe fn mm_deps<'a, P: Pe<'a> + Copy, F: FnMut(&Desc<'a, P>, *mut u8)>(pe: P, image: *mut u8, mut f: F) {
	// Resolve all dependent modules
	let imports = pe.imports().unwrap();
	for desc in imports {
		f(&desc, image);
	}
}
pub fn mm_deps_load<'a, P: Pe<'a> + Copy>(desc: &Desc<'a, P>, image: *mut u8) {
	// Load dependencies through bog-standard LoadLibrary
	let dll_name = desc.dll_name().unwrap().to_os_str();
	let hmod = {
		let wide_name: Vec<u16> = dll_name.encode_wide().collect();
		unsafe { LoadLibraryW(wide_name.as_ptr()) as *const u8 }
	};
	// Fill in the imports from this loaded module
	let view = unsafe { PeView::module(hmod) };
	unsafe { mm_deps_import(desc, image, view) };
}
pub unsafe fn mm_deps_import<'a, 'b, P: Pe<'a> + Copy, Q: Pe<'b> + Copy>(desc: &Desc<'a, P>, image: *mut u8, dep: Q) {
	let int = desc.int().unwrap();
	let exp_by = dep.exports().unwrap().by().unwrap();
	// Grab the IAT to write to
	let iat_ptr = image.offset(desc.image().FirstThunk as isize) as *mut Va;
	let iat_len = int.as_slice().len();
	let iat = slice::from_raw_parts_mut(iat_ptr, iat_len);
	// Loop over name table
	for (imp, dest) in int.zip(iat) {
		let imp = imp.unwrap();
		match exp_by.import(imp).unwrap() {
			Export::Symbol(&rva) => {
				// And write the exported VA to the IAT
				*dest = dep.rva_to_va(rva).unwrap();
			}
			_ => unimplemented!(),
		}
	}
}

pub unsafe fn mm_tls<'a, P: Pe<'a> + Copy>(pe: P, image: *mut u8) {
	unimplemented!()
}

pub unsafe fn mm_protect<'a, P: Pe<'a> + Copy>(pe: P, image: *mut u8) {
	unimplemented!()
}
