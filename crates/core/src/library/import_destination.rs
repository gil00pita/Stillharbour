//! Catalog-scoped preference only. Saving a hint never grants filesystem
//! authority, retargets an immutable plan, or changes base/browsing settings.
use super::{Library, model::ImportDestinationPreference};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
};
use rusqlite::{Connection, params};

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE import_destination_preference(singleton INTEGER PRIMARY KEY CHECK(singleton=1),root_id TEXT REFERENCES roots(id) ON DELETE SET NULL); INSERT INTO import_destination_preference VALUES(1,NULL);")?;
    Ok(())
}
fn load_db(db: &Connection) -> Result<ImportDestinationPreference> {
    let catalog_id: String = db.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |row| row.get(0),
    )?;
    let root_id: Option<String> = db.query_row(
        "SELECT root_id FROM import_destination_preference WHERE singleton=1",
        [],
        |row| row.get(0),
    )?;
    if let Some(id) = &root_id {
        crate::operations::validate_id(id)?;
    }
    Ok(ImportDestinationPreference {
        catalog_id,
        root_id,
    })
}
pub(crate) fn load(catalog: &Catalog) -> Result<ImportDestinationPreference> {
    catalog.call(|db| load_db(db))
}
impl Library {
    pub fn set_import_destination(
        &self,
        catalog_id: String,
        root_id: Option<String>,
    ) -> Result<ImportDestinationPreference> {
        crate::operations::validate_id(&catalog_id)?;
        if let Some(id) = &root_id {
            self.validate_import_destination_preference(&catalog_id, id.clone())?;
        }
        self.catalog.call(move |db| {
            let tx = db.transaction()?;
            let current = load_db(&tx)?;
            if current.catalog_id != catalog_id {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "The open catalog changed. Review its import destination again.",
                ));
            }
            if let Some(id) = &root_id {
                let valid: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM roots WHERE id=? AND mode='managed')",
                    [id],
                    |row| row.get(0),
                )?;
                if !valid {
                    return Err(AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose a managed destination in this catalog; its grant is unchanged.",
                    ));
                }
            }
            tx.execute(
                "UPDATE import_destination_preference SET root_id=? WHERE singleton=1",
                params![root_id],
            )?;
            let result = load_db(&tx)?;
            tx.commit()?;
            Ok(result)
        })
    }
}
