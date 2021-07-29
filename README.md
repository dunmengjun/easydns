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

##运行项目

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