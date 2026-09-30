type Actor = (u32, &'static str, u32, (i32, i32));

pub(super) fn cast(label: &str) -> Option<(u32, Vec<Actor>)> {
    Some(match label {
        "airship-murder-witnesses" => (
            129,
            vec![
                (6, "Chara1", 1, (15, 3)),
                (8, "Chara2", 4, (13, 5)),
                (9, "Chara4", 0, (15, 5)),
            ],
        ),
        "airship-return-departure" => (
            132,
            vec![
                (38, "Chara2", 2, (9, 9)),
                (39, "Chara4", 4, (8, 12)),
                (40, "Chara4", 2, (9, 12)),
                (41, "Chara4", 0, (8, 10)),
                (42, "Chara1", 2, (11, 11)),
                (43, "Chara1", 1, (10, 11)),
            ],
        ),
        "airship-return-deck" => (
            126,
            vec![
                (27, "Chara2", 2, (9, 9)),
                (30, "Chara1", 2, (11, 11)),
                (33, "Chara4", 0, (8, 10)),
                (51, "Chara4", 2, (9, 12)),
                (53, "Chara4", 4, (8, 12)),
                (54, "Chara1", 1, (10, 11)),
            ],
        ),
        "airship-return-tyran" => (
            152,
            vec![
                (2, "Chara1", 2, (8, 13)),
                (3, "Chara4", 4, (4, 14)),
                (4, "Chara4", 2, (4, 15)),
                (5, "Chara2", 2, (3, 12)),
                (6, "Chara1", 1, (5, 12)),
                (7, "Chara4", 0, (4, 11)),
            ],
        ),
        "airship-return-hospital" => (
            154,
            vec![
                (44, "Chara4", 4, (7, 8)),
                (45, "Chara4", 2, (7, 9)),
                (47, "Chara4", 0, (8, 7)),
                (48, "Chara1", 1, (9, 9)),
                (49, "Chara2", 2, (8, 10)),
            ],
        ),
        "airship-sky-briefing" => (
            126,
            vec![
                (27, "Chara2", 2, (9, 9)),
                (28, "Chara1", 2, (11, 10)),
                (51, "Chara4", 2, (9, 12)),
                (53, "Chara4", 4, (8, 12)),
                (54, "Chara1", 1, (10, 11)),
                (59, "Chara4", 0, (11, 12)),
            ],
        ),
        "airship-sky-island" => (
            170,
            vec![
                (8, "Chara1", 0, (15, 16)),
                (1, "Chara1", 1, (16, 16)),
                (2, "Chara4", 2, (13, 17)),
                (3, "Chara4", 4, (12, 17)),
                (4, "Chara4", 0, (18, 18)),
                (5, "Chara2", 2, (19, 18)),
                (6, "Chara1", 2, (17, 17)),
                (7, "Monster2", 3, (21, 17)),
            ],
        ),
        _ => return None,
    })
}
