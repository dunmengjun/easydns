# Easydns

1. 此项目的目的时做一个足够简单高效的dns透传优选和屏蔽广告服务，类似smartdns, 但不会有 smartdns那么多的功能，只会提供核心关键的功能以保持简单高效。
2. 此项目编译到 [Padavan](https://github.com/hanwckf/rt-n56u) 算是rust依赖mipsel toolchain交叉编译到Padavan的模板
3. 百分之百用Rust语言开发，同时尽可能引用少的外部包以保证最终程序的体积。

### 功能完成度

假定的场景是：dns请求中的问题只有一个, 多个的情况属于容错，事实上现在dns请求带多个问题的场景基本已经没有了，家用是碰不到的。

- [x] A(ipv4)记录的透传并过整条链路(缓存和优选)
- [ ] AAAA(ipv6)记录的透传并过整条链路(缓存和优选)
- [x] 所有其他记录容错(直接给到上游服务器去返回，不过缓存和优选)
- [x] 缓存(根据ttl时间, 最大条数限制)
- [x] 多线程(tokio实现)
- [x] 缓存持久化(存本地文件，下次启动时load)
- [x] 域名过滤(过滤广告，返回soa)
  - [x] 返回soa 
  - [ ] 从文件中读
  - [ ] 从网址中读
- [ ] 上游服务器组管理
- [x] dns优选
    - [x] 上游dns服务器优选
    - [x] 返回的IP地址优选
        - [x] ping协议 (需要root权限或者给程序设置cap_net_raw)
        - [ ] tcp协议
            - [ ] 80端口网页中的域名预加载到缓存
- [ ] 参数配置化(统一的配置文件)
- [ ] 标准日志
- [ ] github action自动编译
- [ ] 测试(单元测试，稳定性测试，性能测试)
- [ ] 其他平台的编译(linux, windows, macos)

### 必须的依赖:

1. rust环境
2. mipsel
   toolchain [下载地址](https://github.com/hanwckf/padavan-toolchain/releases/download/v1.1/mipsel-linux-uclibc.tar.xz)
   下完解压即可
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

### 常用命令

设置cap_net_raw

```shell
sudo setcap cap_net_raw=eip {程序名称}
```