extern crate libc;

use libc::{c_void, size_t, syscall};

fn kmalloc() -> *mut c_void {
    unsafe { syscall(548) as *mut c_void }
}

fn main() {
    let ptr = kmalloc();
    println!("Allocated memory at {:?}", ptr);
    // do something with allocated memory
    //kfree(ptr)
}
