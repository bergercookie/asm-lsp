use std::path::PathBuf;

use asm_lsp::RootConfig;

use anyhow::Result;
use serde_json::to_writer_pretty;

use crate::Schema;

impl Schema {
    pub fn run() -> Result<()> {
        let schema = schemars::schema_for!(RootConfig);
        let xtask_path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let schema_path = xtask_path
            .parent()
            .unwrap()
            .join("asm-lsp_config_schema.json");
        let mut file = std::fs::File::create(schema_path)?;
        Ok(to_writer_pretty(&mut file, &schema)?)
    }
}
