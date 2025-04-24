#![no_std]
#![allow(warnings)]
use game_session_io::*;
use gstd::{debug, exec, msg, prelude::*, ActorId, MessageId};
use wordle_io;

const MAX_ATTEMPTS: u8 = 6;
const GAME_DURATION_BLOCKS: u32 = 200;

static mut GAME: Option<Game> = None;

#[derive(Default)]
pub struct Game {
    wordle_program_id: ActorId,
    game_status: GameStatus,
    previous_guesses: Vec<GuessResult>,
    session_status: SessionStatus,
    msg_ids: (MessageId, MessageId),
}

#[no_mangle]
extern "C" fn init() {
    let init_payload: InitGameSession = msg::load().expect("Failed to decode InitGameSession");
    let game = Game {
        wordle_program_id: init_payload.wordle_program_id,
        game_status: GameStatus::NotStarted,
        previous_guesses: Vec::new(),
        session_status: SessionStatus::Waiting,
        msg_ids: (MessageId::zero(), MessageId::zero()),
    };
    debug!("GAME: {:?}", game.game_status);
    unsafe { GAME = Some(game) };
}

#[no_mangle]
extern "C" fn handle() {
    debug!("!!!! HANDLE !!!!");
    debug!("Message ID: {:?}", msg::id());
    let action: Action = msg::load().expect("Failed to decode Action");
    debug!("Message payload: {:?}", action);
    let game = unsafe { GAME.as_mut().expect("Game is not initialized") };

    match &game.session_status {
        SessionStatus::Waiting => {
            debug!("HANDLE: SessionStatus::Waiting");
            match action {
                Action::StartGame => {
                    if !matches!(game.game_status, GameStatus::NotStarted) {
                        panic!("Game is already in progress or finished");
                    }

                    let msg_id = msg::send(
                        game.wordle_program_id,
                        wordle_io::Action::StartGame {
                            user: msg::source(),
                        },
                        0,
                    )
                    .expect("Error in sending message to Wordle program");

                    game.session_status = SessionStatus::MessageSent;
                    game.msg_ids = (msg_id, msg::id());
                    debug!("HANDLE: WAIT");
                    exec::wait();
                }
                Action::CheckWord(word) => {
                    if word.len() != 5 {
                        panic!("Word must be exactly 5 characters long");
                    }
                    if !word.chars().all(|c| c.is_ascii_lowercase()) {
                        panic!("Word must contain only lowercase letters");
                    }

                    let GameStatus::InProgress { attempts, .. } = game.game_status else {
                        panic!("Game is not in progress");
                    };

                    if attempts >= MAX_ATTEMPTS {
                        panic!("Maximum attempts reached");
                    }

                    let msg_id = msg::send(
                        game.wordle_program_id,
                        wordle_io::Action::CheckWord {
                            user: msg::source(),
                            word: word.clone(),
                        },
                        0,
                    )
                    .expect("Error in sending message to Wordle program");

                    game.session_status = SessionStatus::MessageSent;
                    game.msg_ids = (msg_id, msg::id());
                    debug!("HANDLE: WAIT");
                    exec::wait();
                }
                Action::CheckGameStatus => {
                    debug!("CHECKGAME| {:?}", game.game_status);
                    if let GameStatus::InProgress { start_time, .. } = game.game_status {
                        let current_block = exec::block_height() as u64;
                        debug!("CHECKGAME||| {:?}", game.game_status);
                        if current_block >= start_time + GAME_DURATION_BLOCKS as u64 {
                            game.game_status = GameStatus::GameOver(GameResult::TimeOut);
                            debug!("CHECKGAME|||| {:?}", game.game_status);
                            msg::reply(Event::GameOver(GameResult::TimeOut), 0)
                                .expect("Error in sending reply");
                        }
                    }
                }
            }
        }
        SessionStatus::MessageSent => {
            debug!("HANDLE: SessionStatus::MessageSent");
            if msg::id() == game.msg_ids.1 {
                debug!("HANDLE: No response was received");
                msg::reply(Event::NoReplyReceived, 0).expect("Error in sending a reply");
                debug!("HANDLE: SessionStatus::Waiting");
                game.session_status = SessionStatus::Waiting;
            } else {
                debug!("HANDLE: Event::MessageAlreadySent");
                msg::reply(Event::MessageAlreadySent, 0).expect("Error in sending a reply");
            }
        }
        SessionStatus::ReplyReceived(reply_event) => {
            debug!("HANDLE: SessionStatus::ReplyReceived");
            debug!("REPLY_EVENT: {:?}", reply_event);
            match reply_event {
                Event::GameStarted => {
                    debug!("HANDLE: Event::GameStarted");
                    // Schedule game status check
                    let current_block = exec::block_height() as u64;
                    //exec::system_reserve_gas(10_000_000_000);
                    msg::send_delayed(
                        exec::program_id(),
                        Action::CheckGameStatus,
                        0,
                        GAME_DURATION_BLOCKS,
                    )
                    .expect("Error in sending delayed message");

                    game.game_status = GameStatus::InProgress {
                        attempts: 0,
                        start_time: current_block,
                    };
                    debug!("Game status: {:?}", game.game_status);
                    msg::reply(Event::GameStarted, 0).expect("Error in sending reply");
                }
                Event::WordChecked {
                    correct_positions,
                    contained_in_word,
                    attempts_left,
                } => {
                    let GameStatus::InProgress {
                        ref mut attempts, ..
                    } = game.game_status
                    else {
                        panic!("Game is not in progress");
                    };

                    let word = game
                        .previous_guesses
                        .last()
                        .map(|g| g.word.clone())
                        .unwrap_or_default();
                    game.previous_guesses.push(GuessResult {
                        word,
                        correct_positions: correct_positions.to_vec(),
                        contained_in_word: contained_in_word.to_vec(),
                    });

                    *attempts += 1;

                    if correct_positions.len() == 5 {
                        game.game_status = GameStatus::GameOver(GameResult::Win);
                        debug!("GAME_WIN: {:?}", game.game_status);
                        msg::reply(Event::GameOver(GameResult::Win), 0)
                            .expect("Error in sending reply");
                    } else if *attempts >= MAX_ATTEMPTS {
                        game.game_status = GameStatus::GameOver(GameResult::Lose);
                        debug!("GAME_loSE: {:?}", game.game_status);
                        msg::reply(Event::GameOver(GameResult::Lose), 0)
                            .expect("Error in sending reply");
                    } else {
                        msg::reply(
                            Event::WordChecked {
                                correct_positions: correct_positions.to_vec(),
                                contained_in_word: contained_in_word.to_vec(),
                                attempts_left: MAX_ATTEMPTS - *attempts,
                            },
                            0,
                        )
                        .expect("Error in sending reply");
                    }
                }
                _ => {}
            }
            debug!("HANDLE: SessionStatus::Waiting");
            game.session_status = SessionStatus::Waiting;
        }
    }
    debug!("HANDLE: END");
}

