# 铜壶镇 · 第一股泉水

旅店门口落了一颗李子，镇上的泉却一早就断了。北水门底下有一条旧回水廊：西边是修理工的工具箱，东边的铜架上还挂着手轮。去的时候有两条路，回来时得留点力气。

Bevy 游戏实图：[泉市与可点击的石路](reference/visual/copper-square-game.webp) · [地宫交战](reference/visual/spring-cistern-game.webp) · [挑选器物](reference/visual/spring-journal-game.webp) · [归还手轮后的平视](reference/visual/spring-wheel-game.webp)。

## 启动

在本机 Wayland 桌面、仓库根目录运行：

```sh
cargo run --locked
```

需要 Rust 1.89+ 和硬件 Vulkan 驱动。工程锁定 Bevy 0.16.1，只启用 Wayland。普通 `cargo run` 默认使用官方 `bevy/dynamic_linking`，依赖已开启优化；引擎动态库和可执行文件都在 `target/`。游戏素材、字体均在仓库内。

发布构建：`cargo run --locked --release --no-default-features`，关闭开发动态链接。目标为原生 2560×1440、FIFO 垂直同步；默认尝试 100 FPS，持续达不到则切到 50。`-- --fps 50` 可从启动时锁定半刷新档。

本机发布实测：Ryzen 5 5500、RX 6500 XT、RADV / Mesa 26.1.8、Wayfire，原生 2560×1440 / 100 Hz，Vulkan、FIFO。窗口保持前台，默认光照、像素层与景深。自动切到 50 档并预热后，各采连续 600 帧（约 12 秒）；采样期间未截屏。帧间隔包含限帧等待，不是 GPU 单个阶段的耗时。

| 场景 | 平均 FPS | 中位帧时 | P95 | P99 |
|---|---:|---:|---:|---:|
| 已修复的镇泉，十字街俯视并显示斜向鼠标路线 | 50.00 | 20.000 ms | 20.003 ms | 20.016 ms |
| 回水廊东翼，萤石与灯同时可见，飞蛾锁定下一拍 | 50.00 | 20.000 ms | 20.003 ms | 20.005 ms |
| 镇街实际走动、连续推到平视再拉回 | 50.00 | 20.000 ms | 20.003 ms | 20.005 ms |

## 游玩

- **鼠标**：石板的格缝常驻；悬停看脚点、路线和物件。左键点地面走过去，点药师、箱子或水门会走到旁边互动。遇敌或遇到危险预警会停步；相邻时点敌人攻击。右键取消。
- **八向移动**：WASD 或方向键可组合（例如 W+D 直接东北走一格），小键盘的八个方向键也可单按。直走、斜走都花一拍；斜线两旁的格子必须都能通过。鼠标预览和实际行走遵守同一条规则。E／Enter 互动，空格或小键盘 5 等候。滚轮／+、− 从正北俯视推到平视；Home 复位。V 调查，J／Tab 翻手记。
- **镇上**：熟李子可以留给早餐、扔给小魔物，也可以找泉边药师换一瓶药。T 选投掷格；回店吃完还能继续今天的冒险。
- **井下**：看红格和对手的姿势再行动。Q 护卫，挡住敌击并反击相邻敌人；地上的蒸汽喷口要让开。F 用器物技，护卫或等候后可再用；R 喝药回复 4 点生命。圆盾适合守住窄路，钩索可以隔格拉住对手。
- **回程**：从南侧楼梯上来，散落的铜片才算存下；每存够四枚，生命上限加一（最多加六）。带回手轮后，点东侧泉台把它装好。修好的泉会一直流，再下井或再住一晚都可以。
- **显示与退出**：H 收起 HUD。F7 在同机位切换原生 PBR 与世界像素墨层，文字与纸页始终按屏幕像素绘制。F12 使用 Bevy 引擎截图到 `captures/`；Ctrl+Q 退出。

手记保存在 `${XDG_DATA_HOME:-$HOME/.local/share}/wanderstead/plum-mornings.json`，兼容早期只有早餐记录的版本。镇上存好的铜片、器物、药囊、餐点与修复进度保留。在井下倒下或退出，没带上台阶的手轮与散币丢失。`cargo run --locked -- --fresh` 可开临时新局，不读写原存档。

## 设计与制作

[设计笔记 §19](game-design-notes.md#first-outing)记录这场短冒险的取舍。[RPG Maker 地图参考](reference/roguelike-worldbuilding-research.md#rpg-maker-maps)分析《Ara Fell》原版和《Grimm’s Hollow》的实际画面。[镇场概念](reference/visual/copper-square-study.webp)和[回水廊概念](reference/visual/spring-cistern-study.webp)是生成的美术参考；页首四张是游戏窗口截图。

`assets/maps/*.grid` 同时定义地板、阻挡、交互位置和小地图。3D 源文件及原画在 `art/lane/`、`art/cistern/`、`art/market/`；人物、界面和器皿在 `art/figures/`、`art/book/`、`art/inks/`。`tools/` 保留 `blender-4.5` / `bpy` 与图集加工脚本。主游戏不依赖 `temp/` 的参考原型。

各区的漫反射和多级镜面探针由 `python3 tools/make_spring_probes.py` 离线生成；`tools/bake_sky_shelter.py` 用 Blender 从最终 GLB 的真实建筑烘出四个高度的天空可见度。树冠和岸石从 `tools/make_verge_sprites.py` 分离，再交给路面和布景烘焙。`tools/make_figures.py` 导出八向走姿和 64 帧人物轮廓。[前景遮挡实测：同一机位，左关右开](reference/visual/spring-sight-cut.webp)。鼠标射线也能穿过淡出的中心。

开发：`--qa /绝对路径/qa.sock` 开启本地 Unix datagram 接口。`state`、`expedition`、`project X Z [高度]`、`pick X Y` 查询实况；`pointer X Y`、`mouse down|up`、`key kp9 down|up`、`scroll 3` 走实际输入链。`sight [on|off]` 仅用于在原机位比较前景裁切。`capture NAME` 读回窗口，`measure` / `stats` 采集连续 600 帧，`exit` 退出。正常启动不监听 socket。可选 `WANDERSTEAD_GPU_PROFILE=1` 开启 Bevy 渲染阶段诊断；`passes` 返回 GPU 毫秒均值。