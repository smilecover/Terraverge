# 资源与模型管线

## 1. 资源根目录

当前直接使用解包的客户端目录：

```text
minecraft-26.3-client-src/
├── assets/minecraft/
│   ├── blockstates/     1288 个 JSON
│   ├── models/          4192 个 JSON
│   ├── textures/        约 3900 个 PNG
│   ├── lang/en_us.json  8559 个键
│   ├── atlases/         GUI/方块图集配置
│   └── shaders/         26.3 的原版 GLSL/管线定义（尚未启用）
├── data/minecraft/      data pack（尚未读取）
└── version.json
```

`ResourceManager` 以该目录为根：

- 资源：`assets/<namespace>/<path>`
- 数据：`data/<namespace>/<path>`
- 模型：`assets/<ns>/models/<path>.json`
- 方块状态：`assets/<ns>/blockstates/<path>.json`
- 贴图：`assets/<ns>/textures/<path>.png`
- 语言：`assets/<ns>/lang/<code>.json`

尚未支持：zip/jar 资源包、多资源包覆盖顺序、`pack.mcmeta` 版本校验、
异步重载、动画 `.mcmeta`。

## 2. 方块状态（blockstates）

示例 `blockstates/grass_block.json`：

```json
{
  "variants": {
    "snowy=false": [
      {"model": "minecraft:block/grass_block"},
      {"model": "minecraft:block/grass_block", "y": 90},
      {"model": "minecraft:block/grass_block", "y": 180},
      {"model": "minecraft:block/grass_block", "y": 270}
    ],
    "snowy=true": {"model": "minecraft:block/grass_block_snow"}
  }
}
```

实现（`client/renderer/block/dispatch`）：

- 支持 `variants` 对象：值为单个变体或变体数组；
- 选择器按 `a=b,c=d` 解析，逐属性与 `StateDefinition` 比较；
- 空字符串选择器匹配所有状态；
- 数组是加权变体：**当前固定取第一个**（Java 按 `weight` 随机），
  在 `PROGRESS.md` 中登记；
- `multipart` 尚未实现，遇到会返回明确错误，不会静默渲染成空。

变体字段：`model`（必填）、`x`、`y`、`uvlock`、`weight`。
当前只应用 `y` 旋转（90° 的倍数），`x` 旋转会明确报错。

## 3. 模型（cuboid）

模型路径与父链：

```text
stone.json
  -> minecraft:block/cube_all
       -> block/cube              (定义 elements)
            -> block/block        (只定义 display，无 elements)
```

合并规则（`ModelBakery`）：

1. 先递归解析父模型；
2. 子模型有 `elements` 时**整体替换**父元素，否则继承父元素；
3. `textures` 槽位逐键覆盖父模型；
4. `ambientocclusion` 子级优先，缺省为 `true`。

元素格式：

```json
{
  "from": [0, 0, 0],
  "to": [16, 16, 16],
  "faces": {
    "down": {"uv": [0,0,16,16], "texture": "#bottom", "cullface": "down"},
    "up":   {"texture": "#top", "tintindex": 0}
  }
}
```

支持字段：

| 字段 | 状态 |
| --- | --- |
| `from` / `to` | 支持，坐标范围 -16..32 校验 |
| `faces` | 支持，按 `Direction` 解析 |
| `texture` | 支持，`#slot` 别名链最多 16 层 |
| `uv` | 支持，缺省由 `defaultFaceUV` 推导 |
| `cullface` | 支持，烘焙时旋转 |
| `tintindex` | 支持，草色 tint |
| `rotation`（UV 象限） | 支持 0/90/180/270 |
| 元素 `rotation` | 只解析，烘焙时明确报错（未实现） |
| `shade_direction_override` | 只解析（未应用） |
| `light_emission` | 只解析（未应用） |

### UV 与顶点顺序

每个面 4 个顶点，顺序固定为：

```text
0 = (minU, minV)
1 = (minU, maxV)
2 = (maxU, maxV)
3 = (maxU, minV)
```

默认 UV 完全照抄 26.3 `FaceBakery.defaultFaceUV`：

