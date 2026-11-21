# 移植进度

> 最后更新：2026-11-21（里程碑 M0 完成）
>
> 规则：只有同时满足 [PORTING.md](PORTING.md) 第 12 节“完成定义”的类才标记
> 为“完整”；其余标记“部分”并写明缺口。**不要**把能编译当成已移植。

## 1. 规模基线

| 范围 | Java 文件数 |
| --- | ---: |
| 全部 | 7301 |
| `net/minecraft/world` | 2715 |
| `net/minecraft/client` | 1891 |
| `net/minecraft/util` | 735 |
| `net/minecraft/client/renderer` | 715 |
| `net/minecraft/server` | 432 |
| `net/minecraft/network` | 416 |
| `com/mojang` | 379 |
| `net/minecraft/world/level/block`（顶层类） | 330 |
| `net/minecraft/core` | 123 |

当前 Rust 移植层：88 个 `.rs` 文件（含 `mod.rs`），约 45 个有实际行为的类。
方块注册 7 / 330 个方块类。

## 2. 已移植类

状态：✅ 完整（按当前范围） · 🟡 部分 · 🟠 占位/仅类型

### 基础与资源

| Java | Rust | 状态 | 说明 |
| --- | --- | --- | --- |
| `net/minecraft/resources/Identifier` | `resources/identifier.rs` | ✅ | 字符集、默认命名空间、越权检查、语言键 |
| `net/minecraft/IdentifierException` | `net/minecraft/mod.rs` | ✅ | 改为 `Result` 错误类型 |
| `net/minecraft/core/Vec3i` | `core/vec3i.rs` | ✅ | 含比较顺序、方向偏移 |
| `net/minecraft/core/BlockPos` | `core/block_pos.rs` | ✅ | 64 位打包布局逐位一致 |
| `net/minecraft/core/SectionPos` | `core/section_pos.rs` | ✅ | 22/20/22 位打包 |
| `net/minecraft/core/Direction` | `core/direction.rs` | ✅ | 枚举顺序、data3d/2d、旋转关系 |
| `net/minecraft/core/Direction$Axis` | `core/direction.rs`（`Axis`） | ✅ | 以独立类型表达嵌套枚举 |
| `net/minecraft/core/Position` | `core/position.rs` | ✅ | trait |
| `net/minecraft/core/Registry` | `core/registries/mod.rs` | 🟡 | 仅 ID/名字查询；无标签、Holder、同步 |
| `net/minecraft/core/registries/BuiltInRegistries` | `core/registries/builtin_registries.rs` | 🟡 | 仅 BLOCK |
| `net/minecraft/util/Mth` | `util/mth.rs` | 🟡 | 已移植数学/打包所需函数 |
| `net/minecraft/util/StrictJsonParser` 等价物 | `util/json.rs` | ✅ | 自研严格 JSON，替代 Gson |
| `net/minecraft/server/packs/resources/ResourceManager` | `server/packs/resources/mod.rs` | 🟡 | 单目录资源包；无 zip/多包/重载 |

### 世界与方块

| Java | Rust | 状态 | 说明 |
| --- | --- | --- | --- |
| `world/phys/Vec3` | `world/phys/vec3.rs` | ✅ | 常用运算完整 |
| `world/phys/AABB` | `world/phys/aabb.rs` | 🟡 | 碰撞盒核心；无射线/遮挡形状 |
| `world/level/ChunkPos` | `world/level/chunk_pos.rs` | ✅ | 打包、区块边界 |
| `world/level/block/Block` | `world/level/block/block.rs` | 🟡 | 注册/默认状态/渲染形状/碰撞；行为方法未移植 |
| `world/level/block/Blocks` | `world/level/block/blocks.rs` | 🟡 | 仅 AIR/STONE/GRASS_BLOCK/DIRT/COBBLESTONE/BEDROCK/OAK_PLANKS |
| `world/level/block/AirBlock` | `air_block.rs` | 🟡 | 渲染形状正确；其余继承行为未移植 |
| `world/level/block/SnowyBlock` | `snowy_block.rs` | 🟡 | `SNOWY` 属性正确；随机刻未移植 |
| `world/level/block/SpreadingSnowyBlock` | `spreading_snowy_block.rs` | 🟠 | 类型/状态；传播行为待光照 |
| `world/level/block/GrassBlock` | `grass_block.rs` | 🟠 | 状态正确；骨粉/传播未移植 |
| `block/state/BlockBehaviour$Properties` | `state/block_behaviour.rs` | 🟡 | 常用建造者字段；部分链式方法为 no-op |
| `block/state/BlockState` | `state/block_state.rs` | 🟡 | 身份/属性/形状查询；无全局状态表 ID |
| `block/state/StateDefinition` | `state/state_definition.rs` | 🟡 | 笛卡尔展开/选择器；无邻居缓存/编解码 |
| `block/state/properties/Property` | `state/properties/property.rs` | 🟡 | 布尔/整数；无枚举属性 |
| `world/level/chunk/PalettedContainer` | `chunk/paletted_container.rs` | 🟡 | 单值 + u16 调色板；无 global/线程包装 |
| `world/level/chunk/LevelChunkSection` | `chunk/level_chunk_section.rs` | 🟡 | 方块状态与计数；无光照/高度图 |
| `world/level/chunk/LevelChunk` | `chunk/level_chunk.rs` | 🟡 | 段存储；无实体/方块实体/刻 |
| `world/level/Level` | `level/level.rs` | 🟡 | 区块存取；无更新传播/刻/光照 |
| `world/level/levelgen/flat/FlatLevelSource` | `levelgen/flat/` | 🟠 | 固定四层平地，非完整生成器 |
| `client/multiplayer/ClientLevel` | `multiplayer/client_level.rs` | 🟡 | 生成/出生点；无多人/实体管理 |

