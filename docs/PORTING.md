# 移植规则

本文件是 `crates/minecraft` 的硬性约定。新增代码前先读一遍；与这里冲突的
写法必须先在 [DECISIONS.md](DECISIONS.md) 记录理由。

## 1. 基准与源码位置

- 基准：仓库根目录 `minecraft-26.3-client-src/`，`version.json` 中
  `id=26.3`、`protocol_version=777`、资源包 `resource_major=97`。
- **禁止修改 Java 源码目录**。它是对照物，不是构建输入。
- 以类文件为单位对照；方法、常量、字段名优先保持语义一致，而不是保持
  字面相同（Rust 命名规则不同，见第 3 节）。

## 2. 目录与模块映射

`net.minecraft.client.renderer.texture.SpriteLoader` 对应：

```text
crates/minecraft/src/net/minecraft/client/renderer/texture/sprite_loader.rs
```

- 包路径原样保留，只做 snake_case 转换；
- 每个 Java 类一个 Rust 文件（Java 内部类可拆为同目录附属文件）；
- `package-info.java` 忽略；
- 一个包对应目录下的 `mod.rs`，`mod.rs` 里显式 `pub mod`/`pub use`；
- 文件名使用 Java 类名的 snake_case，例如 `BlockStateModelDispatcher` →
  `block_state_model_dispatcher.rs`。

## 3. 命名

| Java | Rust | 示例 |
| --- | --- | --- |
| 类/枚举/记录 | 同名 PascalCase 类型 | `BlockState`、`Direction` |
| 方法/字段 | snake_case | `getBlockState` → `get_block_state` |
| 常量 | 同名 `SCREAMING_SNAKE_CASE` | `SECTION_SIZE` |
| 枚举值 | 保持大写 | `Direction::NORTH` |
| 包 | snake_case 路径 | `net::minecraft::core` |

### 重载

Rust 不支持重载。按以下后缀消歧：

| Java | Rust |
| --- | --- |
| `offset(int,int,int)` | `offset(x, y, z)` |
| `offset(Vec3i)` | `offset_vec(vec)` |
| `above()` / `above(int)` | `above()` / `above_n(steps)` |
| `strength(float)` / `strength(float,float)` | `strength_one(v)` / `strength(v, r)` |
| `getHeight(Types,x,z)` | 专用函数名，如 `get_height_motion_blocking` |
| `Vec3.add(double)` / `add(Vec3)` | `add(value)` / `add_vec(vec)` |
| `move(...)`（Rust 关键字） | `move_by(...)` |

重载规则属于“允许的 API 差异”，调用点仍需与 Java 语义一一对应。

### 嵌套类型

Java 的 `Direction.Axis`、`Direction.AxisDirection`、`Direction.Plane`
在 Rust 中是 `net::minecraft::core` 下的独立类型：`Axis`、
`AxisDirection`、`Plane`。使用 `Direction::axis()` / `Axis::get_positive()`
等方法，不伪造 `Direction::Axis` 路径。

### 26.3 特有命名

26.3 已把旧版 `ResourceLocation` 重命名为 `Identifier`（包路径仍是
`net.minecraft.resources`）。移植必须使用 `Identifier`，不要引入旧名字。
`BlockItemIds`、`BonemealSource` 等新类型同理，按原类名移植。

## 4. 继承与多态

Java 的类继承不能直接映射。当前采用：

1. **数据模型优先**：状态、属性、渲染形状放进公共 `Block`；
2. 子类行为先以“同名构造函数”表达，例如 `GrassBlock::new(Properties) -> Block`；
3. 需要真正按类型分派时，改为 trait：

```rust
pub trait BonemealableBlock { ... }
```

4. 不允许把 Java 的继承层次“拍平”后不记录；每个被拍平的类在
   `docs/PROGRESS.md` 的备注列说明。

现有例子：

- `AirBlock`：构造函数把渲染形状设为 `INVISIBLE`；
- `SnowyBlock`：构造函数注入 `SNOWY` 属性；
- `SpreadingSnowyBlock`/`GrassBlock`：保留类名与状态，行为待光照/特性系统。

## 5. 方块状态与注册表

Java 的 `BlockState` 是全局状态表中的对象身份；跨 `Block` ↔ `BlockState`
存在循环引用。Rust 采用等价的轻量身份：

```rust
pub struct BlockState {
    block_id: u16,
    state_index: u16,
}
```

