use game_session_io::{self, Action, GameResult, GameStatus};
use gstd::prelude::*;
use gtest::{Program, System};

const USER: u64 = 10;

fn init_game_session(system: &System) -> Program {
    system.init_logger();
    system.mint_to(USER, 1000000000000000000000);

    let session_program = Program::from_file(
        system,
        "../target/wasm32-unknown-unknown/debug/game_session.opt.wasm",
    );

    let wordle_program = Program::from_file(
        system,
        "../target/wasm32-unknown-unknown/debug/wordle.opt.wasm",
    );

    wordle_program.send_bytes(USER, []);
    system.run_next_block();

    session_program.send(USER, wordle_program.id());
    system.run_next_block();

    session_program
}

#[test]
fn test_game_start_and_lose() {
    let system = System::new();
    let session_program = init_game_session(&system);

    session_program.send(USER, Action::StartGame);
    system.run_next_block();

    // Make incorrect guesses
    for _ in 0..6 {
        session_program.send(USER, Action::CheckWord("wrong".to_string()));
        system.run_next_block();
    }
}

#[test]
fn test_game_timeout() {
    let system = System::new();
    let session_program = init_game_session(&system);

    session_program.send(USER, Action::StartGame);
    system.run_next_block();
    // Simulate timeout
    system.run_to_block(3000);
    session_program.send(USER, Action::CheckGameStatus);
    system.run_next_block();
}

#[test]
fn test_game_win() {
    let system = System::new();
    let session_program = init_game_session(&system);

    // Start game
    session_program.send(USER, Action::StartGame);
    system.run_next_block();

    // Make correct guess
    session_program.send(USER, Action::CheckWord("world".to_string()));
    system.run_next_block();
    session_program.send(USER, Action::CheckWord("human".to_string()));
    system.run_next_block();

    session_program.send(USER, Action::CheckWord("house".to_string()));
    system.run_next_block();
    session_program.send(USER, Action::CheckWord("horse".to_string()));
    system.run_next_block();
    // Check final state
    let state: GameStatus = session_program.read_state(()).unwrap();
    assert_eq!(state, GameStatus::GameOver(GameResult::Win));
}
