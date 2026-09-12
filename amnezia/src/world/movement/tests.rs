use super::*;

#[test]
fn queue_tracks_pending_movement() {
    let mut q = MoveQueue::default();
    assert!(!q.busy());
    q.enqueue_route([RouteAction::Step {
        dx: 0,
        dy: 1,
        face: DIR_DOWN,
    }]);
    assert!(q.busy());
}

struct FakeChar {
    x: i32,
    y: i32,
    dir: u32,
    frame: u32,
    charset: String,
}

impl Character for FakeChar {
    fn tile(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    fn set_tile(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    fn dir(&self) -> u32 {
        self.dir
    }
    fn set_dir(&mut self, dir: u32) {
        self.dir = dir;
    }
    fn frame(&self) -> u32 {
        self.frame
    }
    fn set_frame(&mut self, frame: u32) {
        self.frame = frame;
    }
    fn index(&self) -> u32 {
        0
    }
    fn charset(&self) -> &str {
        &self.charset
    }
    fn set_graphic(&mut self, name: String, _index: u32) {
        self.charset = name;
    }
}

fn test_map() -> MapData {
    MapData {
        scroll_type: 0,
        panorama: None,
        map_id: 0,
        width: 5,
        height: 5,
        offset_x: 40.0,
        offset_y: 40.0,
        lower: vec![0; 25],
        upper: vec![10000; 25],
        passages_down: vec![0x0F; 162],
        passages_up: vec![0x0F; 144],
        terrain_data: Vec::new(),
        terrains: vec![amnezia_data::TerrainDef { id: 1, ..default() }],
    }
}

#[test]
fn advance_tweens_one_tile_without_changing_animation_state() {
    let data = test_map();
    let mut ch = FakeChar {
        x: 2,
        y: 2,
        dir: DIR_DOWN,
        frame: 2,
        charset: "C".into(),
    };
    let mut q = MoveQueue::default();
    q.enqueue_route([RouteAction::Step {
        dx: -1,
        dy: 0,
        face: 3,
    }]);
    // Collision positions advance before the sprite reaches the tile.
    let start = q.advance(&mut ch, &data, 0.0).unwrap();
    assert_eq!(ch.tile(), (1, 2));
    assert_eq!(ch.dir(), 3);
    let mid = q.advance(&mut ch, &data, STEP_DURATION / 2.0).unwrap();
    assert!(mid.x < start.x);
    assert!(q.busy());
    q.advance(&mut ch, &data, STEP_DURATION).unwrap();
    assert!(q.advance(&mut ch, &data, 0.0).is_none());
    assert!(!q.busy());
    assert_eq!(ch.frame(), 2);
}

#[test]
fn seam_crossings_tween_one_tile_with_a_canonical_logical_destination() {
    let mut data = MapData::for_test(140, 140);
    data.scroll_type = 3;
    for (start, delta, destination) in [
        ((0, 0), (-1, 0), (139, 0)),
        ((139, 0), (1, 0), (0, 0)),
        ((0, 0), (0, -1), (0, 139)),
        ((0, 139), (0, 1), (0, 0)),
    ] {
        let mut ch = FakeChar {
            x: start.0,
            y: start.1,
            dir: 0,
            frame: 1,
            charset: String::new(),
        };
        let mut q = MoveQueue::default();
        q.push_step(RouteAction::Step {
            dx: delta.0,
            dy: delta.1,
            face: 0,
        });
        let from = q.advance(&mut ch, &data, 0.0).unwrap();
        assert_eq!(ch.tile(), destination);
        let mid = q.advance(&mut ch, &data, STEP_DURATION / 2.0).unwrap();
        assert_eq!(q.render_position(&ch, &data), mid);
        assert_eq!(mid.distance(from), 8.0);
        let end = q.advance(&mut ch, &data, STEP_DURATION / 2.0).unwrap();
        assert_eq!(end.distance(from), 16.0);
        assert_eq!(data.world_near(q.render_position(&ch, &data), end), end);
    }
}
