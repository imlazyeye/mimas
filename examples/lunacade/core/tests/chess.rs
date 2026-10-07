use std::path::Path;

use lunacade_core::{Button, Cart, Input, Machine};

const BOARD: &str = include_str!("../../carts/chess/board.mim");
const AI: &str = include_str!("../../carts/chess/ai.mim");
const HELPERS: &str = include_str!("chess/helpers.mim");

fn check(script: &str) {
    let board = format!("{BOARD}\n{HELPERS}");
    // Exercise frontier handling directly, with the parent poised to receive its child's score.
    let ai = format!(
        "{AI}\n
         pub fn test_frontier(position: Board, m: int, depth: int) -> (int, int) {{
             let search = Search::new(position, [m]);
             search.board.make(m);
             search.plies[0].next = 1;
             search.open(depth, -LIMIT, LIMIT);
             if search.at == 0 {{ (0, search.plies[0].best) }}
             else {{ (search.plies[1].moves.len(), 0) }}
         }}"
    );
    let main = format!(
        "use board::{{Board, test_position, test_restore}};
         use ai::Search;
         fn perft(position: Board, depth: int) -> int {{
             if depth == 0 {{ return 1; }}
             let nodes = 0;
             for m in position.legal() {{
                 position.make(m);
                 nodes += perft(position, depth - 1);
                 position.unmake(m);
             }}
             nodes
         }}
         fn has(moves: [int], from: int, to: int, kind: int) -> bool {{
             for m in moves {{
                 if board::origin(m) == from && board::target(m) == to
                     && m >> board::KIND_SHIFT == kind {{ return true; }}
             }}
             false
         }}
         {script}"
    );
    let mut vm =
        mimas::compile_files(&[("board.mim", &board), ("ai.mim", &ai), ("test.mim", &main)])
            .unwrap_or_else(|error| panic!("chess test did not compile: {error:?}"));
    vm.fixture::<mimas::library::Random>().seed(1);
    vm.run()
        .unwrap_or_else(|error| panic!("chess test failed: {error:?}"));
}

