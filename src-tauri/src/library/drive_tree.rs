//! The Drive folder chosen in Settings, with every folder inside it. Drive only lists a folder's
//! direct children, so the tree is walked one level at a time and kept for a few minutes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::sources::DriveFolderChoice;
use crate::drive::client::DriveClient;
use crate::error::AppResult;

/// Walked again after this, so new subfolders show up without restarting.
const TREE_LIFETIME: Duration = Duration::from_secs(5 * 60);
const MAX_DEPTH: usize = 20;
const MAX_FOLDERS: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub struct DriveTree {
    pub root_id: String,
    /// Name of every folder in the tree, the chosen one included.
    pub folders: HashMap<String, String>,
}

impl DriveTree {
    pub fn folder_ids(&self) -> Vec<String> {
        self.folders.keys().cloned().collect()
    }
}

pub async fn walk(drive: &DriveClient, root: &DriveFolderChoice) -> AppResult<DriveTree> {
    let mut folders = HashMap::from([(root.id.clone(), root.name.clone())]);
    let mut level = vec![root.id.clone()];
    for _ in 0..MAX_DEPTH {
        if level.is_empty() || folders.len() >= MAX_FOLDERS {
            break;
        }
        level = drive
            .list_folders(&level)
            .await?
            .into_iter()
            // A folder can sit in two places of the tree; it is walked once.
            .filter_map(|folder| {
                folders
                    .insert(folder.id.clone(), folder.name)
                    .is_none()
                    .then_some(folder.id)
            })
            .collect();
    }
    Ok(DriveTree {
        root_id: root.id.clone(),
        folders,
    })
}

#[derive(Default)]
pub struct DriveTreeCache {
    tree: tokio::sync::Mutex<Option<(Instant, DriveTree)>>,
}

impl DriveTreeCache {
    /// The tree under `root`, walked again when it is old. Callers wait for one walk instead of
    /// each starting their own.
    pub async fn get(&self, drive: &DriveClient, root: &DriveFolderChoice) -> AppResult<DriveTree> {
        let mut cached = self.tree.lock().await;
        if let Some((walked_at, tree)) = cached.as_ref()
            && tree.root_id == root.id
            && walked_at.elapsed() < TREE_LIFETIME
        {
            return Ok(tree.clone());
        }
        let tree = walk(drive, root).await?;
        *cached = Some((Instant::now(), tree.clone()));
        Ok(tree)
    }

    pub async fn clear(&self) {
        self.tree.lock().await.take();
    }
}