```text
DOWN  : (from.x, 16-to.z, to.x, 16-from.z)
UP    : (from.x, from.z, to.x, to.z)
NORTH : (16-to.x, 16-to.y, 16-from.x, 16-from.y)
SOUTH : (from.x, 16-to.y, to.x, 16-from.y)
WEST  : (from.z, 16-to.y, to.z, 16-from.y)
EAST  : (16-to.z, 16-to.y, 16-from.z, 16-from.y)
```

面的顶点位置与默认 UV 的对应关系见
[crates/minecraft/src/net/minecraft/client/resources/model/cuboid/face_bakery.rs](../crates/minecraft/src/net/minecraft/client/resources/model/cuboid/face_bakery.rs)。

### 变体旋转

`y` 旋转绕方块中心 `(0.5, 0.5, 0.5)` 顺时针进行，方向和 `cullface`
同步旋转。公式：

```text
θ = -y°（弧度）
x' = (x-0.5)cosθ + (z-0.5)sinθ + 0.5
z' = -(x-0.5)sinθ + (z-0.5)cosθ + 0.5
```

## 4. 贴图槽与贴图解析

`textures` 是槽位映射，值可以继续以 `#` 引用其他槽位：

```json
"textures": {
  "particle": "#side",
  "down": "#bottom",
  "side": "minecraft:block/grass_block_side"
}
```

解析规则：

- `#name`：查槽位 `name`，继续追踪，最多 16 层；
- 非 `#` 值：交给 `Identifier::parse`（无命名空间时默认 `minecraft`）；
- 面引用的最终贴图标识符决定图集中的 sprite。

## 5. 图集

实现：`client/renderer/texture`。

```text
SpriteLoader::load_atlas(ids)
  -> SpriteContents::load(PNG -> RGBA8)
  -> Stitcher::stitch(单元格行列布局)
  -> TextureAtlas::compose(拷贝像素 + 计算 UV)
```

当前布局：

- 单元格边长 = 所有 sprite 的最大宽/高；
- 列数 = `ceil(sqrt(n))`，行数 = `ceil(n/列数)`；
- 本次启动：9 张 16×16 贴图 → 48×48 图集；
- sprite UV 按其实际宽高计算，非正方形贴图也能正确采样。

已知限制：

- 没有动画贴图与 `.mcmeta`；
- 没有 padding/mip 级别装箱；依赖最近邻采样与不生成 mipmap 避免渗色；
- 缺失贴图使用紫黑棋盘格，而不是中断加载；
- 图集不写回磁盘，调试时可在 `TextureAtlas` 上增加导出方法。

## 6. 着色

- 草方块顶部 `tintindex=0` 使用固定草色
  `(0.5686, 0.7412, 0.3490)`；26.3 的生物群系着色器尚未移植。
- 面向明暗系数（Minecraft 固定值）：UP 1.0、DOWN 0.5、
  NORTH/SOUTH 0.8、WEST/EAST 0.6。
- 最终顶点色 = tint × shade；片元着色器再乘以图集采样值。
- 草方块侧面的 overlay 与底面 quad 重合，深度测试 `LESS` 会保留先绘制
  的底层；overlay 的透明混合管线尚未实现。

## 7. 网格构建

`SectionCompiler::compile`：

- 遍历所有已加载区块的非空段；
- 跳过空气与 `RenderShape::INVISIBLE`；
- 通过 `BlockState -> BakedModel` 查表；
- 有 `cullface` 的面，若邻居“非空气且可碰撞”则跳过；
- 输出世界坐标顶点（方块坐标 + 0..1 局部坐标）；
- 索引为两个三角形 `[0,1,2, 0,2,3]`。

本次启动：81 区块 → 177,408 顶点 / 44,352 四边形。底面与地图外缘的
侧面也会绘制，因此四边形数明显多于 81×256 个顶面。

## 8. 资源自检

```bash
cargo test -p minecraft
```

其中：

- `loads_vanilla_models_when_source_tree_is_present` 会真实加载 26.3 的
  blockstates/models；
- `compiles_flat_world_into_visible_top_faces` 会构建图集并编译 9 个区块的
  网格；
- 若资源目录不存在，这两个测试会打印“跳过”而不是失败。
