# 旅店外的李子 · Wanderstead

一个可独立玩完的 HD-2D 日式西幻小样。铜壶旅店已经摆好早餐。门边有一颗李子；林缘的果树下，还有个跟你一样爱吃李子的小家伙。回餐桌时，怎么吃这一碗由你决定。

Bevy 窗口实图：[默认俯视](reference/visual/first-outing-overhead.webp) · [同一地点放大到平视](reference/visual/first-outing-eye-level.webp) · [门廊早餐与羊皮纸菜单](reference/visual/first-outing-breakfast.webp)。

## 启动

在本机 Wayland 桌面、仓库根目录运行：

```sh
cargo run --locked
```

需要 Rust 1.89+ 和带硬件驱动的 Vulkan 环境。根工程使用 Bevy 0.16.1，只启用 Wayland 窗口后端。默认开发构建启用 `bevy/dynamic_linking`，依赖以优化模式构建；第一次会编译引擎，之后改游戏代码可复用 `bevy_dylib`。二进制、动态库均在 `target/`。素材和字体已随项目提供。

发布档实测命令为 `cargo run --release --no-default-features --locked`（关闭 Bevy 开发动链）。目标桌面 2560×1440，FIFO 垂直同步，优先 100 FPS；持续达不到时降到 50。`-- --fps 50` 可直接指定半刷新档。

在 Ryzen 5 5500 / RX 6500 XT（RADV，Mesa 26.1.8）、100 Hz 原生 Wayland 桌面，发布版以 2560×1440 / FIFO、默认光效运行。林地俯视在前台预热后、不截屏的 600 帧平均 100.00 FPS，帧时中位 10.00 ms、P95 10.22 ms。手动 50 档另取 600 帧，帧时中位 20.00 ms。

## 游玩

- **WASD／方向键**走一格，按住连续走。**E／Enter**拾取、摇树、打招呼或坐下；**空格**等一回合。也可点邻格物件互动，点远处沿可走的路走近。
- **T** 选择抛果落点，三步以内可以越过低墙。方向键或鼠标选格，E 确认、Esc 取消。小魔物听见落果会走过去，抵达后要停一拍才能吞下。你不行动，它也不抢。
- **V** 察看；**J／Tab** 翻旅行手记。看书、观察和变焦均不消耗回合。书页和早餐可以用左右键或页面按钮操作。
- **滚轮／+、−** 连续缩放。默认是北向高俯视；放大时靠近、降到人物眼高，**Home** 回到默认距离。
- 回门廊餐桌，选择放几颗李子，也可以吃原味。吃完可留下散步或住到明天；第二天的落果位置会变。
- **H** 隐藏探索 HUD，**F7** 切换世界材质的像素光照，**F12** 用 Bevy 把窗口截图保存到 `captures/`。**Ctrl+Q** 或关闭窗口退出。

每次端上早餐会记录当天用果、留下的果子和小魔物所得。重开从最近一餐的下一天开始；未吃完早餐的半途行程不计入手记。Linux 存档在 `${XDG_DATA_HOME:-$HOME/.local/share}/wanderstead/plum-mornings.json`。`cargo run --locked -- --fresh` 从第一天试玩，不读写已有手记。

## 设计与制作

[设计笔记 §19](game-design-notes.md#first-outing)包含取景、纸面走法和本次取舍。`art/lane/`、`art/figures/`、`art/inks/`、`art/book/` 与 `tools/` 保留美术源和再制作依据。`temp/` 是本机另外提供的神殿参考原型；镜头与队伍的修正随仓库记录，神殿运行仍使用原有参考资源。主游戏不依赖 `temp/`。

开发用 `--qa /绝对路径/qa.sock` 开启本地 Unix datagram 接口；正常启动不会创建它。`state`、`key e down` / `key e up`、`scroll 3`、`pointer X Y`、`mouse down` / `mouse up`、`measure` / `stats`、`capture NAME`、`exit` 用于原生桌面试玩。客户端绑定具名 socket 接收 JSON；截图答复在引擎读回并写入 PNG 后返回。