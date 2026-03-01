mod ansi;
#[path = "bootstrap/bootstrap_app.rs"]
mod bootstrap_app;
#[path = "bootstrap/bootstrap_config.rs"]
mod bootstrap_config;
#[path = "bootstrap/bootstrap_discovery.rs"]
mod bootstrap_discovery;
mod terminal;
#[macro_use]
mod shared;
#[path = "app/mod.rs"]
mod app;
#[path = "commands/catalog.rs"]
mod commands;
#[path = "commands/help.rs"]
mod commands_hints;
#[path = "commands/palette.rs"]
mod commands_palette;
#[path = "dialogs/dialogs.rs"]
mod dialogs;
#[path = "dialogs/dialogs_confirm.rs"]
mod dialogs_confirm;
#[path = "dialogs/dialogs_create_key.rs"]
mod dialogs_create_key;
#[path = "dialogs/dialogs_create_setup.rs"]
mod dialogs_create_setup;
#[path = "dialogs/dialogs_delete.rs"]
mod dialogs_delete;
#[path = "dialogs/dialogs_edit.rs"]
mod dialogs_edit;
#[path = "dialogs/dialogs_launch.rs"]
mod dialogs_launch;
#[path = "dialogs/dialogs_merge.rs"]
mod dialogs_merge;
#[path = "dialogs/dialogs_projects_crud.rs"]
mod dialogs_projects_crud;
#[path = "dialogs/dialogs_projects_defaults.rs"]
mod dialogs_projects_defaults;
#[path = "dialogs/dialogs_projects_key.rs"]
mod dialogs_projects_key;
#[path = "dialogs/dialogs_projects_reorder.rs"]
mod dialogs_projects_reorder;
#[path = "dialogs/dialogs_projects_state.rs"]
mod dialogs_projects_state;
#[path = "dialogs/dialogs_settings.rs"]
mod dialogs_settings;
#[path = "dialogs/state.rs"]
mod dialogs_state;
#[path = "dialogs/dialogs_stop.rs"]
mod dialogs_stop;
#[path = "dialogs/dialogs_update_from_base.rs"]
mod dialogs_update_from_base;
#[path = "logging/logging_frame.rs"]
mod logging_frame;
#[path = "logging/logging_input.rs"]
mod logging_input;
#[path = "logging/logging_state.rs"]
mod logging_state;
mod msg;
mod runner;
mod selection;
pub use runner::{run_with_debug_record, run_with_event_log};
mod replay;
pub use replay::{ReplayOptions, emit_replay_fixture, replay_debug_record};
mod text;
#[path = "update/update.rs"]
mod update;
#[path = "update/update_core.rs"]
mod update_core;
#[path = "update/update_input_interactive.rs"]
mod update_input_interactive;
#[path = "update/update_input_interactive_clipboard.rs"]
mod update_input_interactive_clipboard;
#[path = "update/update_input_interactive_send.rs"]
mod update_input_interactive_send;
#[path = "update/update_input_key_events.rs"]
mod update_input_key_events;
#[path = "update/update_input_keybinding.rs"]
mod update_input_keybinding;
#[path = "update/update_input_mouse.rs"]
mod update_input_mouse;
#[path = "update/update_lifecycle_create.rs"]
mod update_lifecycle_create;
#[path = "update/update_lifecycle_start.rs"]
mod update_lifecycle_start;
#[path = "update/update_lifecycle_stop.rs"]
mod update_lifecycle_stop;
#[path = "update/update_lifecycle_workspace_completion.rs"]
mod update_lifecycle_workspace_completion;
#[path = "update/update_lifecycle_workspace_refresh.rs"]
mod update_lifecycle_workspace_refresh;
#[path = "update/update_navigation_commands.rs"]
mod update_navigation_commands;
#[path = "update/update_navigation_palette.rs"]
mod update_navigation_palette;
#[path = "update/update_navigation_preview.rs"]
mod update_navigation_preview;
#[path = "update/update_polling_capture_cursor.rs"]
mod update_polling_capture_cursor;
#[path = "update/update_polling_capture_dispatch.rs"]
mod update_polling_capture_dispatch;
#[path = "update/update_polling_capture_live.rs"]
mod update_polling_capture_live;
#[path = "update/update_polling_capture_task.rs"]
mod update_polling_capture_task;
#[path = "update/update_polling_capture_workspace.rs"]
mod update_polling_capture_workspace;
#[path = "update/update_polling_state.rs"]
mod update_polling_state;
#[path = "update/prelude.rs"]
mod update_prelude;
#[path = "update/update_tick.rs"]
mod update_tick;
#[path = "view/view.rs"]
mod view;
#[path = "view/view_chrome_divider.rs"]
mod view_chrome_divider;
#[path = "view/view_chrome_header.rs"]
mod view_chrome_header;
#[path = "view/view_chrome_shared.rs"]
mod view_chrome_shared;
#[path = "view/view_chrome_sidebar.rs"]
mod view_chrome_sidebar;
#[path = "view/view_layout.rs"]
mod view_layout;
#[path = "view/view_overlays_confirm.rs"]
mod view_overlays_confirm;
#[path = "view/view_overlays_create.rs"]
mod view_overlays_create;
#[path = "view/view_overlays_edit.rs"]
mod view_overlays_edit;
#[path = "view/view_overlays_help.rs"]
mod view_overlays_help;
#[path = "view/view_overlays_projects.rs"]
mod view_overlays_projects;
#[path = "view/view_overlays_settings.rs"]
mod view_overlays_settings;
#[path = "view/view_overlays_workspace_delete.rs"]
mod view_overlays_workspace_delete;
#[path = "view/view_overlays_workspace_launch.rs"]
mod view_overlays_workspace_launch;
#[path = "view/view_overlays_workspace_merge.rs"]
mod view_overlays_workspace_merge;
#[path = "view/view_overlays_workspace_stop.rs"]
mod view_overlays_workspace_stop;
#[path = "view/view_overlays_workspace_update.rs"]
mod view_overlays_workspace_update;
#[path = "view/prelude.rs"]
mod view_prelude;
#[path = "view/view_preview.rs"]
mod view_preview;
#[path = "view/view_preview_content.rs"]
mod view_preview_content;
#[path = "view/view_preview_shell.rs"]
mod view_preview_shell;
#[path = "view/view_selection_interaction.rs"]
mod view_selection_interaction;
#[path = "view/view_selection_logging.rs"]
mod view_selection_logging;
#[path = "view/view_selection_mapping.rs"]
mod view_selection_mapping;
#[path = "view/view_status.rs"]
mod view_status;

include!("model.rs");

#[cfg(test)]
mod tests;
