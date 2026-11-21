//! 对应 `net/minecraft/core/Direction.java`。
//!
//! 枚举顺序、`data3d`/`data2d`、旋转关系、法向量都与 26.3 源码一致。

use super::Vec3i;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    DOWN,
    UP,
    NORTH,
    SOUTH,
    WEST,
    EAST,
}

impl Direction {
    pub const VALUES: [Direction; 6] = [
        Direction::DOWN,
        Direction::UP,
        Direction::NORTH,
        Direction::SOUTH,
        Direction::WEST,
        Direction::EAST,
    ];

    /// 按 `data3d` 排序后的数组（与 Java `BY_3D_DATA` 相同）。
    pub const BY_3D_DATA: [Direction; 6] = Self::VALUES;

    /// 水平方向按 `data2d` 排序：SOUTH(0)、WEST(1)、NORTH(2)、EAST(3)。
    pub const BY_2D_DATA: [Direction; 4] = [
        Direction::SOUTH,
        Direction::WEST,
        Direction::NORTH,
        Direction::EAST,
    ];

    pub const PLANE_HORIZONTAL: [Direction; 4] = [
        Direction::NORTH,
        Direction::EAST,
        Direction::SOUTH,
        Direction::WEST,
    ];

    pub const PLANE_VERTICAL: [Direction; 2] = [Direction::UP, Direction::DOWN];

    /// Java `Direction.DOWN` 等构造参数中的 `data3d`。
    pub const fn get_3d_data_value(self) -> i32 {
        match self {
            Direction::DOWN => 0,
            Direction::UP => 1,
            Direction::NORTH => 2,
            Direction::SOUTH => 3,
            Direction::WEST => 4,
            Direction::EAST => 5,
        }
    }

    /// Java 构造参数中的 `data2d`；垂直方向为 -1。
    pub const fn get_2d_data_value(self) -> i32 {
        match self {
            Direction::NORTH => 2,
            Direction::SOUTH => 0,
            Direction::WEST => 1,
            Direction::EAST => 3,
            Direction::DOWN | Direction::UP => -1,
        }
    }

    pub const fn get_axis_direction(self) -> AxisDirection {
        match self {
            Direction::DOWN | Direction::NORTH | Direction::WEST => AxisDirection::NEGATIVE,
            Direction::UP | Direction::SOUTH | Direction::EAST => AxisDirection::POSITIVE,
        }
    }

    pub const fn get_opposite(self) -> Direction {
        match self {
            Direction::DOWN => Direction::UP,
            Direction::UP => Direction::DOWN,
            Direction::NORTH => Direction::SOUTH,
            Direction::SOUTH => Direction::NORTH,
            Direction::WEST => Direction::EAST,
            Direction::EAST => Direction::WEST,
        }
    }

    /// 对应 `getClockWise(Axis)`：绕指定轴顺时针旋转 90°。
    pub const fn get_clock_wise(self, axis: Axis) -> Direction {
        match axis {
            Axis::X => self.get_clock_wise_x(),
            Axis::Y => self.get_clock_wise_y(),
            Axis::Z => self.get_clock_wise_z(),
        }
    }

    /// 对应 `getCounterClockWise(Axis)`。
    pub const fn get_counter_clock_wise(self, axis: Axis) -> Direction {
        match axis {
            Axis::X => self.get_counter_clock_wise_x(),
            Axis::Y => self.get_counter_clock_wise_y(),
            Axis::Z => self.get_counter_clock_wise_z(),
        }
    }

    /// 对应 `getClockWise()`（仅水平方向）。
    pub const fn get_clock_wise_y(self) -> Direction {
        match self {
            Direction::NORTH => Direction::EAST,
            Direction::SOUTH => Direction::WEST,
            Direction::WEST => Direction::NORTH,
            Direction::EAST => Direction::SOUTH,
            Direction::DOWN | Direction::UP => {
                panic!("Unable to get Y-rotated facing of vertical direction")
            }
        }
    }

