use serde::{Serialize, Deserialize};
use teloxide::{
    prelude::*,
    adaptors::AutoSend, dispatching::dialogue::{SqliteStorage,serializer::Json, Storage}, prelude::DialogueWithCx,
    types::Message, Bot,
    macros::{Transition}
};

#[derive(Transition, Serialize, Deserialize, derive_more::From)]
pub enum Dialogue {
    Begin(NoData),
}

#[derive(Serialize, Deserialize)]
pub struct NoData;

impl Default for Dialogue {
    fn default() -> Self {
        Dialogue::Begin(NoData)
    }
}

#[teloxide(subtransition)]
async fn no_data(
    state: NoData,
    cx: TransitionIn<AutoSend<Bot>>,
    _ans: String
) -> TransitionOut<Dialogue> {
    cx.answer("Is this some text?").await?;
    next(state)
}

type StorageError = <SqliteStorage<Json> as Storage<Dialogue>>::Error;
pub type RaffleDialogueContext = DialogueWithCx<AutoSend<Bot>, Message, Dialogue, StorageError>;