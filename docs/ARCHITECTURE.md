# 架构

## 1. Crate 依赖

```text
                 ┌────────────────────────────┐
                 │ game (bin)                 │
                 │ 窗口回调 / 资源装配 / 场景   │
                 └───────┬─────────┬──────────┘
                         │         │
          ┌──────────────▼──┐   ┌──▼─────────────┐
          │ minecraft       │   │ render         │
          │ 26.3 逻辑移植    │   │ -> ren_vulkan  │
          │ 无 GPU 依赖      │   │   ash Vulkan    │
          └───────┬─────────┘   └──▲─────────────┘
                  │                │
               ┌──▼────────┐   ┌──┴────────┐
               │ (std+image)│   │ window    │
               └───────────┘   │ (winit)   │
                               └───────────┘
```

- `minecraft`：Java 包路径一对一移植，只依赖 `std` 与 `image`（PNG 解码）。
- `render`/`ren_vulkan`：Vulkan 教程式后端，与 Minecraft 类型无关。
- `window`：winit 薄封装，向游戏层暴露自有 `WindowEvent`。
- `base`：数学库统一出口（glam）。
- `game`：唯一可执行程序，负责把三者粘起来。

## 2. `minecraft` 模块地图

```text
crates/minecraft/src/net/minecraft/
├── resources/identifier.rs          Identifier（26.3 的 ResourceLocation）
├── core/
│   ├── vec3i.rs / block_pos.rs / section_pos.rs
│   ├── direction.rs                 Direction / Axis / AxisDirection / Plane
│   ├── position.rs
│   └── registries/
│       ├── mod.rs                   Registry<T>
│       └── builtin_registries.rs    BuiltInRegistries.BLOCK
├── util/
│   ├── mth.rs                       数学函数
│   └── json.rs                      严格 JSON 解析器
├── server/packs/resources/          ResourceManager（目录资源包）
├── world/
│   ├── phys/{vec3,aabb}.rs
│   └── level/
│       ├── block/                   Block / Blocks / 状态 / 属性
│       ├── chunk/                   PalettedContainer / LevelChunkSection / LevelChunk
│       ├── level.rs                 Level 存储与方块读写
│       ├── levelgen/flat/           FlatLevelSource（占位生成）
│       └── chunk_pos.rs
└── client/
    ├── camera.rs
    ├── input/input.rs
    ├── player/local_player.rs       第一人称移动与 AABB 碰撞
    ├── multiplayer/client_level.rs  世界生成入口（ClientLevel）
    ├── resources/
    │   ├── language/                LanguageManager
    │   └── model/                   模型 JSON、父链合并、烘焙
    └── renderer/
        ├── block/dispatch/          blockstate variants 选择
        ├── texture/                 SpriteLoader / Stitcher / TextureAtlas
        └── chunk/                   SectionCompiler -> RenderMesh
```

## 3. 启动时序

`game::initgame` → `window::run` → winit `resumed`：

1. `Assets::load`
   - `find_client_root()` 定位 26.3 解包目录；
   - `ResourceManager::new(root)`；
   - `LanguageManager::load("en_us")`；
   - `BlockModelSet::load`
     - `BlockStateModelLoader` 读取全部 `blockstates/*.json`；
     - `ModelBakery` 解析 `models/**/*.json` 并沿 `parent` 合并；
     - 为每个 `BlockState` 烘焙 `BakedQuad`，汇总贴图标识符；
   - `SpriteLoader::load_atlas`：PNG → Stitcher → RGBA8 图集；
   - `ClientLevel::new_flat(4, 0)` 生成 9×9 区块；
   - `SectionCompiler::compile` 合成世界网格；
   - 编译 Slang 着色器。
2. `Render::new`：创建 Vulkan 实例/设备/交换链/管线，上传网格与图集。
3. `Scene::new(spawn)`：玩家与相机。
4. 捕获鼠标，进入重绘循环。

## 4. 每帧数据流

```text
winit 键鼠事件
   │  (window::WindowEvent)
   ▼
Game::window_event  ── 更新 keys / 旋转视角 / Esc 退出
   │
   ▼ RedrawRequested
Game: 计算 dt (<= 50ms)
   │
   ├─ Input { forward, jump, sprint, ... }
   ├─ Scene::update -> LocalPlayer::tick(level, input, dt)
   │                    └─ 轴扫掠碰撞，更新 position/velocity
   ├─ Scene::frame  -> SceneUniforms { model, view, projection }
   └─ Render::draw_frame(uniforms, delta_time)
         ├─ 等栅栏、获取交换链图像
         ├─ 录制计算粒子（可选绘制）
         ├─ 录制图形：绑定世界网格、图集、UBO
         ├─ 提交/呈现
```

