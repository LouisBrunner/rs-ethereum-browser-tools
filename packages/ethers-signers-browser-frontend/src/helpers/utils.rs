use crate::{hooks::use_ws::WSState, ws::WebsocketStatus};

pub(crate) fn get_ws_status(ws: WSState) -> String {
    match ws.status {
        None => "connecting...".to_owned(),
        Some(status) => {
            match status {
                Ok(status) => match status {
                    WebsocketStatus::Connected => "connected".to_owned(),
                    WebsocketStatus::Pending => "connecting...".to_owned(),
                    WebsocketStatus::Disconnected(_e) => {
                        format!("disconnected (check that the command is still running), reconnecting...")
                    }
                    WebsocketStatus::Error(e) => format!("error ({})", e),
                },
                Err(e) => format!("error ({})", e),
            }
        }
    }
}
