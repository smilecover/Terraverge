pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl WindowConfig {
    pub const DEFAULT_WIDTH: u32 = 1280;
    pub const DEFAULT_HEIGHT: u32 = 720;

    pub const MIN_WIDTH: u32 = 640;
    pub const MIN_HEIGHT: u32 = 360;

    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        let width = if width == 0 {
            Self::DEFAULT_WIDTH
        } else {
            width
        };
        let height = if height == 0 {
            Self::DEFAULT_HEIGHT
        } else {
            height
        };

        Self {
            title: title.into(),
            width: width.max(Self::MIN_WIDTH),
            height: height.max(Self::MIN_HEIGHT),
        }
    }

    pub fn with_size(title: impl Into<String>, size: (u32, u32)) -> Self {
        Self::new(title, size.0, size.1)
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new("Terraverge", 1440, 810)
    }
}