MVP 全部由游戏层计算；渲染后端不依赖 glam，也不决定相机。

## 5. 世界存储

- 世界尺寸与 26.3 主世界一致：`min_y=-64`、`height=384`、24 个段。
- `Level` 持有 `HashMap<ChunkPos, LevelChunk>`。
- `LevelChunk` 有 24 个段槽位，空段为 `None`，非空段按需分配。
- `LevelChunkSection` 使用 `PalettedContainer`：
  - 单值模式（`data=None`）；
  - 调色板 + 4096 个 `u16` 索引（`data=Some`）；
  - 索引顺序与 Java 相同：`y << 8 | z << 4 | x`。
- 未加载区块与越界 Y 返回空气。
- `get_height_motion_blocking` 从最高点向下扫描可碰撞方块，返回其上方高度。

`FlatLevelSource` 目前固定生成：

```text
y=-64  bedrock
y=-63  dirt
y=-62  dirt
y=-61  grass_block
```

这是**渲染链路占位**，不是 26.3 的平地配置系统；完整实现需要
`FlatLevelGeneratorSettings`、生物群系、结构与装饰器。

## 6. 玩家与相机

`LocalPlayer`：

- 位置是脚底坐标，AABB 0.6×1.8，眼睛高度 1.62；
- 速度常量：行走 4.317、疾跑 5.612、潜行 1.31 方块/秒；
- 重力 32 方块/秒²，跳跃初速度 9.0，终端速度 -60；
- 每帧按 `dt` 积分，按 Y→X→Z 顺序以 0.05 方块步长扫掠；
- 碰撞体暂按整方块处理，未接入 26.3 的 `VoxelShape`。

`Camera` 保存位置与 yaw/pitch：

- yaw=0 朝南(+Z)，yaw 增大向西；
- pitch>0 向下；
- 视线向量 `(-sin(yaw)cos(pitch), -sin(pitch), cos(yaw)cos(pitch))`。

视图矩阵 `look_to_mat4(eye, direction, +Y)`，投影使用
`glam::camera::rh::proj::vulkan::perspective(70°, aspect, 0.05, 512)`。

## 7. Vulkan 后端

- 图形管线：三角形列表、深度测试 `LESS`、MSAA resolve、动态 viewport/scissor。
- 顶点格式：`position: vec3 f32`、`color: vec3 f32`、`uv: vec2 f32`（32 字节）。
- 着色器：Slang 运行期编译；片元 = 纹理采样 × 顶点色。
- 图集采样：`NEAREST`、`CLAMP_TO_EDGE`、不生成 mipmap，避免像素画糊化与
  跨 sprite 渗色。
- 背面剔除当前为 `NONE`：
  - 方块面已在 CPU 侧按邻居剔除；
  - 关闭 CPU 绕序与 Vulkan 正面定义的耦合，待渲染回归测试稳定后再开 `BACK`。
- 清屏色：`(0.4706, 0.6549, 1.0)`，Minecraft 默认天空色。
- 教程的 compute 粒子系统保留在引擎里，但 Minecraft 模式通过
  `Render::set_particles_enabled(false)` 关闭绘制。

## 8. 资源与图集

详见 [ASSETS.md](ASSETS.md)。关键不变量：

- `ResourceManager` 的根目录包含 `assets/` 与 `data/`；
- 贴图路径 = `assets/<namespace>/textures/<path>.png`；
- 模型路径 = `assets/<namespace>/models/<path>.json`；
- blockstate 路径 = `assets/<namespace>/blockstates/<path>.json`；
- 图集当前用固定单元格行列排布，sprite UV 以实际像素尺寸计算。

## 9. 当前明确的缺口

- 无 GUI/字体/准星/物品栏/菜单；
- 无音效、实体、方块实体、光照、刻调度、方块更新、流体；
- 无网络、多人、存档、NBT；
- 无区块异步编译与按需加载（网格只在启动时构建一次）；
- 无透明/裁剪（cutout/translucent）分层渲染；
- 无生物群系着色，草色固定为 `(0.5686, 0.7412, 0.3490)`；
- 无动画贴图（`.mcmeta`）与资源包叠加。
