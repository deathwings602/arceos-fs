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