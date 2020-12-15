extern crate libc;
use std::ptr;

#[repr(C)] pub struct Env { _private: [u8; 0] }
extern {
    pub fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut Env;
    pub fn free_env(e: *mut Env);
    pub fn get_atomic_int(e: *const Env, index: libc::size_t) -> libc::c_int;
    pub fn get_int(e: *const Env, index: libc::size_t) -> libc::c_int;

    pub fn test(tid: libc::c_int, e: *mut Env);
}

struct Environment {
    num_atomic_ints: usize,
    num_ints: usize,
    env: *mut Env
}

impl Environment {
    pub fn new(atomic_ints: usize, ints: usize) -> Option<Self> {
        let mut e = Environment { num_atomic_ints: atomic_ints, num_ints: ints, env: ptr::null_mut() };
        unsafe {
            e.env = alloc_env(e.num_atomic_ints, e.num_ints);
        }
        if e.env.is_null() {
            None
        } else {
            Some(e)
        }
    }

    pub fn atomic_int(&self, index: usize) -> Option<i32> {
        if self.num_atomic_ints < index {
            None
        } else {
            unsafe {
                Some (get_atomic_int(self.env, index))
            }
        }
    }

    pub fn int(&self, index: usize) -> Option<i32> {
        if self.num_ints < index {
            None
        } else {
            unsafe {
                Some (get_int(self.env, index))
            }
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            free_env(self.env);
            self.env = ptr::null_mut();
        }
    }
}

fn run_thread(tid: i32, e: &mut Environment) {
    unsafe {
        test(tid, e.env);
    }
}

fn main() {
    let mut e = Environment::new(2, 2).unwrap();

    run_thread(0, &mut e);
    println!("x={0}", e.atomic_int(0).unwrap());
    println!("y={0}", e.atomic_int(1).unwrap());
    println!("0:r0={0}", e.int(0).unwrap());
    println!("1:r0={0}", e.int(1).unwrap());

    run_thread(1, &mut e);
    println!("x={0}", e.atomic_int(0).unwrap());
    println!("x={0}", e.atomic_int(0).unwrap());
    println!("y={0}", e.atomic_int(1).unwrap());
    println!("0:r0={0}", e.int(0).unwrap());
    println!("1:r0={0}", e.int(1).unwrap());
}
