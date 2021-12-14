pub mod start;

use start::*;
use teloxide::{prelude::*, utils::command::BotCommand};

use crate::dialogue::Dialogue;

pub type Context = UpdateWithCx<AutoSend<Bot>, Message>;

#[derive(BotCommand)]
#[command(
    rename="lowercase",
    parse_with="split"
)]
pub enum Command {
    #[command()]
    Start(StartData)
}

pub async fn handle_action(ctx: Context, command: Command) -> TransitionOut<Dialogue> {
    match command {
        Command::Start(data) => start_cmd(data.referrer, ctx).await
    }
}