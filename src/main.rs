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
use std::sync::Arc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use userdb::db_instances::sqlite_instance::SQLiteInstance;

use async_mutex::Mutex;
use teloxide::{prelude::*, 
    dispatching::dialogue::{SqliteStorage, serializer::Json, Storage},
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


pub async fn handle_dialogue(ctx: UpdateWithCx<RaffleBot, Message>, dialogue: Dialogue) 
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

async fn handle<S : Storage<Dialogue>>(upd: UpdateWithCx<RaffleBot, Message>, storage: Arc<S>)
    where <S as Storage<Dialogue>>::Error: std::fmt::Debug {
    let chat_id = upd.update.chat.id;
    let dialogue: Result<Option<Dialogue>, _> = storage.clone().get_dialogue(upd.update.chat.id).await;
    let dialogue = match dialogue {
        Ok(some_dialogue) => match some_dialogue {
            Some(d) => d,
            None => Dialogue::default()
        },
        Err(e) => {
            log::error!("While reading dialogue for {}: {:?}", upd.update.chat.id, e);
            Dialogue::default()
        }
    };

    let result: Result<(), _> = match handle_dialogue(upd, dialogue).await {
        Ok(DialogueStage::Next(stage)) => 
            storage.update_dialogue(chat_id, stage).await,
        
        Ok(DialogueStage::Exit) => {
            storage.remove_dialogue(chat_id).await
        }
        Err(e) => {
            log::error!("While executing dialogue handler: {0}", e.to_string());
            Ok(())
        }
    };

    match result {
        Ok(_) => {}
        Err(err) => {
            log::error!("While updating dialogue: {:?}", err);
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    teloxide::enable_logging!();
    
    let bot = Bot::from_env()
        .cache_me()
        .auto_send();
    let me = bot
        .get_me()
        .await
        .expect("Please set the TELOXIDE_TOKEN env variable");
    let my_name = me.user.username.expect("Bots must have an username");
    log::info!("Got bot with username {}", my_name);

    let storage : Arc<SqliteStorage<Json>> = SqliteStorage::open("dialogues.db", Json).await.expect("Could not open dialgoue storage");
    Dispatcher::new(bot)
        .setup_ctrlc_handler()
        .messages_handler(|rx: DispatcherHandlerRx<RaffleBot, Message>| async move {
            UnboundedReceiverStream::new(rx)
            .filter(|upd| {
                let is_private = upd.update.chat.is_private();
                async move {
                    is_private
                }
            })
            .map(move |upd| {
                (upd, storage.clone())
            })
            .for_each_concurrent(None, |(upd, storage)| async move {
                handle(upd, storage.clone()).await;
            }).await;
            
        })
        .dispatch()
        .await;
        Ok(())
}
  