#![no_std]
extern crate alloc;
mod layout;
mod config;
mod block_cache;
mod block_dev;

pub use block_dev::BlockDevice;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
