use std::collections::{HashMap, HashSet};

use futures_util::{StreamExt, stream};

use crate::drive::client::DriveClient;
use crate::error::AppError;

/// Shown for files whose parent folder the account cannot see, which is how Drive presents
/// files someone else shared.
pub const SHARED_WITH_ME: &str = "Shared with me";

const LOOKUP_CONCURRENCY: usize = 8;

/// Session cache of Drive folder names, fetched on demand.
#[derive(Default)]
pub struct FolderNames {
    names: tokio::sync::Mutex<HashMap<String, String>>,
}

impl FolderNames {
    /// Returns names for the given folder IDs. Folders that could not be looked up because of a
    /// transient error are left out and retried on the next call.
    pub async fn resolve(
        &self,
        drive: &DriveClient,
        folder_ids: &[String],
    ) -> HashMap<String, String> {
        let missing: HashSet<String> = {
            let names = self.names.lock().await;
            folder_ids
                .iter()
                .filter(|id| !names.contains_key(*id))
                .cloned()
                .collect()
        };

        let fetched: Vec<(String, Option<String>)> = stream::iter(missing)
            .map(|id| async move {
                let name = match drive.folder(&id).await {
                    Ok(folder) => Some(folder.name),
                    Err(AppError::NotFound | AppError::Forbidden(_)) => {
                        Some(SHARED_WITH_ME.to_owned())
                    }
                    Err(_) => None,
                };
                (id, name)
            })
            .buffer_unordered(LOOKUP_CONCURRENCY)
            .collect()
            .await;

        let mut names = self.names.lock().await;
        for (id, name) in fetched {
            if let Some(name) = name {
                names.insert(id, name);
            }
        }
        folder_ids
            .iter()
            .filter_map(|id| names.get(id).map(|name| (id.clone(), name.clone())))
            .collect()
    }

    pub async fn clear(&self) {
        self.names.lock().await.clear();
    }
}
