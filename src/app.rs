use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

use crate::model::{
    ActiveModal, CaddyControlMethod, CaddyProxyStatus, FormState, ProxyConfig, Service,
    ServiceSource, View,
};

pub enum AppAction {
    Quit,
    SwitchView,
    MoveUp,
    MoveDown,
    JumpTop,
    JumpBottom,
    AddProxy,
    EditProxy,
    OpenBrowser,
    Refresh,
    CaddyMenu,
    Help,
    CloseModal,
    FormNextField,
    FormPrevField,
    FormConfirm,
    FormCharInput(char),
    FormBackspace,
    CaddyStart,
    CaddyStop,
    CaddyRestart,
    SelectItem(usize),
    None,
}

pub struct App {
    pub view: View,
    pub services: Vec<Service>,
    pub global_services: Vec<Service>,
    pub selected: usize,
    pub modal: ActiveModal,
    pub form: FormState,
    pub caddy_status: CaddyProxyStatus,
    pub caddy_control: Option<CaddyControlMethod>,
    pub caddy_selected: usize,
    pub compose_files: Vec<PathBuf>,
    pub docker_client: Option<bollard::Docker>,
    pub has_project: bool,
    pub active_domains: Vec<String>,
    pub status_message: Option<String>,
}

impl App {
    pub async fn new() -> Result<Self> {
        // 1. Connect to docker (may fail gracefully)
        let docker_client_result = crate::docker::client::connect().await;
        let (docker_client, caddy_status, caddy_control, global_services) =
            match docker_client_result {
                Ok(client) => {
                    let caddy_status =
                        crate::docker::containers::get_caddy_proxy_status(&client.docker)
                            .await
                            .unwrap_or(CaddyProxyStatus::Unknown);
                    let caddy_control =
                        Some(crate::docker::containers::detect_caddy_control_method());
                    let global =
                        crate::docker::containers::list_caddy_services(&client.docker)
                            .await
                            .unwrap_or_default();
                    (Some(client.docker), caddy_status, caddy_control, global)
                }
                Err(_) => (None, CaddyProxyStatus::Unknown, None, vec![]),
            };

        // 2. Discover compose files in cwd
        let cwd = std::env::current_dir()?;
        let compose_files =
            crate::compose::discovery::find_compose_files(&cwd).unwrap_or_default();
        let has_project = !compose_files.is_empty();

        // 3. Parse project services from compose files
        let mut services: Vec<Service> = Vec::new();
        for file in &compose_files {
            if let Ok(compose) = crate::compose::parser::parse_compose_file(file) {
                if let Ok((_, mut svc)) =
                    crate::compose::parser::extract_services(&compose, file)
                {
                    services.append(&mut svc);
                }
            }
        }

        // 4. Merge runtime status
        if let Some(ref docker) = docker_client {
            let _ =
                crate::docker::containers::merge_runtime_status(docker, &mut services).await;
        }

        // 5. Query caddy active domains
        let active_domains =
            crate::caddy::admin::get_active_domains().await.unwrap_or_default();

        // 6. Determine starting view
        let view = if has_project {
            View::Project
        } else {
            View::Global
        };

        Ok(App {
            view,
            services,
            global_services,
            selected: 0,
            modal: ActiveModal::None,
            form: FormState::default(),
            caddy_status,
            caddy_control,
            caddy_selected: 0,
            compose_files,
            docker_client,
            has_project,
            active_domains,
            status_message: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        let mut terminal = ratatui::Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal).await;

        // Restore terminal
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut ratatui::Terminal<
            ratatui::backend::CrosstermBackend<std::io::Stdout>,
        >,
    ) -> Result<()> {
        loop {
            terminal.draw(|frame| crate::ui::draw(frame, self))?;

            if crossterm::event::poll(std::time::Duration::from_millis(100))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    let action = self.handle_key(key);
                    let should_quit = self.execute_action(action).await?;
                    if should_quit {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn handle_key(&self, key: KeyEvent) -> AppAction {
        match &self.modal {
            ActiveModal::None => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => AppAction::Quit,
                KeyCode::Tab => AppAction::SwitchView,
                KeyCode::Char('j') | KeyCode::Down => AppAction::MoveDown,
                KeyCode::Char('k') | KeyCode::Up => AppAction::MoveUp,
                KeyCode::Char('g') => AppAction::JumpTop,
                KeyCode::Char('G') => AppAction::JumpBottom,
                KeyCode::Char('a') => AppAction::AddProxy,
                KeyCode::Char('e') => AppAction::EditProxy,
                KeyCode::Char('o') => AppAction::OpenBrowser,
                KeyCode::Char('r') => AppAction::Refresh,
                KeyCode::Char('c') => AppAction::CaddyMenu,
                KeyCode::Char('?') => AppAction::Help,
                _ => AppAction::None,
            },
            ActiveModal::AddProxy | ActiveModal::EditProxy => match key.code {
                KeyCode::Esc => AppAction::CloseModal,
                KeyCode::Tab => AppAction::FormNextField,
                KeyCode::BackTab => AppAction::FormPrevField,
                KeyCode::Enter => AppAction::FormConfirm,
                KeyCode::Backspace => AppAction::FormBackspace,
                KeyCode::Char(c) => AppAction::FormCharInput(c),
                _ => AppAction::None,
            },
            ActiveModal::CaddyMenu => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => AppAction::CloseModal,
                KeyCode::Char('j') | KeyCode::Down => {
                    AppAction::SelectItem((self.caddy_selected + 1) % 3)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    AppAction::SelectItem(self.caddy_selected.saturating_sub(1))
                }
                KeyCode::Enter => match self.caddy_selected {
                    0 => AppAction::CaddyStart,
                    1 => AppAction::CaddyStop,
                    _ => AppAction::CaddyRestart,
                },
                _ => AppAction::None,
            },
            ActiveModal::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                    AppAction::CloseModal
                }
                _ => AppAction::None,
            },
        }
    }

