use core::alloc::Layout;

use crate::{
    fs::{
        operation::OpenMode,
        vfs::inode::{FileInfo, InodeTy},
    },
    user::get_current_process,
};
use alloc::{string::String, vec, vec::Vec};
use framework::{
    memory::{addr_to_array, addr_to_mut_ref, write_for_syscall},
    ref_to_mut,
};

use x86_64::VirtAddr;

pub fn open(buf_addr: usize, buf_len: usize, open_mode: usize) -> usize {
    let mut buf = vec![0; buf_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(buf_addr as u64),
        buf_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", buf_addr);
    }

    let path = String::from(core::str::from_utf8(buf.as_slice()).unwrap());

    let open_mode = match open_mode {
        0 => OpenMode::Read,
        1 => OpenMode::Write,
        _ => return 0,
    };

    let fd = crate::fs::operation::open(path.clone(), open_mode);
    if let Some(fd) = fd {
        fd
    } else {
        0
    }
}

pub fn write(fd: usize, buf_addr: usize, buf_len: usize) -> usize {
    let mut buf = vec![0; buf_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(buf_addr as u64),
        buf_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", buf_addr);
    }

    crate::fs::operation::write(fd, buf.as_slice())
}

pub fn read(fd: usize, buf_addr: usize, buf_len: usize) -> usize {
    let mut buf = vec![0; buf_len];

    let len = crate::fs::operation::read(fd, buf.as_mut());

    write_for_syscall(VirtAddr::new(buf_addr as u64), buf.as_slice());

    let mut buf = vec![0; buf_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(buf_addr as u64),
        buf_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", buf_addr);
    }

    len
}

pub fn close(fd: usize) -> usize {
    if let Some(_) = crate::fs::operation::close(fd) {
        1
    } else {
        0
    }
}

pub fn lseek(fd: usize, offset: usize) -> usize {
    if let Some(_) = crate::fs::operation::lseek(fd, offset) {
        1
    } else {
        0
    }
}

pub fn fsize(fd: usize) -> usize {
    crate::fs::operation::fsize(fd).unwrap_or(0)
}

pub fn open_pipe(buf_addr: usize) -> usize {
    let buffer: &mut [usize] = addr_to_array::<usize>(VirtAddr::new(buf_addr as u64), 2);
    if let Some(_) = crate::fs::operation::open_pipe(buffer) {
        1
    } else {
        0
    }
}

pub fn dir_item_num(path_addr: usize, path_len: usize) -> usize {
    let mut buf = vec![0; path_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(path_addr as u64),
        path_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", path_addr);
    }

    let path = String::from(core::str::from_utf8(buf.as_slice()).unwrap());

    let file_infos = crate::fs::operation::list_dir(path);

    file_infos.len()
}

pub fn list_dir(path_addr: usize, path_len: usize, buf_addr: usize) -> usize {
    let mut buf = vec![0; path_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(path_addr as u64),
        path_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", path_addr);
    }

    let path = String::from(core::str::from_utf8(buf.as_slice()).unwrap());

    #[derive(Clone)]
    #[allow(dead_code)]
    struct TemporyInfo {
        name: &'static [u8],
        ty: InodeTy,
    }

    let file_infos: Vec<TemporyInfo> = {
        let infos = crate::fs::operation::list_dir(path);
        let mut new_infos = Vec::new();
        for info in infos.iter() {
            let FileInfo { name, ty } = info;
            let new_name = ref_to_mut(&*get_current_process().read())
                .heap
                .allocate(Layout::from_size_align(name.len(), 8).unwrap())
                .unwrap();
            let new_name = addr_to_array(VirtAddr::new(new_name), name.len());
            new_name[..name.len()].copy_from_slice(name.as_bytes());
            new_infos.push(TemporyInfo {
                name: new_name,
                ty: *ty,
            });
        }
        new_infos
    };

    write_for_syscall(VirtAddr::new(buf_addr as u64), file_infos.as_slice());

    0
}

pub fn change_cwd(path_addr: usize, path_len: usize) -> usize {
    let mut buf = vec![0; path_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(path_addr as u64),
        path_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", path_addr);
    }

    let path = String::from(core::str::from_utf8(buf.as_slice()).unwrap());

    crate::fs::operation::change_cwd(path);

    0
}

pub fn get_cwd() -> usize {
    let path = crate::fs::operation::get_cwd();
    let new_path_ptr = ref_to_mut(&*get_current_process().read())
        .heap
        .allocate(Layout::from_size_align(path.len(), 8).unwrap())
        .unwrap();
    let new_path = addr_to_array(VirtAddr::new(new_path_ptr), path.len());
    new_path[..path.len()].copy_from_slice(path.as_bytes());
    let ret_struct_ptr = ref_to_mut(&*get_current_process().read())
        .heap
        .allocate(Layout::from_size_align(16, 8).unwrap())
        .unwrap();
    let path_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr));
    *path_ptr = new_path_ptr;
    let len_ptr = addr_to_mut_ref(VirtAddr::new(ret_struct_ptr + 8));
    *len_ptr = path.len();
    ret_struct_ptr as usize
}

pub fn create(path_addr: usize, path_len: usize, ty: usize) -> usize {
    let mut buf = vec![0; path_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(path_addr as u64),
        path_len,
        &mut buf,
    ) {
        panic!("Read error at {:x}!", path_addr);
    }

    let path = String::from(core::str::from_utf8(buf.as_slice()).unwrap());
    let ty = match ty {
        0 => InodeTy::Dir,
        1 => InodeTy::File,
        _ => return 0,
    };
    crate::fs::operation::create(path, ty).unwrap_or(0)
}

pub fn get_type(fd: usize) -> usize {
    match crate::fs::operation::get_type(fd) {
        Some(ty) => ty as usize,
        None => usize::MAX,
    }
}

pub fn mount(
    to_path_addr: usize,
    to_path_len: usize,
    p_path_addr: usize,
    p_path_len: usize,
) -> usize {
    let mut to_buf = vec![0; to_path_len];
    let mut p_buf = vec![0; p_path_len];

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(to_path_addr as u64),
        to_path_len,
        &mut to_buf,
    ) {
        panic!("Read error at {:x}!", to_path_addr);
    }

    if let Err(_) = get_current_process().read().page_table.read(
        VirtAddr::new(p_path_addr as u64),
        p_path_len,
        &mut p_buf,
    ) {
        panic!("Read error at {:x}!", p_path_addr);
    }

    let to_path = String::from(core::str::from_utf8(to_buf.as_slice()).unwrap());
    let p_path = String::from(core::str::from_utf8(p_buf.as_slice()).unwrap());

    if let Some(_) = crate::fs::operation::mount(to_path, p_path) {
        1
    } else {
        0
    }
}
