use async_std::task;
use log::info;
use tauri::{api::dialog::FileDialogBuilder, command, State};

use crate::state::LocalState;

#[command]
pub fn load_backup(local_state: State<'_, LocalState>) -> Result<String, String> {
    if let Some(path) = FileDialogBuilder::new().pick_file() {
        info!("importing file: {:?}", path);

        task::block_on(async {
            info!("creating account");
            let t: Result<(u32, deltachat::context::Context), String> = local_state
                .add_account()
                .await
                .map_err(|e| format!("{:?}", e));

            let (id, ctx) = t?;

            info!("importing...");
            local_state
                .import(&ctx, id, path.as_path().into())
                .await
                .map_err(|e| format!("{:?}", e))?;

            Ok(id.to_string())
        })
    } else {
        Ok((-1).to_string())
    }
}