    pub async fn execute_action(&mut self, action: AppAction) -> Result<bool> {
        match action {
            AppAction::Quit => return Ok(true),
            AppAction::SwitchView => {
                if self.has_project {
                    self.view = match self.view {
                        View::Project => View::Global,
                        View::Global => View::Project,
                    };
                    self.selected = 0;
                }
            }
            AppAction::MoveDown => {
                let len = self.all_services().len();
                if len > 0 && self.selected < len - 1 {
                    self.selected += 1;
                }
            }
            AppAction::MoveUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            AppAction::JumpTop => self.selected = 0,
            AppAction::JumpBottom => {
                let len = self.all_services().len();
                if len > 0 {
                    self.selected = len - 1;
                }
            }
            AppAction::AddProxy => {
                let has_no_proxy = self
                    .all_services()
                    .get(self.selected)
                    .is_some_and(|s| s.proxy.is_none());
                if has_no_proxy {
                    let idx = self.selected;
                    self.open_add_form(idx);
                }
            }
            AppAction::EditProxy => {
                let has_proxy = self
                    .all_services()
                    .get(self.selected)
                    .is_some_and(|s| s.proxy.is_some());
                if has_proxy {
                    let idx = self.selected;
                    self.open_edit_form(idx);
                }
            }
            AppAction::OpenBrowser => {
                let _ = self.open_selected_in_browser();
            }
            AppAction::Refresh => {
                let _ = self.refresh().await;
            }
            AppAction::CaddyMenu => {
                self.modal = ActiveModal::CaddyMenu;
                self.caddy_selected = 0;
            }
            AppAction::Help => {
                self.modal = ActiveModal::Help;
            }
            AppAction::CloseModal => {
                self.close_modal();
            }
            AppAction::FormNextField => {
                self.form.focused_field = (self.form.focused_field + 1) % 3;
            }
            AppAction::FormPrevField => {
                self.form.focused_field = self.form.focused_field.saturating_sub(1);
            }
            AppAction::FormConfirm => {
                let _ = self.save_proxy().await;
            }
            AppAction::FormCharInput(c) => match self.form.focused_field {
                0 => self.form.domain.push(c),
                1 => self.form.port.push(c),
                2 => self.form.tls.push(c),
                _ => {}
            },
            AppAction::FormBackspace => match self.form.focused_field {
                0 => {
                    self.form.domain.pop();
                }
                1 => {
                    self.form.port.pop();
                }
                2 => {
                    self.form.tls.pop();
                }
                _ => {}
            },
            AppAction::CaddyStart => {
                let _ = self.manage_caddy("start").await;
                self.close_modal();
            }
            AppAction::CaddyStop => {
                let _ = self.manage_caddy("stop").await;
                self.close_modal();
            }
            AppAction::CaddyRestart => {
                let _ = self.manage_caddy("restart").await;
                self.close_modal();
            }
            AppAction::SelectItem(idx) => {
                self.caddy_selected = idx;
            }
            AppAction::None => {}
        }
        Ok(false)
    }

    pub async fn refresh(&mut self) -> Result<()> {
        // Re-query docker state
        if let Some(ref docker) = self.docker_client {
            self.caddy_status =
                crate::docker::containers::get_caddy_proxy_status(docker)
                    .await
                    .unwrap_or(CaddyProxyStatus::Unknown);
            self.global_services =
                crate::docker::containers::list_caddy_services(docker)
                    .await
                    .unwrap_or_default();
        }

        // Re-parse compose files
        let cwd = std::env::current_dir()?;
        self.compose_files =
            crate::compose::discovery::find_compose_files(&cwd).unwrap_or_default();
        self.services.clear();
        for file in &self.compose_files.clone() {
            if let Ok(compose) = crate::compose::parser::parse_compose_file(file) {
                if let Ok((_, mut svc)) =
                    crate::compose::parser::extract_services(&compose, file)
                {
                    self.services.append(&mut svc);
                }
            }
        }
        if let Some(ref docker) = self.docker_client {
            let _ = crate::docker::containers::merge_runtime_status(
                docker,
                &mut self.services,
            )
            .await;
        }

        self.active_domains =
            crate::caddy::admin::get_active_domains().await.unwrap_or_default();
        self.status_message = Some("Refreshed".to_string());
        Ok(())
    }

