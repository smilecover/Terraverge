# 架构决策记录（ADR）

格式：背景 → 决策 → 后果。所有与 Java 源码的**有意偏差**都必须出现在这里。

---

## ADR-001：使用单个 `minecraft` crate 镜像 Java 包路径

**背景**：Java 允许包之间循环引用（`Block` ↔ `BlockState`、`ClientLevel` ↔
`Minecraft`）。若按层拆成多个 crate，会频繁出现无法打破的依赖环。

**决策**：`crates/minecraft` 一个 crate，内部模块路径严格镜像
`net.minecraft.*`。需要 GPU 的边界放在 `game`/`render`，不反向依赖。

**后果**：

- 可以像 Java 一样自由跨包引用；一次编译覆盖整个移植层；
- 单 crate 文件数未来会很大，必要时按“叶子包”拆 crate，但接口仍保持
  `net::minecraft` 路径的再导出；
- `minecraft` 不依赖 ash/winit/glam，保持可测试与可替换。

---

## ADR-002：尽量少依赖第三方库；JSON 自研，PNG 复用 `image`

**背景**：需求明确要求“尽量不依赖其他第三方库”。现有工作区已经使用
`ash`、`ash-window`、`winit`、`glam`、`image`、`shader-slang`。

**决策**：

- 新增依赖只允许 `image`（仅 `png` feature），用于 26.3 贴图解码；
- JSON 解析器自研（`util/json.rs`），不引入 serde/gson 等价物；
- NBT、zip、音频等未来能力优先自研；确需引入时先写 ADR；
- 数学使用工作区已有的 `glam`，但只在 `game`/`base` 边界使用。

**后果**：资源解析行为可控、可测；代价是需要维护解析器，并逐项补齐 Gson
的边界行为。当前拒绝注释/尾随逗号，符合 26.3 的严格解析路径。

---

## ADR-003：暂时直接读取解包目录，不读 zip/jar

**背景**：`minecraft-26.3-client-src` 已解包，含 `assets/` 与 `data/`；
实现 zip 资源包需要 deflate 依赖或自研。

**决策**：`ResourceManager` 只支持目录资源包，根目录包含 `assets/`、`data/`。
查找顺序：环境变量 → 工作区默认路径 → 当前目录祖先。

**后果**：开发期零拷贝、可直接查看资源；发布时需补 zip 支持与多包优先级。
路径错误时启动会给出明确错误并退出，不会用占位资源静默继续。

---

## ADR-004：`BlockState` 用 `(block_id, state_index)` 表示身份

**背景**：Java `BlockState` 是全局状态表中的对象，且与 `Block` 循环引用。
直接照搬 `Rc<Block>` 会泄漏或引入大量生命周期参数。

**决策**：

- `BlockState` 是 `Copy` 的 `(u16, u16)`；
- 属性值保存在 `Block` 的 `StateDefinition` 中；
- 全局方块注册表用 `OnceLock` 构建，`get_block()` 返回 `&'static Block`；
- `Blocks::stone()` 等访问函数替代 Java 静态字段。

**后果**：查询高效、无循环引用；代价是 `BlockState` 的 Debug/序列化需要
通过注册表，且本地状态 ID 不等同于 Java 的数值 ID。网络/存档兼容前必须
增加显式 ID 映射（见 ADR-011）。

---

## ADR-005：Java 继承先映射为“同名构造函数 + 公共数据模型”

**背景**：Rust 无类继承。26.3 的方块层次很深（`Block` → `SnowyBlock` →
`SpreadingSnowyBlock` → `GrassBlock`）。

**决策**：

- 数据（状态属性、渲染形状、碰撞）统一放 `Block`；
- 子类以同名构造函数表达，例如 `GrassBlock::new(Properties) -> Block`；
- 一旦出现真正的按类型分派（骨粉、随机刻、方块实体），改为 trait。

**后果**：当前可读且不伪造行为；代价是类型系统暂时无法区分
“草方块实例”和“普通方块实例”。在 `PROGRESS.md` 中逐类标注为“部分/占位”。

---

## ADR-006：Vulkan 采用像素画采样，并暂时关闭背面剔除

**背景**：Minecraft 贴图是 16×16 像素画；图集若使用线性过滤/mipmap 会
模糊并在 sprite 边界渗色。方块网格的面绕序尚未有回归测试锁定。

**决策**：

- 图集：`NEAREST`、`CLAMP_TO_EDGE`、不生成 mipmap、关闭各向异性；
- 清屏色改为 Minecraft 天空色 `(0.4706, 0.6549, 1.0)`；
- 图形管线 `cull_mode = NONE`；CPU 侧已按邻居剔除不可见面；
- 教程 compute 粒子默认关闭绘制（`Render::set_particles_enabled(false)`）。

**后果**：像素风正确、不会因绕序错误导致地形消失；代价是关闭背面剔除会
多绘制少量面。M2 增加渲染回归测试后再评估开启 `BACK` 与 mipmap。

---

## ADR-007：世界使用固定四层平地作为 M0 占位

**背景**：完整 26.3 世界生成涉及生物群系、密度函数、结构、特征、光照，
无法在首个切片内完成。但渲染链路需要一个可加载世界。

**决策**：`FlatLevelSource` 生成 `bedrock/dirt/dirt/grass_block` 四层，
世界参数与主世界一致（`min_y=-64`、高度 384、24 段），半径 4 区块。

