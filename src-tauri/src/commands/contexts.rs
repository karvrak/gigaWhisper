//! Context Commands
//!
//! Tauri commands for managing transcription contexts.

use crate::config::TranscriptionContext;
use crate::shortcuts;
use crate::AppState;

/// Get all transcription contexts
#[tauri::command]
pub async fn get_contexts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TranscriptionContext>, String> {
    let config = state.config.read();
    Ok(config.contexts.contexts.clone())
}

/// Save (create or update) a transcription context
#[tauri::command]
pub async fn save_context(
    app: tauri::AppHandle,
    context: TranscriptionContext,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut config = state.config.write();

        // Non-default contexts require premium
        if context.id != "default" && !config.premium.is_premium {
            return Err(
                "Premium license required to create or edit non-default contexts".to_string(),
            );
        }

        // Find existing context by ID or add new one
        if let Some(existing) = config
            .contexts
            .contexts
            .iter_mut()
            .find(|c| c.id == context.id)
        {
            *existing = context;
        } else {
            config.contexts.contexts.push(context);
        }

        config.save().map_err(|e| e.to_string())?;
    }

    // Re-register shortcuts to pick up new/changed context shortcuts
    if let Err(e) = shortcuts::update_shortcuts(&app) {
        tracing::warn!("Failed to update shortcuts after context save: {}", e);
    }

    Ok(())
}

/// Delete a transcription context
#[tauri::command]
pub async fn delete_context(
    app: tauri::AppHandle,
    context_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut config = state.config.write();

        // Don't allow deleting the default context
        if context_id == "default" {
            return Err("Cannot delete the default context".to_string());
        }

        config.contexts.contexts.retain(|c| c.id != context_id);

        // If active context was deleted, switch to default
        if config.contexts.active_context == context_id {
            config.contexts.active_context = "default".to_string();
        }

        config.save().map_err(|e| e.to_string())?;
    }

    // Re-register shortcuts to remove deleted context shortcut
    if let Err(e) = shortcuts::update_shortcuts(&app) {
        tracing::warn!("Failed to update shortcuts after context delete: {}", e);
    }

    Ok(())
}

/// Set the active context
#[tauri::command]
pub async fn set_active_context(
    context_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.write();

    // Verify context exists
    if !config.contexts.contexts.iter().any(|c| c.id == context_id) {
        return Err(format!("Context '{}' not found", context_id));
    }

    config.contexts.active_context = context_id;
    config.save().map_err(|e| e.to_string())?;
    Ok(())
}
