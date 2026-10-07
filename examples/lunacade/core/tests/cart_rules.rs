use std::path::Path;

use lunacade_core::{Cart, Input, Machine, WIDTH};

fn check(name: &str, script: &str, module: Option<(&str, &str)>) {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../carts"));
    let mut cart = Cart::from_dir(&root.join(name)).unwrap();
    cart.files
        .get_mut(&format!("{name}.mim"))
        .unwrap()
        .push_str(&format!("\n{script}"));
    if let Some((file, extra)) = module {
        cart.files.get_mut(file).unwrap().push_str(extra);
    }
    Machine::load(&cart, 1).unwrap_or_else(|problems| panic!("{name}: {problems:?}"));
}

#[test]
fn snake_wins_when_the_board_is_full() {
    check(
        "snake",
        "game.begin();
         game.body = for cell in CELLS collect cell;
         game.occupied = array::new_filled(true, CELLS);
         game.place_food();
         if game.scene != Scene::Over(true) || game.timer != 0 {
             panic(\"a filled board should win without choosing food\");
         }",
        None,
    );
}

#[test]
fn snake_allows_a_vacating_tail_and_preserves_a_fatal_position() {
    check(
        "snake",
        "fn prepare(game: Game) {
             game.begin();
             game.body = [33, 34, 66, 65];
             game.occupied = array::new_filled(false, CELLS);
             for cell in game.body { game.occupied[cell] = true; }
             game.food = 100;
         }
         prepare(game);
         game.turn = ivec2(0, 1);
         game.step();
         if game.body != [65, 33, 34, 66] || game.scene != Scene::Playing {
             panic(\"the tail vacates its cell\");
         }
         prepare(game);
         game.turn = ivec2(1, 0);
         game.step();
         if game.scene != Scene::Over(false) || game.body != [33, 34, 66, 65] {
             panic(\"a fatal move changed the snake\");
         }
         for cell in CELLS {
             if game.occupied[cell] != (cell in game.body) { panic(\"stale occupancy\"); }
         }
         game.body = [0, 1, 2];
         game.turn = ivec2(-1, 0);
         game.step();
         if game.body != [0, 1, 2] { panic(\"wall collision removed the tail\"); }",
        None,
    );
}

#[test]
fn breakout_breaks_a_brick_once_and_docks_before_serving() {
    check(
        "breakout",
        "game.ball.pos = vec2((LEFT + BRICK_W ~/ 2).to_float(), (TOP + BRICK_H ~/ 2).to_float());
         if !game.collide() || game.collide() { panic(\"brick should break once\"); }
         if game.standing != ROWS * COLS - 1 || game.score != ROWS * 10 {
             panic(\"wrong remaining bricks or score\");
         }
         if game.brick_at(LEFT - 1, TOP) != null || game.brick_at(RIGHT, TOP) != null {
             panic(\"brick lookup escaped the rack\");
         }
         game.ball.pos = vec2(0.0, 999.0);
         game.enter(Scene::Serve);
         if game.ball.pos.y != PADDLE_Y - RADIUS {
             panic(\"serve still shows the lost ball\");
         }",
        None,
    );
}

#[test]
fn asteroid_children_are_not_shot_in_their_birth_frame() {
    check(
        "asteroids",
        "let run = Run::new();
         let at = vec2(32.0, 32.0);
         run.rocks = [Rock::new(at, 2, 0.0)];
         run.shots = [Shot { pos = at, vel = Vec2::zero(), life = 10 },
                      Shot { pos = at, vel = Vec2::zero(), life = 10 }];
         run.shoot_rocks();
         if run.score != POINTS[2] || run.rocks.len() != 2 {
             panic(\"new children were hit in the same scan\");
         }
         for rock in run.rocks { if rock.size != 1 { panic(\"wrong split size\"); } }
         for shot in run.shots { if shot.life != 0 { panic(\"used shot remained alive\"); } }
         run.shots = [Shot { pos = at, vel = Vec2::zero(), life = 10 }];
         run.shoot_rocks();
         if run.score != POINTS[2] + POINTS[1] || run.rocks.len() != 3 {
             panic(\"split survivors were lost or hit twice\");
         }
         if run.rocks[0].size != 0 || run.rocks[1].size != 0 || run.rocks[2].size != 1 {
             panic(\"split changed collision tie-breaking order\");
         }",
        None,
    );
}

#[test]
fn swarm_grid_queries_match_a_full_scan_and_reset_cleanly() {
    check(
        "swarm",
        "use std::math::{Vec2, vec2};
         let center = vec2(-32.0, 17.0);
         let horde = horde::Horde::new();
         for i in 40 {
             let offset = vec2((i * 9 - 180).to_float(), (i * 17 % 83 - 40).to_float());
             horde.spawn(enemies::Slime::new(), center.add(offset), 1.0);
         }
         horde.foes[3].hp = 0.0;
         horde.update(center);
         for x in [-50.5, 0.0, 40.0] {
             let at = center.add(vec2(x, -13.25));
             let actual = horde.touching(at, 30.0);
             let expected = for foe in horde.foes {
                 let r = 30.0 + foe.kind.REACH;
                 if foe.hp > 0.0 && foe.pos.distance_squared(at) < r * r collect foe;
             };
             if actual.len() != expected.len() { panic(\"missed grid overlap\"); }
             for foe in expected {
                 if foe !in actual { panic(\"grid returned the wrong overlap\"); }
             }
             let skip = [0, 7, 12];
             let nearest? = horde.nearest(at, 150.0, skip) else panic(\"missing nearest foe\");
             let d = nearest.pos.distance_squared(at);
             if nearest.id in skip { panic(\"nearest ignored excluded ids\"); }
             for foe in horde.foes {
                 if foe.hp > 0.0 && foe.id !in skip && foe.pos.distance_squared(at) < d {
                     panic(\"grid missed a closer foe\");
                 }
             }
         }
         horde.grid.reset(center);
         if !horde.touching(center, 250.0).is_empty() { panic(\"stale grid entries\"); }
         let outside = horde.spawn(enemies::Bat::new(),
             vec2(horde.grid.left.to_float() - 0.1, center.y), 1.0);
         horde.grid.insert(outside);
         for bucket in horde.grid.cells {
             if !bucket.is_empty() { panic(\"negative offset entered bucket zero\"); }
         }",
        None,
    );
}

#[test]
fn a_piercing_bolt_can_hit_another_overlapping_enemy() {
    check(
        "swarm",
        "weapons::test_piercing();",
        Some((
            "weapons.mim",
            "\npub fn test_piercing() {
                 let p = Player::new(vec2(0.0, 0.0), false);
                 let horde = Horde::new();
                 let first = horde.spawn(enemies::Slime::new(), p.pos, 1.0);
                 let next = horde.spawn(enemies::Slime::new(), p.pos, 1.0);
                 horde.update(p.pos);
                 let wand = Wand::new();
                 wand.cool = 999;
                 wand.bolts = [Bolt { pos = p.pos, vel = Vec2::zero(), life = 10,
                                      pierce = 2, last = first.id }];
                 wand.update(p, horde, Fx::new());
                 if first.hp != first.max_hp || next.hp >= next.max_hp {
                     panic(\"previous target hid another overlapping foe\");
                 }
                 if wand.bolts[0].last != next.id || wand.bolts[0].pierce != 1 {
                     panic(\"piercing hit was not recorded\");
                 }
             }",
        )),
    );
}

#[test]
fn capped_swarm_upgrades_still_offer_a_heal() {
    check(
        "swarm",
        "let p = game.run.player;
         let weapons: [weapons::Weapon] = [weapons::Wand::new(), weapons::Whip::new(),
             weapons::Orbit::new(), weapons::Bomb::new()];
         for weapon in weapons { for _ in 5 { weapon.level_up(); } }
         p.ranks = array::new_filled(player::MAX_RANK, 4);
         let rolled = Offer::roll(p, weapons);
         if rolled != [Offer::Heal] { panic(\"capped upgrades need a usable heal offer\"); }
         p.hp = 30;
         rolled[0].apply(p, weapons);
         if p.hp != 65 { panic(\"heal did not restore health\"); }",
        None,
    );
}

#[test]
fn swarm_ground_scrolls_continuously_across_negative_tile_boundaries() {
    fn draw(x: i32) -> Vec<u8> {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../carts/swarm"));
        let mut cart = Cart::from_dir(root).unwrap();
        cart.files.get_mut("swarm.mim").unwrap().push_str(&format!(
            "\nuse std::math::ivec2;\n
             luna::draw(|| ground::draw(ivec2({x}, -16)));"
        ));
        let mut machine = Machine::load(&cart, 1).unwrap();
        machine.frame(&Input::default());
        assert!(machine.fault().is_none());
        machine.screen().pixels.to_vec()
    }
    let before = draw(-16);
    let after = draw(-15);
    for (before, after) in before.chunks(WIDTH).zip(after.chunks(WIDTH)) {
        assert_eq!(&before[1..], &after[..WIDTH - 1], "scenery jumped a tile");
    }
}
