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
        "wand::test_piercing();",
        Some((
            "weapons/wand.mim",
            "\npub fn test_piercing() {
                 let p = Player::new(Vec2::zero(), false);
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
         let weapons: [weapons::Weapon] = [wand::Wand::new(), spark::Spark::new(),
             orbit::Orbit::new(), bomb::Bomb::new()];
         for weapon in weapons { while !weapon.maxed() { weapon.level_up(); } }
         p.ranks = array::new_filled(player::MAX_RANK, player::PASSIVES.len());
         let rolled = Offer::roll(p, weapons);
         if rolled != [Offer::Heal] { panic(\"capped upgrades need a usable heal offer\"); }
         p.hp = 30;
         rolled[0].apply(p, weapons);
         if p.hp != 65 { panic(\"heal did not restore health\"); }",
        None,
    );
}

#[test]
fn a_maxed_swarm_weapon_evolves_once_its_catalyst_is_held() {
    check(
        "swarm",
        "let p = game.run.player;
         let held: [weapons::Weapon] = [knife::Knives::new()];
         for weapon in bases() {
             if weapon.EVOLVED { panic(\"an evolved weapon is on the menu\"); }
         }
         while !held[0].maxed() { held[0].level_up(); }
         for _ in 40 {
             for offer in Offer::roll(p, held) {
                 if offer.title(held) == \"EVOLVE: FLURRY\" { panic(\"evolved without its catalyst\"); }
             }
         }
         p.boost(player::Passive::Haste);
         let rolled = Offer::roll(p, held);
         if rolled[0].title(held) != \"EVOLVE: FLURRY\" { panic(\"a ready evolution wasn't offered first\"); }
         rolled[0].apply(p, held);
         if held.len() != 1 || held[0].NAME != \"FLURRY\" { panic(\"the knife didn't become the flurry\"); }
         for _ in 40 {
             for offer in Offer::roll(p, held) {
                 if offer.title(held) in [\"NEW: KNIFE\", \"NEW: FLURRY\", \"EVOLVE: FLURRY\"] {
                     panic(\"an evolved weapon or its base came back\");
                 }
             }
         }",
        None,
    );
}

#[test]
fn swarm_elites_come_on_kill_counts_and_a_lich_every_ten_minutes() {
    check(
        "swarm",
        "fn elites(run: Run) -> int {
             (for foe in run.horde.foes { if foe.kind.elite() collect foe; }).len()
         }
         let run = Run::new(Mode::Normal);
         run.horde.kills = waves::ELITE_EVERY - 1;
         run.update();
         if elites(run) != 0 { panic(\"an elite came early\"); }
         for count in 1..=3 {
             run.horde.kills = waves::ELITE_EVERY * count;
             run.update();
             run.update();
             if elites(run) != count { panic(\"one elite for every count of kills\"); }
         }
         if run.horde.boss != null { panic(\"a lich came on kills\"); }
         run.t = waves::LICH_EVERY - 1;
         run.update();
         let first? = run.horde.boss else panic(\"no lich at ten minutes\");
         first.hp = 0.0;
         if run.update() == Event::Died { panic(\"the run ended with the lich\"); }
         run.t = waves::LICH_EVERY * 2 - 1;
         run.update();
         let second? = run.horde.boss else panic(\"no lich at twenty minutes\");
         if second.id == first.id || second.max_hp != first.max_hp * 2.0 {
             panic(\"the second lich should be new and twice as tough\");
         }",
        None,
    );
}

#[test]
fn swarm_splits_are_capped_each_frame() {
    check(
        "swarm",
        "use std::math::Vec2;
         let at = Vec2::zero();
         let horde = horde::Horde::new();
         let brutes = for _ in 5 collect horde.spawn(enemies::Brute::new(), at, 2.0);
         horde.update(at);
         for brute in brutes.reversed() {
             if brute.id != 0 { horde.hurt(brute, 9999.0, at, 0.0); }
         }
         if horde.kills != 4 || horde.len() != 5 + 8 { panic(\"four brutes should leave eight skulls\"); }
         let born = horde.foes[5];
         if born.kind.boss() || born.max_hp != born.kind.HP * 2.0 || born.kind.SIZE != 8 { panic(\"a child lost its parent's toughness\"); }
         if horde.touching(at, 50.0).len() != 1 { panic(\"children were hittable in their birth frame\"); }
         horde.update(at);
         horde.hurt(brutes[0], 9999.0, at, 0.0);
         if horde.len() != 1 + 8 + 3 { panic(\"the cap didn't reset on the next frame\"); }",
        None,
    );
}

#[test]
fn a_swarm_crate_heals_or_clears_the_screen_but_spares_the_lich() {
    check(
        "swarm",
        "use std::math::vec2;
         let run = Run::new(Mode::Normal);
         let p = run.player;
         let near = for i in 6 collect run.horde.spawn(enemies::Slime::new(), p.pos.add(vec2(40.0 + i.to_float(), 30.0)), 1.0);
         let far = run.horde.spawn(enemies::Slime::new(), p.pos.add(vec2(260.0, 0.0)), 1.0);
         let lich = run.horde.spawn(enemies::Lich::new(), p.pos.add(vec2(-60.0, 0.0)), 1.0);
         run.horde.boss = lich;
         crates::test_put(run.crates, p.pos, crates::Loot::Bomb);
         run.update();
         for foe in near { if foe.hp > 0.0 { panic(\"the bomb left a foe on the screen\"); } }
         if far.hp <= 0.0 { panic(\"the bomb reached past the screen\"); }
         if lich.hp != lich.max_hp { panic(\"the bomb hurt the lich\"); }
         if run.horde.kills != 6 { panic(\"bomb kills weren't counted\"); }
         p.hp = 50;
         crates::test_put(run.crates, p.pos, crates::Loot::Heal);
         run.update();
         if p.hp != 80 { panic(\"the crate didn't heal\"); }
         for _ in 12 {
             if crates::test_drop(run.crates, p) != null { panic(\"a new crate landed on the player\"); }
         }
         if crates::test_count(run.crates) != 5 { panic(\"crates piled up\"); }",
        Some((
            "crates.mim",
            "\npub fn test_put(all: Crates, pos: Vec2, loot: Loot) {
                 all.list.push(Crate { pos, loot });
             }
             pub fn test_drop(all: Crates, p: Player) -> Loot? {
                 all.wait = 1;
                 all.update(p)
             }
             pub fn test_count(all: Crates) -> int { all.list.len() }",
        )),
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
