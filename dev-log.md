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