    pub const fn get_counter_clock_wise_y(self) -> Direction {
        match self {
            Direction::NORTH => Direction::WEST,
            Direction::SOUTH => Direction::EAST,
            Direction::WEST => Direction::SOUTH,
            Direction::EAST => Direction::NORTH,
            Direction::DOWN | Direction::UP => {
                panic!("Unable to get CCW Y-rotated facing of vertical direction")
            }
        }
    }

    const fn get_clock_wise_x(self) -> Direction {
        match self {
            Direction::DOWN => Direction::SOUTH,
            Direction::UP => Direction::NORTH,
            Direction::NORTH => Direction::DOWN,
            Direction::SOUTH => Direction::UP,
            Direction::WEST | Direction::EAST => self,
        }
    }

    const fn get_counter_clock_wise_x(self) -> Direction {
        match self {
            Direction::DOWN => Direction::NORTH,
            Direction::UP => Direction::SOUTH,
            Direction::NORTH => Direction::UP,
            Direction::SOUTH => Direction::DOWN,
            Direction::WEST | Direction::EAST => self,
        }
    }

    const fn get_clock_wise_z(self) -> Direction {
        match self {
            Direction::DOWN => Direction::WEST,
            Direction::UP => Direction::EAST,
            Direction::WEST => Direction::UP,
            Direction::EAST => Direction::DOWN,
            Direction::NORTH | Direction::SOUTH => self,
        }
    }

    const fn get_counter_clock_wise_z(self) -> Direction {
        match self {
            Direction::DOWN => Direction::EAST,
            Direction::UP => Direction::WEST,
            Direction::WEST => Direction::DOWN,
            Direction::EAST => Direction::UP,
            Direction::NORTH | Direction::SOUTH => self,
        }
    }

    pub const fn get_step_x(self) -> i32 {
        self.get_step().get_x()
    }

    pub const fn get_step_y(self) -> i32 {
        self.get_step().get_y()
    }

    pub const fn get_step_z(self) -> i32 {
        self.get_step().get_z()
    }

    pub const fn get_step(self) -> Vec3i {
        match self {
            Direction::DOWN => Vec3i::new(0, -1, 0),
            Direction::UP => Vec3i::new(0, 1, 0),
            Direction::NORTH => Vec3i::new(0, 0, -1),
            Direction::SOUTH => Vec3i::new(0, 0, 1),
            Direction::WEST => Vec3i::new(-1, 0, 0),
            Direction::EAST => Vec3i::new(1, 0, 0),
        }
    }

    pub const fn step_tuple(self) -> (i32, i32, i32) {
        match self {
            Direction::DOWN => (0, -1, 0),
            Direction::UP => (0, 1, 0),
            Direction::NORTH => (0, 0, -1),
            Direction::SOUTH => (0, 0, 1),
            Direction::WEST => (-1, 0, 0),
            Direction::EAST => (1, 0, 0),
        }
    }