    pub async fn save_proxy(&mut self) -> Result<()> {
        let port: u16 = self.form.port.parse().unwrap_or(80);
        let config = ProxyConfig {
            domain: self.form.domain.clone(),
            port,
            tls: self.form.tls.clone(),
        };

        // Find the service's source file
        let services = match self.view {
            View::Project => &self.services,
            View::Global => &self.global_services,
        };

        let Some(service) = services.get(self.form.service_index) else {
            return Ok(());
        };

        let ServiceSource::Compose {
            ref file,
            ref service_name,
        } = service.source
        else {
            return Ok(());
        };

        let file = file.clone();
        let service_name = service_name.clone();

        // Parse, modify, write
        let mut compose = crate::compose::parser::parse_compose_file(&file)?;
        crate::compose::writer::add_caddy_labels(&mut compose, &service_name, &config)?;
        crate::compose::writer::write_compose_file(&compose, &file)?;

        // Apply with compose up
        if self.docker_client.is_some() {
            crate::docker::compose::compose_up(
                &file,
                &crate::docker::client::RuntimeType::Docker,
            )
            .await?;
        }

        self.close_modal();
        self.refresh().await?;
        self.status_message = Some(format!("Proxy added: {}", config.domain));
        Ok(())
    }

    pub async fn manage_caddy(&mut self, action: &str) -> Result<()> {
        let method = self
            .caddy_control
            .clone()
            .unwrap_or(CaddyControlMethod::Container);
        let runtime = crate::docker::client::RuntimeType::Docker;

        if let Some(ref docker) = self.docker_client {
            match action {
                "start" => {
                    crate::docker::containers::start_caddy(docker, &method, &runtime)
                        .await?
                }
                "stop" => {
                    crate::docker::containers::stop_caddy(docker, &method, &runtime)
                        .await?
                }
                "restart" => {
                    crate::docker::containers::restart_caddy(docker, &method, &runtime)
                        .await?
                }
                _ => {}
            }
        }

        // Refresh caddy status after a short delay
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Some(ref docker) = self.docker_client {
            self.caddy_status =
                crate::docker::containers::get_caddy_proxy_status(docker)
                    .await
                    .unwrap_or(CaddyProxyStatus::Unknown);
        }

        self.status_message = Some(format!("caddy-proxy {}ed", action));
        Ok(())
    }

    pub fn open_selected_in_browser(&self) -> Result<()> {
        let services = self.all_services();
        if let Some(service) = services.get(self.selected) {
            if let Some(ref proxy) = service.proxy {
                let url = format!("https://{}", proxy.domain);
                open::that(&url)?;
            }
        }
        Ok(())
    }

    pub fn open_add_form(&mut self, service_index: usize) {
        let services = match self.view {
            View::Project => &self.services,
            View::Global => &self.global_services,
        };

        if let Some(service) = services.get(service_index) {
            let domain =
                crate::compose::parser::default_domain(&service.name, &service.project);
            let port = service
                .available_ports
                .first()
                .copied()
                .unwrap_or(80)
                .to_string();
            self.form = FormState {
                focused_field: 0,
                domain,
                port,
                tls: "internal".to_string(),
                service_index,
            };
            self.modal = ActiveModal::AddProxy;
        }
    }

    pub fn open_edit_form(&mut self, service_index: usize) {
        let services = match self.view {
            View::Project => &self.services,
            View::Global => &self.global_services,
        };

        if let Some(service) = services.get(service_index) {
            let (domain, port, tls) = if let Some(ref proxy) = service.proxy {
                (
                    proxy.domain.clone(),
                    proxy.port.to_string(),
                    proxy.tls.clone(),
                )
            } else {
                (
                    crate::compose::parser::default_domain(
                        &service.name,
                        &service.project,
                    ),
                    "80".to_string(),
                    "internal".to_string(),
                )
            };
            self.form = FormState {
                focused_field: 0,
                domain,
                port,
                tls,
                service_index,
            };
            self.modal = ActiveModal::EditProxy;
        }
    }

    pub fn all_services(&self) -> &[Service] {
        match self.view {
            View::Project => &self.services,
            View::Global => &self.global_services,
        }
    }

    pub fn proxied_services(&self) -> Vec<&Service> {
        self.all_services()
            .iter()
            .filter(|s| s.proxy.is_some())
            .collect()
    }

    pub fn unproxied_services(&self) -> Vec<&Service> {
        self.all_services()
            .iter()
            .filter(|s| s.proxy.is_none())
            .collect()
    }

    pub fn current_selected_service(&self) -> Option<&Service> {
        self.all_services().get(self.selected)
    }

    pub fn close_modal(&mut self) {
        self.modal = ActiveModal::None;
    }
}
