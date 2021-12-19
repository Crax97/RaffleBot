mod start;
mod redeem;
mod points;
mod admin;
mod dialogues;

use start::*;
use admin::*;
use redeem::*;
use points::*;
use teloxide::{prelude::*, utils::command::BotCommand, adaptors::CacheMe};

pub type RaffleBot = AutoSend<CacheMe<Bot>>;
pub type Context = UpdateWithCx<RaffleBot, Message>;

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
    Stats,
    Join(StartData),
    Leave,
    Redeem(String),
    GenerateCode(String),
    Points,
}

pub async fn handle_action(ctx: Context, command: Command) -> TransitionOut<Dialogue> {
    match command {
        Command::Start(data) => start_cmd(data.referrer, ctx).await,
        Command::GenerateCode(uses) => generate_code_cmd(uses, ctx).await,
        Command::Join(data) => join_cmd(data.referrer, ctx).await,
        Command::Leave => leave_cmd(ctx).await,
        Command::Redeem(data) => redeem_code_cmd(data, ctx).await,
        Command::Points => get_points_cdm(ctx).await,
        Command::Stats => stats(ctx).await,

        Command::StartRaffle => create_raffle(ctx).await,
        Command::EndRaffle => end_raffle(ctx).await
    }
}