#[test]
fn opening_move_counts_and_balanced_evaluation() {
    check(
        "let position = Board::new();
         for (depth, expected) in [(1, 20), (2, 400), (3, 8902)] {
             if perft(position, depth) != expected { panic(f\"opening perft {depth}\"); }
         }
         test_restore(position);
         // Equal, mirrored pawn advances must cancel in White's evaluation.
         for m in position.legal() {
             if board::origin(m) == 85 && board::target(m) == 65 { position.make(m); break; }
         }
         for m in position.legal() {
             if board::origin(m) == 35 && board::target(m) == 55 { position.make(m); break; }
         }
         if position.score != 0 { panic(\"symmetric moves changed the evaluation\"); }
         test_restore(position);",
    );
}

#[test]
fn kiwipete_castling_and_pins() {
    check(
        "// The standard Kiwipete position stresses pins, checks and both castling sides.
         let position = test_position([
             \"r...k..r\", \"p.ppqpb.\", \"bn..pnp.\", \"...PN...\",
             \".p..P...\", \"..N..Q.p\", \"PPPBBPPP\", \"R...K..R\",
         ], board::WHITE, 15, 0);
         for (depth, expected) in [(1, 48), (2, 2039), (3, 97862)] {
             if perft(position, depth) != expected { panic(f\"Kiwipete perft {depth}\"); }
         }
         test_restore(position);",
    );
}

#[test]
fn castling_cannot_cross_or_land_in_check() {
    check(
        "for file in [5, 6] {
             let top = if file == 5 \"....kr..\" else \"....k.r.\";
             let position = test_position([
                 top, \"........\", \"........\", \"........\",
                 \"........\", \"........\", \"........\", \"R...K..R\",
             ], board::WHITE, 3, 0);
             let moves = position.legal();
             if has(moves, 95, 97, board::CASTLE) { panic(\"castled through check\"); }
             if !has(moves, 95, 93, board::CASTLE) { panic(\"lost safe queenside castle\"); }
             test_restore(position);
         }",
    );
}

#[test]
fn en_passant_cannot_expose_the_king() {
    check(
        "let position = test_position([
             \"....k...\", \"........\", \"........\", \"r....pPK\",
             \"........\", \"........\", \"........\", \"........\",
         ], board::WHITE, 0, 46);
         let pseudo? = position.moves(false) else panic(\"invalid test position\");
         if !has(pseudo, 57, 46, board::EN_PASSANT) { panic(\"no en-passant candidate\"); }
         if has(position.legal(), 57, 46, board::EN_PASSANT) {
             panic(\"en passant exposed the king to the rook\");
         }
         test_restore(position);",
    );
}

#[test]
fn promotions_and_search_copies_restore_all_state() {
    check(
        "let position = test_position([
             \"r.r...k.\", \".P......\", \"........\", \"........\",
             \"........\", \"........\", \"........\", \"......K.\",
         ], board::WHITE, 0, 0);
         let moves = position.legal();
         // Five king moves and three queen-only promotions, including both captures.
         if moves.len() != 8 { panic(\"wrong promotion move count\"); }
         for to in 21..24 {
             if !has(moves, 32, to, board::PROMOTE) { panic(\"missing promotion\"); }
         }
         test_restore(position);
         let copy = position.copy();
         for m in copy.legal() {
             if board::origin(m) == 97 {
                 copy.make(m);
                 if position.king(board::WHITE) != 97 || position.sq[97] != board::KING {
                     panic(\"search copy changed the live king\");
                 }
                 copy.unmake(m);
                 break;
             }
         }",
    );
}

#[test]
fn search_finds_mate_and_keeps_the_live_position() {
    check(
        "let position = test_position([
             \".......k\", \"........\", \".....KQ.\", \"........\",
             \"........\", \"........\", \"........\", \"........\",
         ], board::WHITE, 0, 0);
         let search = Search::new(position, position.legal());
         for _ in 120 { if !search.done { search.think(); } }
         if !search.done || search.score < 28800 { panic(\"missed mate in one\"); }
         if position.king(board::WHITE) != 46 || position.sq[47] != board::QUEEN {
             panic(\"search changed the live position\");
         }
         position.make(search.best);
         if !position.in_check() || !position.legal().is_empty() { panic(\"best move is not mate\"); }
         position.unmake(search.best);
         test_restore(position);",
    );
}

#[test]
fn frontier_rejects_illegal_moves_before_evaluation() {
    check(
        "let position = test_position([
             \"k...r...\", \"........\", \"........\", \"........\",
             \"........\", \"........\", \"....R...\", \"....K...\",
         ], board::WHITE, 0, 0);
         // e2-d2 exposes White's king. Even at the capture-depth cap, it must be rejected.
         let (moves, score) = ai::test_frontier(position, 85 + 84 * 128, -4);
         if moves != 0 || score != -30000 { panic(\"frontier accepted a pinned rook move\"); }",
    );
}

#[test]
fn frontier_searches_quiet_check_evasions() {
    check(
        "let position = test_position([
             \"k.......\", \"....r...\", \"........\", \"........\",
             \"........\", \"........\", \"........\", \"....K...\",
         ], board::BLACK, 0, 0);
         // ...Re7-e8 checks White, whose only answers are quiet king moves.
         let (moves, score) = ai::test_frontier(position, 35 + 25 * 128, 0);
         if moves == 0 { panic(\"checked side was allowed to stand pat\"); }",
    );
}

#[test]
fn restarting_clears_the_entire_round() {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../carts/chess"));
    let mut cart = Cart::from_dir(path).unwrap();
    cart.files.get_mut("chess.mim").unwrap().push_str(
        "\ngame.round.hold = 99;
         game.round.positions = 1234;
         game.round.depth = 4;
         game.finish(\"TEST\");
         game.age = RESTART_DELAY;
         luna::update(|| {
             game.update();
             if game.age == 0 {
                 let round = game.round;
                 print((round.cursor, round.hold, round.resign, round.positions, round.depth));
             }
         });",
    );
    let mut machine = Machine::load(&cart, 1).unwrap();
    machine.frame(&Input {
        held: Button::Start.bit(),
        ..Input::default()
    });
    assert!(machine.fault().is_none());
    assert_eq!(machine.take_output(), ["[85, 0, 0, 0, 0]"]);
}
