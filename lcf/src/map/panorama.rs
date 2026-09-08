#[derive(Debug, Default)]
pub struct Panorama {
    pub name: String,
    pub loop_x: bool,
    pub loop_y: bool,
    pub auto_x: bool,
    pub auto_y: bool,
    pub speed_x: i32,
    pub speed_y: i32,
}
