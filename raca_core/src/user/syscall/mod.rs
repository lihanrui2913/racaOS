mod debug;
mod fs;
mod mm;
mod task;

#[allow(unused_variables)]
pub fn syscall_handler(
    idx: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    match idx {
        0 => debug::write(arg1, arg2),
        1 => debug::show_cpu_id(),
        2 => fs::open(arg1, arg2, arg3),
        3 => {
            // 正式的写入 syscall
            fs::write(arg1, arg2, arg3)
        }
        4 => {
            // read

            fs::read(arg1, arg2, arg3)
        }
        5 => {
            // dump hex buffer
            debug::dump_hex_buffer(arg1, arg2)
        }
        6 => {
            // create a new app
            task::create_process(arg1)
        }
        7 => mm::malloc(arg1, arg2),
        8 => mm::free(arg1, arg2, arg3),
        9 => fs::close(arg1),
        10 => fs::lseek(arg1, arg2),
        11 => fs::fsize(arg1),
        12 => fs::open_pipe(arg1),
        _ => 0,
    }
}