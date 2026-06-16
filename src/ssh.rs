use std::fs;

use crate::db::Database;

pub fn ssh_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/.ssh", home)
}

pub fn authorized_keys_path() -> String {
    format!("{}/authorized_keys", ssh_dir())
}

pub async fn regenerate_authorized_keys(db: &Database) -> Result<(), String> {
    let all_keys = db.get_all_ssh_keys().await.map_err(|e| format!("DB error: {}", e))?;
    let script_path = format!("{}/gitpage-shell", ssh_dir());

    let mut output = String::new();

    for (key, user, repo) in &all_keys {
        let command = format!(
            "command=\"{}\" \"{}\" \"{}\"",
            script_path, user.username, repo.name
        );
        let restrictions = "no-port-forwarding,no-X11-forwarding,no-agent-forwarding";
        output.push_str(&format!(
            "{},{},{} {}\n",
            command, restrictions, key.public_key, key.name
        ));
    }

    let output_path = authorized_keys_path();
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    fs::write(&output_path, &output).map_err(|e| format!("Failed to write {}: {}", output_path, e))?;

    Ok(())
}