### 客户端渲染与输入

| Java | Rust | 状态 | 说明 |
| --- | --- | --- | --- |
| `client/Camera` | `camera.rs` | 🟡 | 位置/朝向/基向量；无视锥/雾/bobbing |
| `client/input/Input` | `input/input.rs` | ✅ | 输入状态与移动轴 |
| `client/player/LocalPlayer` | `player/local_player.rs` | 🟡 | 移动/跳跃/AABB 碰撞；非 20 TPS 精确物理 |
| `client/resources/language/LanguageManager` | `resources/language/` | 🟡 | 单语言加载；无回退链/重载 |
| `client/resources/model/BlockStateModelLoader` | `resources/model/block_state_model_loader.rs` | 🟡 | 同步单包；无 multipart/多包 |
| `client/resources/model/ModelManager` / `ModelBakery` | `resources/model/model_manager.rs` | 🟡 | 父链/烘焙；无物品模型/特殊渲染器 |
| `.../cuboid/CuboidModel` | `.../cuboid/cuboid_model.rs` | 🟡 | 元素/贴图槽/父；display 忽略 |
| `.../cuboid/CuboidModelElement` | `.../cuboid/cuboid_model_element.rs` | 🟡 | 解析完整；元素旋转未应用 |
| `.../cuboid/CuboidFace` | `.../cuboid/cuboid_face.rs` | ✅ | UV/旋转/tint/cull |
| `.../cuboid/CuboidRotation` | `.../cuboid/cuboid_rotation.rs` | 🟡 | 仅解析 |
| `.../cuboid/UnbakedCuboidGeometry` | `.../cuboid/cuboid_model.rs` | 🟡 | 面轴剔除；元素旋转未应用 |
| `.../cuboid/FaceBakery` | `.../cuboid/face_bakery.rs` | 🟡 | 顶点/UV/默认 UV/y 变体旋转 |
| `.../geometry/BakedQuad` | `.../geometry/baked_quad.rs` | 🟡 | 未压缩顶点/UV；无材质信息互化 |
| `.../sprite/TextureSlots` | `.../sprite/texture_slots.rs` | 🟡 | 合并/别名解析；无 Data 编解码 |
| `renderer/block/dispatch/BlockStateModelDispatcher` | `renderer/block/dispatch/` | 🟡 | variants；无 multipart |
| `renderer/block/dispatch/Variant` | 同上 | 🟡 | 字段完整；权重随机未实现 |
| `renderer/texture/SpriteContents` | `renderer/texture/sprite_contents.rs` | 🟡 | PNG/缺失贴图；无动画 |
| `renderer/texture/SpriteLoader` | `renderer/texture/sprite_loader.rs` | 🟡 | 单包同步加载 |
| `renderer/texture/Stitcher` | `renderer/texture/stitcher.rs` | 🟠 | 固定单元格布局，未实现原装箱算法 |
| `renderer/texture/TextureAtlas` | `renderer/texture/texture_atlas.rs` | 🟡 | CPU 合成/UV；无 GPU 管理 |
| `renderer/texture/TextureAtlasSprite` | `renderer/texture/texture_atlas_sprite.rs` | ✅ | UV 映射 |
| `renderer/chunk/SectionCompiler` | `renderer/chunk/section_compiler.rs` | 🟡 | 单层不透明网格；无分层/异步 |
| （新增边界类型） | `renderer/chunk/render_mesh.rs` | — | 与 GPU 解耦的顶点/索引 |