#[no_mangle]
extern "C" fn handle_reply() {
    debug!("HANDLE_REPLY");
    let reply_to = msg::reply_to().expect("Failed to get reply_to message id");
    let game = unsafe { GAME.as_mut().expect("Game is not initialized") };

    if reply_to == game.msg_ids.0 && matches!(game.session_status, SessionStatus::MessageSent) {
        let reply: wordle_io::Event = msg::load().expect("Failed to decode reply");
        let reply_event = match reply {
            wordle_io::Event::GameStarted { user: _ } => Event::GameStarted,
            wordle_io::Event::WordChecked {
                user: _,
                correct_positions,
                contained_in_word,
            } => Event::WordChecked {
                correct_positions,
                contained_in_word,
                attempts_left: 0,
            },
        };
        debug!(
            "HANDLE_REPLY: SessionStatus::ReplyReceived {:?}",
            reply_event
        );
        game.session_status = SessionStatus::ReplyReceived(reply_event);
        let original_message_id = game.msg_ids.1;
        debug!("HANDLE: WAKE");
        exec::wake(original_message_id).expect("Failed to wake message");
    }
}

#[no_mangle]
extern "C" fn state() {
    let game = unsafe { GAME.as_ref().expect("Game is not initialized") };
    let state = State {
        wordle_program_id: game.wordle_program_id,
        game_status: game.game_status.clone(),
        previous_guesses: game.previous_guesses.clone(),
    };
    msg::reply(state, 0).expect("Failed to share state");
}

#[cfg(test)]
mod tests;