    pub const fn get_name(self) -> &'static str {
        match self {
            Direction::DOWN => "down",
            Direction::UP => "up",
            Direction::NORTH => "north",
            Direction::SOUTH => "south",
            Direction::WEST => "west",
            Direction::EAST => "east",
        }
    }

    pub const fn get_axis(self) -> Axis {
        match self {
            Direction::DOWN | Direction::UP => Axis::Y,
            Direction::NORTH | Direction::SOUTH => Axis::Z,
            Direction::WEST | Direction::EAST => Axis::X,
        }
    }

    pub fn by_name(name: &str) -> Option<Direction> {
        Self::VALUES.iter().copied().find(|d| d.get_name() == name)
    }

    /// Java 用 `Math.abs(data % 6)`，这里保留同样的环绕行为。
    pub fn from_3d_data_value(data: i32) -> Direction {
        let index = data.rem_euclid(Self::BY_3D_DATA.len() as i32) as usize;
        Self::BY_3D_DATA[index]
    }

    pub fn from_2d_data_value(data: i32) -> Direction {
        let index = data.rem_euclid(Self::BY_2D_DATA.len() as i32) as usize;
        Self::BY_2D_DATA[index]
    }

    pub fn from_y_rot(y_rot: f64) -> Direction {
        Self::from_2d_data_value((y_rot / 90.0 + 0.5).floor() as i32 & 3)
    }

    pub fn from_axis_and_direction(axis: Axis, direction: AxisDirection) -> Direction {
        match axis {
            Axis::X => match direction {
                AxisDirection::POSITIVE => Direction::EAST,
                AxisDirection::NEGATIVE => Direction::WEST,
            },
            Axis::Y => match direction {
                AxisDirection::POSITIVE => Direction::UP,
                AxisDirection::NEGATIVE => Direction::DOWN,
            },
            Axis::Z => match direction {
                AxisDirection::POSITIVE => Direction::SOUTH,
                AxisDirection::NEGATIVE => Direction::NORTH,
            },
        }
    }

    /// 对应 `toYRot`。
    pub const fn to_y_rot(self) -> f32 {
        (self.get_2d_data_value() & 3) as f32 * 90.0
    }

    /// 对应静态 `getYRot(Direction)`；垂直方向会 panic。
    pub const fn get_y_rot(direction: Direction) -> f32 {
        match direction {
            Direction::NORTH => 180.0,
            Direction::SOUTH => 0.0,
            Direction::WEST => 90.0,
            Direction::EAST => -90.0,
            Direction::DOWN | Direction::UP => {
                panic!("No y-Rot for vertical axis")
            }
        }
    }

    pub fn get_approximate_nearest_f64(dx: f64, dy: f64, dz: f64) -> Direction {
        Self::get_approximate_nearest(dx as f32, dy as f32, dz as f32)
    }

    /// 对应 `getApproximateNearest(float,float,float)`：取点积最大的方向。
    pub fn get_approximate_nearest(dx: f32, dy: f32, dz: f32) -> Direction {
        let mut result = Direction::NORTH;
        let mut highest_dot = f32::MIN;
        for direction in Self::VALUES {
            let step = direction.get_step();
            let dot =
                dx * step.get_x() as f32 + dy * step.get_y() as f32 + dz * step.get_z() as f32;
            if dot > highest_dot {
                highest_dot = dot;
                result = direction;
            }
        }
        result
    }

    pub fn get_nearest(x: i32, y: i32, z: i32, or_else: Option<Direction>) -> Option<Direction> {
        let abs_x = x.abs();
        let abs_y = y.abs();
        let abs_z = z.abs();
        if abs_x > abs_z && abs_x > abs_y {
            Some(if x < 0 {
                Direction::WEST
            } else {
                Direction::EAST
            })
        } else if abs_z > abs_x && abs_z > abs_y {
            Some(if z < 0 {
                Direction::NORTH
            } else {
                Direction::SOUTH
            })
        } else if abs_y > abs_x && abs_y > abs_z {
            Some(if y < 0 {
                Direction::DOWN
            } else {
                Direction::UP
            })
        } else {
            or_else
        }
    }

    pub fn get_nearest_vec(vec: Vec3i, or_else: Option<Direction>) -> Option<Direction> {
        Self::get_nearest(vec.get_x(), vec.get_y(), vec.get_z(), or_else)
    }
}