- `block_id`：`BuiltInRegistries::block()` 中的下标；
- `state_index`：该方块 `StateDefinition` 展开后的状态下标；
- 属性值保存在 `StateDefinition` 中，`BlockState` 本身是 `Copy`；
- `BlockState::get_block()` 通过 `OnceLock` 注册表解析成 `&'static Block`；
- 状态 ID 是**本移植的注册顺序**，不承诺与 Java 的网络/存档数值 ID 相同；
  在需要网络协议/NBT 兼容时必须引入 Java 的注册表数值映射。

Java 静态字段 `Blocks.STONE` 在 Rust 中变成访问函数 `Blocks::stone()`，
因为注册表是运行期懒初始化的；访问函数每次都返回注册表中的
`&'static Block`，不复制方块。

## 6. 静态初始化

Java 的静态块在类首次加载时执行，且允许相互引用。Rust 使用：

```rust
static REGISTRY: OnceLock<Registry<Block>> = OnceLock::new();
pub fn block() -> &'static Registry<Block> {
    REGISTRY.get_or_init(Blocks::bootstrap)
}
```

约定：

- 初始化函数里**不要**再调用公开访问器（会重入 `OnceLock`）；
- 注册顺序在 `Blocks::bootstrap` 中显式书写，和 `Blocks.java` 的字段顺序
  对齐（当前只包含首批方块）；
- 初始化完成后注册表只读，可安全地跨线程共享。

## 7. 错误与空值

| Java | Rust |
| --- | --- |
| `throw new IdentifierException(...)` | `Result<_, IdentifierException>` |
| `@Nullable` | `Option<T>` |
| Gson `JsonParseException` | `JsonError`（`util::json`） |
| 资源缺失 | `ResourceError::NotFound` |
| 模型数据错误 | `ModelError` |
| 不应失败的不变量 | `assert!` / `expect`，并写清不变量 |

禁止用 `unwrap()` 处理外部资源数据；测试和硬编码常量除外。

## 8. JSON、Codec 与网络

- Java 使用 Gson/DFU Codec；Rust 侧实现了严格 JSON 解析器
  `net::minecraft::util::json`。新增格式时优先写**显式字段解析**，
  错误信息带上文件/字段名。
- 不支持注释、尾随逗号（与 26.3 的 `StrictJsonParser` 一致）。
- NBT、网络 ByteBuf、Codec/StreamCodec 尚未移植；涉及这些的类先不要
  伪造“能解析”的接口，应在 `PROGRESS.md` 标记阻塞。

## 9. 浮点与坐标

- Java `double` → Rust `f64`；`float` → `f32`；坐标、AABB 用 `f64`。
- 方块坐标是整数 `i32`；区块段坐标右移 4 位（算术右移，负数向下取整）。
- 渲染网格顶点用 `f32`（与 GPU/渲染后端一致）。
- Minecraft 视角约定：yaw=0 朝南(+Z)，yaw 增大向西；pitch>0 向下看。
- 模型坐标单位为 1/16 方块；图集 UV 为 0..1，模型 JSON 的 UV 为 0..16。

## 10. 渲染后端的边界

`minecraft` crate **不得**依赖 `ash`、`winit`、`glam` 或 `render`。
它只输出：

- `RenderMesh`（位置/颜色/UV + u32 索引）；
- 图集 RGBA8 像素；
- 相机/玩家状态。

`game` 层负责把 `RenderVertex` 转成 `ren_vulkan::Vertex`，并把像素交给
`TextureData`。这样后端替换或增加（例如未来软件光栅化测试）不会污染移植层。

## 11. 新类移植流程

1. 找到 Java 文件，通读类头、常量、构造器、公开 API；
2. 在相同的 `net::minecraft::...` 路径建 Rust 文件，添加模块声明；
3. 先移植不可变数据与 getter，再移植纯函数，最后移植带 I/O/全局状态的逻辑；
4. 每个关键行为加单元测试，测试名描述 Java 语义（如
   `packed_layout_matches_java_offsets`）；
5. 如遇阻塞（光照、实体、NBT 等），在 `PROGRESS.md` 写清阻塞点，
   **不要**用简化实现冒充；
6. 更新 `PROGRESS.md` 和 `DECISIONS.md`（若产生新偏差）；
7. 运行 `cargo test --workspace`，再运行客户端做一次目视检查
   （涉及渲染时）。

## 12. 完成定义

一个类算“已移植”需要同时满足：

- 公开 API 与 Java 对应（允许第 3 节的命名差异）；
- 关键常量/数据布局与 Java 数值一致，并有测试；
- 不依赖伪造的占位行为；
- 在 `PROGRESS.md` 中列出 Java 路径与 Rust 路径；
- 如果只完成了一部分，明确标记“部分”，并列出缺失方法/行为。
