use anyhow::Result;
use sled::Db;

use crate::models::RepoState;

pub struct StateDb {
    db: Db,
}

impl StateDb {
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn save_repo_state(&self, state: &RepoState) -> Result<()> {
        let key = state.name.as_bytes();
        let value = serde_json::to_vec(state)?;
        self.db.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get_repo_state(&self, name: &str) -> Result<Option<RepoState>> {
        if let Some(value) = self.db.get(name.as_bytes())? {
            let state: RepoState = serde_json::from_slice(&value)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    pub fn list_all_states(&self) -> Result<Vec<RepoState>> {
        let mut states = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            let state: RepoState = serde_json::from_slice(&value)?;
            states.push(state);
        }
        Ok(states)
    }
}
