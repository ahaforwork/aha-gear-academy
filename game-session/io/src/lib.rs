#![no_std]

use gmeta::{In, InOut, Metadata, Out};
use gstd::{prelude::*, ActorId, Debug};

pub struct GameSessionMetadata;

impl Metadata for GameSessionMetadata {
    type Init = In<InitGameSession>;
    type Handle = InOut<Action, Event>;
    type Reply = ();
    type Others = ();
    type Signal = ();
    type State = Out<State>;
}

#[derive(Encode, Decode, TypeInfo, Debug)]
pub struct InitGameSession {
    pub wordle_program_id: ActorId,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum Action {
    StartGame,
    CheckWord(String),
    CheckGameStatus,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum Event {
    GameStarted,
    WordChecked {
        correct_positions: Vec<u8>,
        contained_in_word: Vec<u8>,
        attempts_left: u8,
    },
    GameOver(GameResult),
    NoReplyReceived,
    MessageAlreadySent,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum GameResult {
    Win,
    Lose,
    TimeOut,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum GameStatus {
    NotStarted,
    InProgress { attempts: u8, start_time: u64 },
    GameOver(GameResult),
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub struct State {
    pub wordle_program_id: ActorId,
    pub game_status: GameStatus,
    pub previous_guesses: Vec<GuessResult>,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub struct GuessResult {
    pub word: String,
    pub correct_positions: Vec<u8>,
    pub contained_in_word: Vec<u8>,
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum SessionStatus {
    Waiting,
    MessageSent,
    ReplyReceived(Event),
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Waiting
    }
}
