# 进展日志

## 第6周
1. 运行 arceos 的 `helloworld`，浏览了多线程相关的代码。
2. 阅读 ext3 的论文 "Journaling the Linux ext2fs Filesystem"，了解了如何高效实现日志和故障恢复，论文地址：https://pdos.csail.mit.edu/6.S081/2020/readings/journal-design.pdf
3. 阅读了 xv6 文件系统部分的代码，它实现了一个简单的带有 log 的文件系统，但是不如 ext3 高效。

### 问题
1. 据我了解，ext4 文件系统有很多的 feature 在 arceos 中无法体现（比如 superblock 的备份，block extent之类的），而且类似于 flexible block group 的 feature 所要求的文件系统容量也比较大，我们在实现文件系统时是否需要严格按照其规范实现？还是说实现其关键的 feature 和功能即可。


### 下一周计划
1. 将 xv6 的文件系统迁移至 arceos，预计实现顺序为：block_manager, 支持 concurrency 的 buffer_manager, 文件（夹）读写、创建删除、软链接等、LOG机制。
2. 进一步调研 ext3 中的LOG机制。

## 第7周
1. 调整了一下目标，因为 ext4 过于复杂，决定按照 ext2 -> （增加 log 机制）ext3 -> （添加其他特性）类ext4
的顺序来进行实现。
2. 阅读了 [The Second Extended File System](https://www.nongnu.org/ext2-doc/ext2.html)，完全了解
了 ext2 的磁盘布局、文件索引等信息。
3. 在 `crates/ext2fs` 中实现了 ext2 文件系统的所有磁盘数据结构，并完成了一部分 buffer_manager 的代码。

### 问题
1. `virtio-driver` 的 block size 是 512 bytes，但是 ext2 要求至少是 1024 bytes，而我想要实现的是 4096 bytes。这个大小是否可以调整呢。
2. 不清楚应该在 crate 中还是在 module 中实现一个支持并发的 buffer_manager，比如说 xv6 的如下代码在 arceos 中感觉找不到比较好的支持：
```c
static struct buf*
bget(uint dev, uint blockno)
{
  struct buf *b;

  acquire(&bcache.lock);

  // Is the block already cached?
  for(b = bcache.head.next; b != &bcache.head; b = b->next){
    if(b->dev == dev && b->blockno == blockno){
      b->refcnt++;
      release(&bcache.lock);
      acquiresleep(&b->lock); //？ sleep lock 需要 OS 的支持
      return b; //？ 如何返回一个上锁的结构的引用？可以返回一个 MutexGuard<T> 这样的对象吗？
    }
  }
  ...
}
//？ 综上，考虑到所需要的对于 OS 的支持，是否应该在 modules 中而非 crates 中实现 buffer_manager
//？ 但是这么做会导致 crates/ext2fs 的割裂比较严重（把 buffer_manager 实现成一个 trait？）
```

### 下一周计划
1. 在上述问题得到解决前，会先在 ext2fs 中实现一个可以从镜像中创建、读写文件系统的库，同时也先不考虑并发。
这部分剩下的工作有：
+ file_disk_manager: 模拟一个基于镜像文件的磁盘管理器
+ buffer_manager (no sync)
+ vfs: 先支持常见的文件操作：创建目录、创建文件，读写文件
2. 与助教沟通，找到一个协调并发问题的方案。


## 第 8 周
1. 在 `crates/ext2fs` 中仿照 `easy-fs` 写了文件系统的接口，目前支持：创建ext2文件系统镜像、从镜像中打开文件系统 `create_file`、`create_dir`、`link` 、`unlink` 等功能。

### 下一周计划
1. 将 ext2 集成到目前的 Arceos 的文件系统框架中；
2. 进一步完善 ext2 文件系统的功能，比如支持软链接、unlink 一个目录（目前只支持文件）、文件状态；
3. 调研带日志的文件系统的实现，准备把 ext2 升级至 ext3。

## 第 9 周
1. 在 ext2 中支持了更多的文件操作，比如：symlink、chown、chmod、truncate
2. 实现了一个 LRU 的 buffer_manager，一些细节如下：
+ 这里我们希望可以用一个容器来管理所有的缓存 (BTreeMap)，同时希望可以维护一个 LRU 队列，所以需要使用到
侵入式链表：
```rust
pub struct BlockCacheManager {
    device: Arc<dyn BlockDevice>,
    max_cache: usize,
    blocks: BTreeMap<usize,Arc<SpinMutex<BlockCache>>>, // 实际上管理 cache 的生命周期
    lru_head: InListNode<BlockCache, ManagerAccessBlockCache> // LRU 链表头
}

pub struct BlockCache {
    lru_head: InListNode<BlockCache, ManagerAccessBlockCache>,
    block_id: usize,
    modified: bool,
    valid: bool,
    cache: Box<[u8]>
}
```
侵入式链表本身和普通的 C 风格的链表类似，内部使用指针来维护连接关系，但是本身不包含数据
```rust
pub struct  ListNode<T> {
    prev: *mut ListNode<T>,
    next: *mut ListNode<T>,
    data: T
}

pub struct InListNode<T, A = ()> {
    node: ListNode<PhantomData<(T, A)>>,
}
```
这里，为了能够通过侵入式链表来访问数据，可以参考 C 语言中的技巧：即可以通过计算 `BlockCache` 中的 `lru_head` 域相对于这个结构体的偏移，然后对于 `&lru_head` 就可以减去这个偏移得到 `BlockCache` 的
起始地址，然后将其转为：`*mut BlockCache` 即可。当然，这么做需要我们自己确保操作的安全性。
```rust
// 该特征中的方法可以从一个侵入式链表的表头得到包含它的结构体的引用
pub trait ListAccess<A, B>: 'static {
    fn offset() -> usize;
    #[inline(always)]
    unsafe fn get(b: &B) -> &A {
        &*(b as *const B).cast::<u8>().sub(Self::offset()).cast()
    }
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(b: &mut B) -> &mut A {
        &mut *(b as *mut B).cast::<u8>().sub(Self::offset()).cast()
    }
}

#[macro_export]
macro_rules! inlist_access {
    ($vis: vis $name: ident, $T: ty, $field: ident) => {
        $vis struct $name {}
        impl $crate::list::access::ListAccess<$T, $crate::list::instrusive::InListNode<$T, Self>>
            for $name
        {
            #[inline(always)]
            fn offset() -> usize {
                $crate::offset_of!($T, $field)
            }
        }
    };
}

// 例子
crate::inlist_access!(AccessA, A, node);
struct A {
    _v1: usize,
    node: InListNode<A, AccessA>,
    _v2: usize,
}
```
+ 为了可以更加灵活使用锁，比如有些情况下需要再不持有锁的前提下对其内部进行读写（自信确保安全性），所以
重新实现了支持以上操作的 `SpinMutex`：
```rust
pub struct SpinMutex<T: ?Sized, S: MutexSupport> {
    lock: AtomicBool,
    _marker: PhantomData<S>,
    _not_send_sync: PhantomData<*const ()>,
    data: UnsafeCell<T>, // actual data
}

impl<T: ?Sized, S: MutexSupport> SpinMutex<T, S> {
    ...
    #[inline(always)]
    pub unsafe fn unsafe_get(&self) -> &T {
        &*self.data.get()
    }
    #[allow(clippy::mut_from_ref)]
    #[inline(always)]
    pub unsafe fn unsafe_get_mut(&self) -> &mut T {
        &mut *self.data.get()
    }
    ...
}
```
+ 最后简单介绍一下 LRU buffer_manager 的实现：

    - 在获取缓存块时，先在 blocks 中查找，如果没有则看目前的缓存块数是否达到上限，没有达到则新分配一块，否则就顺着 LRU 队列找到一个没有被其他进程持有的块牺牲掉，这个可以使用 `Arc` 的引用计数来判断。
    - 为了 LRU 策略可以正常执行，需要再不使用块后显示调用 `release_block`，它在没有进程使用该块时会将它重新插入到 LRU 队列的尾部
    ```rust
    pub fn release_block(&mut self, bac: Arc<SpinMutex<BlockCache>>) {
        if Arc::strong_count(&bac) == 2 {
            let ptr = unsafe { bac.unsafe_get_mut() };
            ptr.lru_head.pop_self();
            self.lru_head.push_prev(&mut ptr.lru_head);
            
        }
    }
    ```

3. 阅读了 ![ftl-os]( https://gitlab.eduxiji.net/DarkAngelEX/oskernel2022-ftlos) 中文件系统的实现，上述的侵入式链表就是参考这个实现。另外他们也实现了一个 `inode_manager` 用来管理 `inode` 的缓存，这样在读写文件的时候就不需要每次读磁盘才能知道要读写的块，同时也可以处理多个进程同时读写一个文件的情况。另外也可以更好地支持 Linux 对于文件操作的规范：即只有当文件的引用计数和被所有进程的引用计数都归零后才会回收对应的空间。

### 下一周计划
1. 实现 `inode_manager`，解决并发问题，并且加上缓存机制来提高速度（可选）
2. 为 ext2 提供更好的封装，目前的实现都是直接操作 `Inode`，可以进一步包装成 `Dir` 和 `File`，也可以支持路径搜索等更加复杂的操作
3. 进一步阅读 `ftl-os` 的实现以及 Linux 的相关资料，主要想要了解 vfs 如何设计（或许只是想知道？）