# HD-2D 场景预览

当前保留一处室外街区与一处室内遗迹，以及人物、环境动画、光影、像素材质、镜头和整件前景遮挡处理。场景名称、角色身份、故事和玩法均待重新设计。

当前实机：[室外街区](reference/visual/scene-town.webp) · [室内遗迹](reference/visual/scene-cistern.webp)。

## 运行

在当前 Wayland 桌面、仓库根目录执行：

```sh
cargo run --locked
```

Bevy 固定为 0.16.1，需要 Rust 1.89+ 和硬件 Vulkan 驱动。普通开发运行启用官方动态链接，产物留在 `target/`。发布构建：

```sh
cargo run --locked --release --no-default-features
```

目标为原生 2560×1440、FIFO 垂直同步；优先 100 FPS，持续达不到则切至 50。`-- --fps 50` 可从启动锁定半刷新档。截图和性能验证在真实 Wayland 窗口内进行。

## 查看场景

- **走动**：WASD／方向键可组合，小键盘八方向可单按。斜步需要两个肩格都能通过。
- **鼠标**：悬停显示格子和可达路线，左键点地面行走，右键或 Esc 取消。人物与道具只作为场景陈设。
- **镜头**：滚轮、`+`／`−` 连续推近或拉远，Home 恢复默认正北俯视。最近为真正平视。
- **场景**：F6 在室外街区与室内遗迹之间切换。喷泉持续流水，生物只播放待机动作。
- **显示**：F7 对比像素处理前后；F12 使用 Bevy 保存截图到 `captures/`。
- **退出**：Ctrl+Q 或关闭窗口。

运行状态只含当前场景、主角位置和朝向。旧存档保持原样，当前程序不读取或写入它。

## 保留的制作管线

`assets/maps/*.grid` 定义场景地格与通行。美术源文件保留在 `art/`，生产素材在 `assets/`，建模和纹理脚本在 `tools/`。旧 UI 原画、字体及其许可也保留为素材资料，当前构建没有启用 UI、文字或 2D Sprite 插件。

昼间光色、天空补光、曝光、泛光和景深共用 `assets/inks/market-morning.json`。室外场景制作：

```sh
blender-4.5 -b --python-exit-code 1 --python tools/make_lane.py -- --export-only
blender-4.5 -b --python-exit-code 1 --python tools/bake_market_dressing.py
blender-4.5 -b --python-exit-code 1 --python tools/bake_sky_shelter.py -- --town-only
python3 tools/make_spring_probes.py
```

更新地宫时先运行 `tools/make_cistern.py -- --export-only`，再运行布景脚本。导出器把完整物件编号写入第二组 UV；附属装饰、灯芯、水面和人物使用同一遮挡编号。镜头至主角之间的所有前景物体同时整件隐藏，鼠标可选到露出的道路，碰撞与投影保持原样。

`tools/make_figures.py` 加工旅人走姿，`tools/make_trade_figures.py` 加工镇民生活动作。原始素材与许可证随项目保留。

## 本地检查接口

`--qa /绝对路径/qa.sock` 按需启用 Unix datagram 接口，正常运行不监听 socket。

- `state`：场景、位置、朝向、移动序号、选格、路线、镜头、窗口与帧率档位。
- `project X Z [高度]`、`pick X Y`：坐标投影与实际可见表面拾取。
- `pointer X Y`、`mouse down|up`、`key kp9 down|up`、`key f6 down|up`、`scroll 3`：走实际输入处理链。
- `sight [on|off]`：读取隐藏模型，或在同一机位比较遮挡效果。
- `optics [air|bloom|dof on|off]`：读取或切换光学层。
- `capture NAME`：引擎内截图；`measure` / `stats`：采集连续 600 帧；`exit`：退出。

设计笔记与 `reference/` 调查资料保留。§19 记录场景和渲染制作约定，后续名称、故事与玩法由项目作者确定。