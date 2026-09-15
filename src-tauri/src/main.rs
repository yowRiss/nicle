mod error;
mod workspace;
mod filesystem;
mod search;
mod terminal;
mod runner;
mod processes;
mod preview;
mod settings;
mod benchmark;
mod git;
pub mod plugins;
use tauri::Manager;
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(workspace::Workspace::default())
        .manage(search::SearchState::default())
        .manage(terminal::TerminalState::default())
        .manage(runner::RunnerState::default())
        .manage(preview::PreviewState::default())
        .manage(plugins::PluginState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let plugin_state = app.state::<plugins::PluginState>();
            plugin_state.0.set_app_handle(handle.clone());
            let plugin_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(installed) = plugins::storage::load_installed(&plugin_handle) {
                    for item in installed {
                        if item.start_with_nicle {
                            let state = plugin_handle.state::<plugins::PluginState>();
                            let _ = state.0.start_app(&plugin_handle, &item.id).await;
                        }
                    }
                }
            });
            if std::env::args().any(|a| a == "--benchmark") {
                benchmark::run_benchmark_suite(handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace::open_workspace,
            workspace::choose_folder,
            workspace::reveal_in_file_manager,
            filesystem::list_directory,
            filesystem::read_file,
            filesystem::save_file,
            filesystem::create_entry,
            filesystem::rename_entry,
            filesystem::delete_entry,
            search::search_files,
            search::cancel_search,
            terminal::terminal_start,
            terminal::terminal_ack,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_stop,
            runner::run_file,
            runner::stop_runner,
            preview::start_preview,
            preview::stop_preview,
            settings::load_settings,
            settings::save_settings,
            plugins::plugins_get_catalog,
            plugins::plugins_get_installed,
            plugins::plugins_check_runtime,
            plugins::plugins_get_status,
            plugins::plugins_install,
            plugins::plugins_start,
            plugins::plugins_stop,
            plugins::plugins_restart,
            plugins::plugins_update,
            plugins::plugins_uninstall,
            plugins::plugins_save_settings,
            plugins::plugins_get_logs,
            plugins::plugins_clear_logs,
            plugins::plugins_open_dashboard,
            plugins::plugins_check_bun_runtime,
            plugins::plugins_link_dev,
            plugins::plugins_unlink_dev,
            plugins::plugins_get_commands,
            plugins::plugins_scaffold,
            git::git_status,
            git::git_diff,
            git::git_stage,
            git::git_unstage,
            git::git_discard,
            git::git_commit,
            git::git_push,
            git::git_pull,
            git::git_branches,
            git::git_checkout,
            git::git_create_branch,
            git::git_init
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                handle.state::<search::SearchState>().cancel();
                handle.state::<terminal::TerminalState>().stop();
                handle.state::<runner::RunnerState>().stop();
                handle.state::<preview::PreviewState>().stop();
                handle.state::<plugins::PluginState>().stop_all();
            }
        }),
        Err(error) => eprintln!("Nicle: {error}"),
    }
}
