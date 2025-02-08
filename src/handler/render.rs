use htmlize::escape_text;
use telegram_types::bot::types::{
    InlineKeyboardButton, InlineKeyboardButtonPressed, InlineKeyboardMarkup,
};

use crate::controller::EvalPageData;

pub(super) fn render_page_data(data: EvalPageData) -> (String, InlineKeyboardMarkup) {
    // TODO: trim into 4096 characters
    let text = format!(
        "<b>{}</b>\n<blockquote expandable><code>{}</code></blockquote>",
        escape_text(data.title),
        escape_text(data.content)
    );
    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![
            InlineKeyboardButton {
                text: match (data.has_fatal_error, data.has_error, data.has_warning) {
                    (true, _, _) => "👻",
                    (false, true, _) => "❌️",
                    (false, false, true) => "⚠️",
                    (false, false, false) => "✅",
                }
                .into(),
                pressed: InlineKeyboardButtonPressed::CallbackData(format!(
                    "v1:state:build:{}",
                    data.revision_id
                )),
            },
            InlineKeyboardButton {
                text: match data.revision {
                    0 => "📃".into(),
                    rev => format!("📃{rev}"),
                },
                pressed: InlineKeyboardButtonPressed::CallbackData(format!(
                    "v1:state:output:{}",
                    data.revision_id
                )),
            },
            InlineKeyboardButton {
                text: "🔗".into(),
                pressed: if let Some(perma_link) = data.perma_link {
                    InlineKeyboardButtonPressed::Url(perma_link)
                } else {
                    InlineKeyboardButtonPressed::CallbackData(format!(
                        "v1:genlink:{}",
                        data.revision_id
                    ))
                },
            },
            InlineKeyboardButton {
                text: "🗑️".into(),
                pressed: InlineKeyboardButtonPressed::CallbackData(format!(
                    "v1:del:{}",
                    data.revision_id
                )),
            },
        ]],
    };
    (text, keyboard)
}