### 引擎侧（非 Java 对应）

| 模块 | 状态 | 说明 |
| --- | --- | --- |
| `ren_vulkan` | ✅ 可用 | MSAA/深度/纹理/图集采样/粒子开关 |
| `window` | ✅ 可用 | 窗口、键鼠、原始鼠标位移、光标捕获 |
| `game` | ✅ 可用 | 资源装配、场景、输入循环 |

## 3. 资源格式支持矩阵

| 格式 | 状态 |
| --- | --- |
| `blockstates/*.json` variants | 🟡 支持；multipart 未实现 |
| 加权变体 | 🟡 取第一个，未随机 |
| 模型 `parent` 链 | ✅ |
| 模型 `elements`/`faces`/`uv`/`cullface`/`tintindex` | ✅ |
| 面 UV 旋转 0/90/180/270 | ✅ |
| 变体 `y` 旋转 | ✅ |
| 变体 `x` 旋转 | ❌ 明确报错 |
| 元素 `rotation` | ❌ 解析后报错 |
| `uvlock` | ❌ 忽略 |
| `ambientocclusion` | 🟡 解析，未参与顶点 AO |
| 贴图 `.mcmeta` 动画 | ❌ |
| PNG | ✅（`image`，仅 png feature） |
| `en_us.json` | ✅ |
| NBT（`.nbt`） | ❌ |
| `data/` 数据包 | ❌ |
| zip/jar 资源包 | ❌ |
| 26.3 原版 shader/pipeline JSON | ❌ 使用自有 Slang 管线 |

## 4. 已知渲染偏差

- 草色固定，不随生物群系变化；
- 无光照/天空光/方块光，明暗只来自固定面向系数；
- 无透明/裁剪分层，树叶、玻璃、水等尚不可用（这些方块也尚未注册）；
- 无 mipmap、无各向异性；远处高频纹理会有闪烁；
- 图集无 padding，理论上有跨 sprite 渗色风险；
- 背面剔除关闭；
- 无雾、无天空盒/太阳/云；
- 无 HUD、准星、字体、物品栏。

## 5. 路线图

### M0 — 可运行垂直切片（已完成，2026-11-21）

真实资源 → 模型 → 图集 → 网格 → Vulkan；第一人称移动与碰撞。

### M1 — 资源/模型完整性

- `multipart` 与条件模型；
- 加权变体随机；
- 变体 `x` 旋转、元素 `rotation`、`uvlock`；
- 动画贴图与 `.mcmeta`；
- 图集 padding、mip 策略与渲染回归测试；
- zip/jar 资源包与多包优先级；
- 扩展方块类到常用完整立方体方块（石类、羊毛、木板等），
  验证通用模型路径。

### M2 — 世界与区块

- 按方块类型区分 solid/cutout/translucent 图层；
- 区块脏标记与同步/异步重建；
- 视锥剔除、按区块绘制；
- 高度图、天空光/方块光、光照传播；
- 生物群系与草地/树叶着色；
- 流体、方块更新、刻调度；
- 完整 `FlatLevelGeneratorSettings`。

### M3 — 交互与实体

- `VoxelShape` 碰撞/选择形状；
- 射线拾取、破坏/放置方块；
- 掉落物、物品栏、方块实体；
- 实体系统与基础生物；
- 玩家 20 TPS 固定刻物理。

### M4 — UI 与选项

- 2D UI 渲染管线与字体（`assets/minecraft/font`）；
- 标题界面、暂停菜单、选项界面；
- HUD：准星、快捷栏、F3 调试信息；
- `Options` 持久化与键位绑定。

### M5 — 声音

- `sounds.json` 与音频设备；
- 环境音/方块音/音乐。

### M6 — 网络与多人

- 协议 777 的 `ByteBuf`/`StreamCodec` 对应物；
- 客户端连接、区块/实体/方块更新包；
- 集成服务器与本地世界目录。

### M7 — 数据驱动内容与完整世界生成

- NBT、区域文件、数据包；
- 配方、进度、战利品表、标签；
- 命令系统；
- 结构/特征/生物群系/维度。

## 6. 每次会话的更新要求

- 新增/修改类后更新第 2 节的状态与说明；
- 新增偏差时同步更新 [DECISIONS.md](DECISIONS.md)；
- 里程碑完成时更新顶部日期与路线图勾选状态；
- 行为无法验证时写明“未验证”，不要标 ✅。