**后果**：可验证区块、段、邻居剔除、碰撞与相机；但**不能**把
`FlatLevelSource` 视为已移植。M2 用真正的平地设置与生成管线替换。

---

## ADR-008：启动阶段同步加载，单线程

**背景**：Java 客户端异步加载资源/模型并分帧上传区块。首切片优先正确性和
可调试性。

**决策**：资源、模型、图集、世界网格全部在 `resumed` 中同步构建；网格一次
上传，不响应方块变化。

**后果**：启动路径简单、错误可定位；代价是启动时间随资源增长、运行时无法
更新区块。M2 引入任务队列与脏区块重建。

---

## ADR-009：允许 Java 方法重载在 Rust 中加后缀

**背景**：Rust 不支持重载，而 Java 大量使用 `offset(int,int,int)`/
`offset(Vec3i)`、`strength(float)`/`strength(float,float)`。

**决策**：使用可预测的后缀（`_vec`、`_n`、`_xyz`、`_one`、`_by`），并在
[PORTING.md](PORTING.md) 表中维护。调用点仍按 Java 语义选择。

**后果**：API 不再逐字相同，但可机械映射；新增重载时必须更新约定表。

---

## ADR-010：渲染层边界类型 `RenderMesh`/`RenderVertex`

**背景**：`ren_vulkan::Vertex` 使用 ash 相关布局；若 `minecraft` 直接引用，
会把 GPU 依赖带进移植层并阻碍测试。

**决策**：`minecraft` 输出 `RenderMesh { position, color, uv, indices }`；
`game` 转换为 `render::Vertex`。图集同理只输出 RGBA8 像素。

**后果**：模型/网格可在无 Vulkan 环境下单元测试；代价是一次很小的复制。

---

## ADR-011：当前状态/方块 ID 不承诺 Java 数值兼容

**背景**：本移植按自己的注册顺序分配 `block_id` 与 `state_index`，Java 的
`Block.BLOCK_STATE_REGISTRY` 数值由原版注册顺序决定。

**决策**：M0–M2 不处理网络与存档；UI/调试使用的状态身份只在本进程内有效。

**后果**：连接到 26.3 服务器或读取原版存档前，必须实现 Java 对应的注册表
数值映射与 NBT/协议编解码（M6）。

---

## ADR-012：玩家物理采用每帧 dt + 0.05 步长扫掠

**背景**：Java 实体是 20 TPS 固定刻，带子刻插值、流体/台阶/滑动等复杂规则。
首切片需要可操作的相机。

**决策**：`LocalPlayer::tick(level, input, dt)` 直接按帧更新，Y→X→Z 轴
以 0.05 方块步长扫掠；碰撞形状按整方块；速度/重力取等价数值。

**后果**：操作手感接近但不等于原版，可能出现台阶/边缘行为差异。M3 必须
重写为固定刻 + `VoxelShape`，本 ADR 届时废止。

---

## ADR-013：单 pass alpha 混合 + shader alpha discard，暂不分层排序

**背景**：26.3 按 `ChunkSectionLayer` 分成 solid/cutout/translucent 三层，
半透明层还会按视点排序。把这条链路完整移植需要多缓冲、多管线与排序器。

**决策**：

- 图形管线启用标准 alpha 混合；
- 片元 alpha < 0.1 时 `discard`，覆盖树叶等 cutout；
- `noOcclusion` 方块不参与遮挡剔除，玻璃类相邻同种方块跳过面；
- 所有面仍在同一个网格、同一次绘制中提交。

**后果**：树叶和玻璃已经可用，不透明方块视觉不受影响；但半透明面没有
排序，玻璃/水密集场景会出现混合伪影。M2 完成分层与排序后，本 ADR 废止，
`BakedQuad.force_translucent` 将用于选择图层。

---

## ADR-014：元素旋转不做 FaceBakery 的顶点重排

**背景**：Java `FaceBakery` 在元素旋转后重新计算法线，并按 `FaceInfo`
规范顺序交换顶点/UV（`recalculateWinding`）。

**决策**：本移植保留烘焙时的顶点与 UV 绑定，只重新计算 `direction` 与
`shade_direction`。旋转矩阵行列式为 +1，三角形绕序不会翻转。

**后果**：三角形覆盖与 UV 插值一致；当前关闭背面剔除，重排与否不影响
可见性。未来开启 `BACK` 前需要用渲染回归测试确认所有旋转模型的绕序；
若发现不一致，再补 `recalculateWinding` 的精确移植。

---

## ADR-015：方块破坏/放置是临时调试交互

**背景**：正式交互需要 `MultiPlayerGameMode`、`ClipContext`、物品栏、
方块更新与网络包。当前需要验证网格热更新与世界写入链路。

**决策**：

- 游戏层实现体素 DDA 射线（reach 5）与直接 `Level::set_block_state`；
- 左键破坏、右键放置；数字键 1–5 选择石头/草/木板/树叶/玻璃；
- 每次修改后全量重建网格并调用 `Render::update_mesh`。

**后果**：可以立即验证方块更改、透明材质与模型烘焙；但这**不是**正式
交互移植，没有掉落物、声音、冷却、物品消耗与服务器校验。M3 用正式类
替换它，并删除本 ADR 对应的调试代码。
