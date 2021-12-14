use std::{str::FromStr};
use serde::{Serialize, Deserialize};

use teloxide::prelude::*;
use userdb::db::UserID;
use crate::{commands::Context, dialogue::Dialogue};


#[derive(Serialize, Deserialize)]
pub struct StartData {
    pub referrer: Option<UserID>
}

impl FromStr for StartData {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StartData {
            referrer: match s.parse::<i64>() {
                Ok(num) => Some(num),
                Err(_) => None
            }
        })
    }
}


pub async fn start_cmd(
    referrer: Option<UserID>,
    cx: Context) -> TransitionOut<Dialogue> {
    cx.answer("Hello!").await?;
    cx.answer(match referrer {
        Some(id) => {
            format!("You were referred by someone, right? {}", id)
        }
        None => {
            "No one referred you?".to_owned()
        }
    }).await?;
    cx.answer("https://t.me/crax_testing_bot?start=1234").await?;
    exit()
}