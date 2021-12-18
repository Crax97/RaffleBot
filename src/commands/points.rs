use teloxide::prelude::*;
use userdb::db::RaffleDB;
use crate::commands::Context;
use crate::utils::*;

use super::dialogues::*;

pub async fn get_points_cdm(
    ctx: Context) -> TransitionOut<Dialogue> {
    let user_id = match ctx.update.from() {
        Some(user) => user.id,
        None => {
            return next(Dialogue::Begin(NoData));
        }
    };
    if is_admin(user_id) {
        // show admin keyboard
        ctx.answer("You can't really have points as an admin...")
        .await?;
        return next(Dialogue::Begin(NoData));
    }
    let partecipant = {
        let raffle_db = crate::DB_INSTANCE.lock().await;
        raffle_db.get_partecipant(user_id)
    };
    let partecipant = match partecipant {
        Ok(part_maybe) => match part_maybe {
            Some(part) => part,
            None => {
                ctx.answer("Sorry, you must be a member of the raffle in order to get points.")
                .await?;
                return next(Dialogue::Begin(NoData));
            }
        },
        Err(e) => {
            on_error(e, &ctx.update, &ctx.requester, "on points").await;
            return next(Dialogue::Begin(NoData));
        }
    };

    ctx.answer(format!("Sure! You have {} points.", partecipant.priority))
    .await?;
    next(Dialogue::Begin(NoData))
}