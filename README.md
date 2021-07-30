# easydns
此项目编译到 [Padavan](https://github.com/hanwckf/rt-n56u)
算是rust依赖mipsel toolchain交叉编译到Padavan的模板

### 必须的依赖:
1. rust环境
2. mipsel toolchain [下载地址](https://github.com/hanwckf/padavan-toolchain/releases/download/v1.1/mipsel-linux-uclibc.tar.xz) 下完解压即可
3. cargo exec [仓库地址](https://github.com/dunmengjun/cargo-exec) clone完成后
```shell
cargo install --path .
```
### 可选依赖:
4. 在本机上测试和运行依赖于 qemu-mipsel 5.2.0, 单纯编译不需要。

## 运行项目

### 自动rust版本同步
```shell
rustup show
```

### 编译
```shell
cargo exec build
```

### 运行
```shell
cargo exec run
```
### 单元测试
```shell
cargo exec test
```
### 发布
```shell
cargo exec release
```

### 调试
调试依赖
1. qemu-mipsel 5.2.0 
2. mipsel toolchain
3. mipsel gdb ([安装教程](https://blog.csdn.net/zqj6893/article/details/84662579))

有了这三个工具调试就很简单了

#### gdb server 启动
```shell
qemu-mipsel -L /path to mipsel toolchain/mipsel-linux-uclibc/sysroot -g 1234 ./target/mipsel-unknown-linux-uclibc/debug/easydns
```
#### gdb client连接
```shell
$ ./mipsel-linux-gdb
GNU gdb (GDB) 9.2
Copyright (C) 2020 Free Software Foundation, Inc.
License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.
Type "show copying" and "show warranty" for details.
This GDB was configured as "--host=x86_64-pc-linux-gnu --target=mipsel-linux".
Type "show configuration" for configuration details.
For bug reporting instructions, please see:
<http://www.gnu.org/software/gdb/bugs/>.
Find the GDB manual and other documentation resources online at:
    <http://www.gnu.org/software/gdb/documentation/>.

For help, type "help".
Type "apropos word" to search for commands related to "word".
(gdb) target remote localhost:1234
Remote debugging using localhost:1234
```
在clion这种IDE上可以以Remote GDB Server来配置，核心就是上面两个命令