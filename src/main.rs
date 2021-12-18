extern crate serde;
extern crate serde_json;
extern crate userdb;
extern crate lazy_static;
extern crate teloxide;
extern crate log;
extern crate tokio;
extern crate tokio_stream;
extern crate async_mutex;

mod commands;
mod utils;

use commands::*;
use userdb::db_instances::sqlite_instance::SQLiteInstance;

use async_mutex::Mutex;
use teloxide::{prelude::*, 
    dispatching::dialogue::{SqliteStorage, serializer::Json},
    utils::command::BotCommand
    };
use lazy_static::lazy_static;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}

lazy_static! {
    pub static ref DB_INSTANCE : Mutex<SQLiteInstance> = Mutex::new(SQLiteInstance::create("raffle_db.db")
                                                .expect("Failure to open userdb"));
}


pub async fn handle_dialogue(ctx: UpdateWithCx<AutoSend<Bot>, Message>, dialogue: Dialogue) 
    -> TransitionOut<Dialogue> {
    let text =match ctx.update.text() {
        Some(s) => Some(s.to_owned()),
        None => None,
    };
    match text {
        Some(ans) => {
            let me = ctx.requester.get_me().await.unwrap();
            let name = me.user.username.expect("Must have an username");
            let cmd = Command::parse(ans.as_str(), name);
            if cmd.is_ok() {
                handle_action(ctx, cmd.unwrap()).await
            } else {
                dialogue.react(ctx, ans).await
            }
        }
        None => {
            dialogue.react(ctx, String::new()).await
        },
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    teloxide::enable_logging!();
    
    let bot = Bot::from_env()
        .auto_send();
    let me = bot
        .get_me()
        .await
        .expect("Please set the TELOXIDE_TOKEN env variable");
    let my_name = me.user.username.expect("Bots must have an username");
    log::info!("Got bot with username {}", my_name);

    Dispatcher::new(bot)
        .setup_ctrlc_handler()
        .messages_handler(DialogueDispatcher::with_storage(|DialogueWithCx{cx,dialogue} : RaffleDialogueContext| async move {
            let dialogue = dialogue.expect("No dialogue");
            handle_dialogue(cx, dialogue).await.expect("Something bad happened")
        },
        SqliteStorage::open("dialogues.db", Json).await.unwrap()
        ))
        .dispatch() 
        .await;
        Ok(())
}
  