impl Direction {
    pub const fn axis_plane(self) -> Plane {
        match self.get_axis() {
            Axis::X | Axis::Z => Plane::HORIZONTAL,
            Axis::Y => Plane::VERTICAL,
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

impl Direction {
    /// Rust 侧便捷方法：把 `Direction::Axis` 当作枚举成员访问。
    pub const fn axis(self) -> Axis {
        self.get_axis()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const VALUES: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn by_name(name: &str) -> Option<Axis> {
        Self::VALUES.iter().copied().find(|a| a.get_name() == name)
    }

    pub const fn get_name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self, Axis::Y)
    }

    pub const fn is_horizontal(self) -> bool {
        matches!(self, Axis::X | Axis::Z)
    }

    pub const fn get_positive(self) -> Direction {
        match self {
            Axis::X => Direction::EAST,
            Axis::Y => Direction::UP,
            Axis::Z => Direction::SOUTH,
        }
    }

    pub const fn get_negative(self) -> Direction {
        match self {
            Axis::X => Direction::WEST,
            Axis::Y => Direction::DOWN,
            Axis::Z => Direction::NORTH,
        }
    }

    pub const fn get_directions(self) -> [Direction; 2] {
        [self.get_positive(), self.get_negative()]
    }

    pub const fn get_plane(self) -> Plane {
        match self {
            Axis::X | Axis::Z => Plane::HORIZONTAL,
            Axis::Y => Plane::VERTICAL,
        }
    }

    pub const fn choose_i32(self, x: i32, y: i32, z: i32) -> i32 {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }

    pub const fn choose_f64(self, x: f64, y: f64, z: f64) -> f64 {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }

    pub const fn choose_bool(self, x: bool, y: bool, z: bool) -> bool {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }

    pub fn test(self, input: Option<Direction>) -> bool {
        input.is_some_and(|direction| direction.get_axis() == self)
    }
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    POSITIVE,
    NEGATIVE,
}

impl AxisDirection {
    pub const fn get_step(self) -> i32 {
        match self {
            AxisDirection::POSITIVE => 1,
            AxisDirection::NEGATIVE => -1,
        }
    }

    pub const fn get_name(self) -> &'static str {
        match self {
            AxisDirection::POSITIVE => "Towards positive",
            AxisDirection::NEGATIVE => "Towards negative",
        }
    }

    pub const fn opposite(self) -> AxisDirection {
        match self {
            AxisDirection::POSITIVE => AxisDirection::NEGATIVE,
            AxisDirection::NEGATIVE => AxisDirection::POSITIVE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    HORIZONTAL,
    VERTICAL,
}

impl Plane {
    pub const fn faces(self) -> &'static [Direction] {
        match self {
            Plane::HORIZONTAL => &Direction::PLANE_HORIZONTAL,
            Plane::VERTICAL => &Direction::PLANE_VERTICAL,
        }
    }

    pub fn test(self, input: Option<Direction>) -> bool {
        input.is_some_and(|direction| direction.get_axis().get_plane() == self)
    }

    pub const fn length(self) -> usize {
        self.faces().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_values_match_vanilla() {
        assert_eq!(Direction::DOWN.get_3d_data_value(), 0);
        assert_eq!(Direction::EAST.get_3d_data_value(), 5);
        assert_eq!(Direction::NORTH.get_2d_data_value(), 2);
        assert_eq!(Direction::SOUTH.get_2d_data_value(), 0);
        assert_eq!(Direction::from_2d_data_value(5), Direction::WEST);
        assert_eq!(Direction::from_3d_data_value(-1), Direction::EAST);
    }

    #[test]
    fn rotations_match_java() {
        assert_eq!(Direction::NORTH.get_clock_wise_y(), Direction::EAST);
        assert_eq!(Direction::NORTH.get_counter_clock_wise_y(), Direction::WEST);
        assert_eq!(Direction::DOWN.get_clock_wise(Axis::X), Direction::SOUTH);
        assert_eq!(Direction::UP.get_clock_wise(Axis::Z), Direction::EAST);
    }

    #[test]
    fn y_rot_roundtrip() {
        for direction in Direction::BY_2D_DATA {
            assert_eq!(
                Direction::from_y_rot(direction.to_y_rot() as f64),
                direction
            );
        }
    }
}
