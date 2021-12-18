mod start;
mod manager_keyboard;
mod redeem;
mod points;
mod admin;
mod dialogues;

use start::*;
use admin::*;
use redeem::*;
use points::*;
use manager_keyboard::*;
use teloxide::{prelude::*, utils::command::BotCommand, dispatching::dialogue::{SqliteStorage, serializer::Json, Storage}};

use dialogues::*;

pub type Context = UpdateWithCx<AutoSend<Bot>, Message>;
type StorageError = <SqliteStorage<Json> as Storage<Dialogue>>::Error;
pub type RaffleDialogueContext = DialogueWithCx<AutoSend<Bot>, Message, Dialogue, StorageError>;

pub use dialogues::Dialogue;

#[derive(BotCommand)]
#[command(
    rename="lowercase",
    parse_with="split"
)]
pub enum Command {
    #[command()]
    Start(StartData),
    StartRaffle,
    EndRaffle,
    Join(StartData),
    Leave,
    Redeem(String),
    GenerateCode(String),
    Points,
    ManagerKeyboard,
}

pub async fn handle_action(ctx: Context, command: Command) -> TransitionOut<Dialogue> {
    match command {
        Command::Start(data) => start_cmd(data.referrer, ctx).await,
        Command::GenerateCode(uses) => generate_code_cmd(uses, ctx).await,
        Command::Join(data) => join_cmd(data.referrer, ctx).await,
        Command::Leave => leave_cmd(ctx).await,
        Command::Redeem(data) => redeem_code_cmd(data, ctx).await,
        Command::Points => get_points_cdm(ctx).await,
        Command::ManagerKeyboard => send_manager_keyboard_command(ctx).await,

        Command::StartRaffle => create_raffle(ctx).await,
        Command::EndRaffle => end_raffle(ctx).await
    }
}