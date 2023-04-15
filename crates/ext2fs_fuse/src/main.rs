#![allow(unused)]
use clap::{App, Arg};
use ext2fs::{BlockDevice, Ext2FileSystem, BLOCK_SIZE, BLOCKS_PER_GRP, EXT2_S_IFDIR, EXT2_S_IFREG};
use std::fs::{read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use log::*;

const NUM_BLOCKS: usize = BLOCKS_PER_GRP;

struct BlockFile {
    file: Mutex<File>,
    num_blocks: usize
}

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE, "Not a complete block!");
    }

    fn block_num(&self) -> usize {
        self.num_blocks
    }

    fn block_size(&self) -> usize {
        BLOCK_SIZE
    }
}

impl BlockFile {
    pub fn new(f: File, num_blocks: usize) -> Self {
        f.set_len((BLOCK_SIZE * num_blocks) as u64);
        Self { file: Mutex::new(f), num_blocks }
    }
}

fn main() {
    env_logger::init();
    efs_test();
}

fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?, NUM_BLOCKS 
    ));
    Ext2FileSystem::create(block_file.clone());
    let efs = Ext2FileSystem::open(
        block_file.clone(), 
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32
    );
    let root_inode = Ext2FileSystem::root_inode(&efs);
    root_inode.create("filea", EXT2_S_IFREG);
    root_inode.create("fileb", EXT2_S_IFREG);
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);

    root_inode.unlink("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(filea.read_at(0, &mut buffer), 0,);
        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SIZE);
    random_str_test(8 * BLOCK_SIZE + BLOCK_SIZE / 2);
    random_str_test(100 * BLOCK_SIZE);
    random_str_test(70 * BLOCK_SIZE + BLOCK_SIZE / 7);
    random_str_test((12 + 128) * BLOCK_SIZE);
    random_str_test(400 * BLOCK_SIZE);
    random_str_test(1000 * BLOCK_SIZE);
    random_str_test(2000 * BLOCK_SIZE);

    Ok(())
}