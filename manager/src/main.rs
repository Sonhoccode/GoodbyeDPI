#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke, Vec2};
use windows_service::service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState, ServiceType};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

const SERVICE_NAME: &str = "GoodbyeDPI";
mod update;
const GREEN: Color32 = Color32::from_rgb(128, 189, 150);
const RED: Color32 = Color32::from_rgb(211, 133, 129);
const BLUE: Color32 = Color32::from_rgb(176, 187, 202);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppStatus {
    Running,
    Stopped,
    Pending,
    Missing,
    Error,
}

impl AppStatus {
    fn from_service_state(state: ServiceState) -> Self {
        match state {
            ServiceState::Running => Self::Running,
            ServiceState::Stopped => Self::Stopped,
            ServiceState::StartPending | ServiceState::StopPending => Self::Pending,
            _ => Self::Pending,
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Running => GREEN,
            Self::Stopped | Self::Missing | Self::Error => RED,
            Self::Pending => BLUE,
        }
    }
}

struct ManagerApp {
    status: AppStatus,
    message: String,
    last_refresh: Instant,
    language: usize,
    automatic: bool,
    page: usize,
    preset: usize,
    confirm_uninstall: bool,
    show_coffee: bool,
    action_receiver: Option<Receiver<(bool, Result<(), String>)>>,
    steam_receiver: Option<Receiver<Result<(), String>>>,
    steam_result: Option<Result<(), String>>,
    update_receiver: Option<Receiver<Result<Option<update::Release>, String>>>,
    download_receiver: Option<Receiver<Result<PathBuf, String>>>,
    available_release: Option<update::Release>,
    update_message: String,
}

const LANGUAGES: [&str; 4] = ["Tiếng Việt", "English", "Español", "Русский"];
fn tr(language: usize, vi: &'static str, en: &'static str, es: &'static str, ru: &'static str) -> &'static str {
    [vi, en, es, ru][language]
}

fn language_file() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|path| PathBuf::from(path).join("GoodbyeDPI Manager").join("language.txt"))
}

fn load_language() -> usize {
    language_file().and_then(|path| fs::read_to_string(path).ok())
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value < LANGUAGES.len()).unwrap_or(1)
}

fn save_language(language: usize) -> std::io::Result<()> {
    let path = language_file().ok_or_else(|| std::io::Error::other("LOCALAPPDATA unavailable"))?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, language.to_string())
}

const LEGACY_DNS_ARGS: &str = "--dns-addr 77.88.8.8 --dns-port 1253 --dnsv6-addr 2a02:6b8::feed:0ff --dnsv6-port 1253";

fn preset_from_command(command: &str) -> usize {
    if command.split_whitespace().any(|argument| argument == "-6") { 1 } else { 0 }
}

fn query_preset() -> usize {
    open_service(ServiceAccess::QUERY_CONFIG).and_then(|service| service.query_config()).ok()
        .map(|config| preset_from_command(&config.executable_path.to_string_lossy()))
        .unwrap_or(0)
}

fn apply_preset(preset: usize) -> Result<(), String> {
    if query_status() != AppStatus::Stopped { return Err("Stop the service first".into()); }
    change_service_preset(if preset == 1 { "-6" } else { "-5" })
}

fn bundled_engine() -> Result<PathBuf, String> {
    let engine = std::env::current_exe().map_err(|error| error.to_string())?
        .parent().ok_or("Missing application directory")?.join("engine").join("goodbyedpi.exe");
    if !engine.is_file() { return Err("Bundled engine not found".into()); }
    Ok(engine)
}

fn change_service_preset(mode: &str) -> Result<(), String> {
    let service = open_service(ServiceAccess::QUERY_CONFIG | ServiceAccess::CHANGE_CONFIG)
        .map_err(|error| error.to_string())?;
    let config = service.query_config().map_err(|error| error.to_string())?;
    let info = ServiceInfo {
        name: SERVICE_NAME.into(), display_name: config.display_name,
        service_type: config.service_type, start_type: config.start_type,
        error_control: config.error_control, executable_path: bundled_engine()?,
        launch_arguments: vec![mode.into()], dependencies: config.dependencies,
        account_name: config.account_name, account_password: None,
    };
    service.change_config(&info).map_err(|error| error.to_string())
}

fn legacy_dns_mode(command: &str, engine: &Path) -> Option<&'static str> {
    ["-5", "-6"].into_iter().find(|mode| {
        command.eq_ignore_ascii_case(&format!("\"{}\" {mode} {LEGACY_DNS_ARGS}", engine.display()))
    })
}

fn migrate_legacy_dns_service() -> Result<(), String> {
    let engine = bundled_engine()?;
    let service = open_service(ServiceAccess::QUERY_CONFIG).map_err(|error| error.to_string())?;
    let config = service.query_config().map_err(|error| error.to_string())?;
    let Some(mode) = legacy_dns_mode(&config.executable_path.to_string_lossy(), &engine) else { return Ok(()); };
    let was_running = query_status() == AppStatus::Running;
    change_service_preset(mode)?;
    if was_running {
        set_service_running(false)?;
        set_service_running(true)?;
    }
    Ok(())
}

fn query_automatic() -> bool {
    open_service(ServiceAccess::QUERY_CONFIG)
        .and_then(|service| service.query_config())
        .is_ok_and(|config| config.start_type == ServiceStartType::AutoStart)
}

fn set_automatic(enabled: bool) -> Result<(), String> {
    let mode = if enabled { "auto" } else { "demand" };
    let output = Command::new("sc.exe")
        .args(["config", SERVICE_NAME, "start=", mode])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stdout).trim().to_owned()) }
}

fn uninstaller_path() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|dir| dir.join("unins000.exe"))
}

impl Default for ManagerApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || { let _ = sender.send(update::check()); });
        let (steam_sender, steam_receiver) = mpsc::channel();
        std::thread::spawn(move || { let _ = steam_sender.send(check_steam_website()); });
        Self {
            status: query_status(),
            message: String::new(),
            last_refresh: Instant::now(),
            language: load_language(),
            automatic: query_automatic(),
            page: 0,
            preset: query_preset(),
            confirm_uninstall: false,
            show_coffee: false,
            action_receiver: None,
            steam_receiver: Some(steam_receiver),
            steam_result: None,
            update_receiver: Some(receiver),
            download_receiver: None,
            available_release: None,
            update_message: String::new(),
        }
    }
}

impl eframe::App for ManagerApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(Ok((start, result))) = self.action_receiver.as_ref().map(|receiver| receiver.try_recv()) {
            self.message = match result {
                Ok(()) => tr(self.language,
                    if start { "Đã bật GoodbyeDPI" } else { "Đã tắt GoodbyeDPI" },
                    if start { "GoodbyeDPI started" } else { "GoodbyeDPI stopped" },
                    if start { "GoodbyeDPI activado" } else { "GoodbyeDPI detenido" },
                    if start { "GoodbyeDPI запущен" } else { "GoodbyeDPI остановлен" }).into(),
                Err(error) => format!("{}: {error}", tr(self.language, "Không thể đổi dịch vụ", "Service action failed", "Error del servicio", "Ошибка службы")),
            };
            self.action_receiver = None;
            self.status = query_status();
            self.last_refresh = Instant::now();
            self.start_steam_check();
        }
        if let Some(Ok(result)) = self.steam_receiver.as_ref().map(|receiver| receiver.try_recv()) {
            self.steam_result = Some(result);
            self.steam_receiver = None;
        }
        if let Some(Ok(result)) = self.update_receiver.as_ref().map(|receiver| receiver.try_recv()) {
            match result {
                Ok(Some(release)) => { self.update_message = format!("{} available", release.version); self.available_release = Some(release); }
                Ok(None) => self.update_message = "Up to date".into(),
                Err(error) => self.update_message = format!("Update check failed: {error}"),
            }
            self.update_receiver = None;
        }
        if let Some(Ok(result)) = self.download_receiver.as_ref().map(|receiver| receiver.try_recv()) {
            match result {
                Ok(path) => match Command::new(path).spawn() {
                    Ok(_) => root_ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                    Err(error) => self.update_message = format!("Cannot run installer: {error}"),
                },
                Err(error) => self.update_message = format!("Update failed: {error}"),
            }
            self.download_receiver = None;
        }
        if self.action_receiver.is_none() && self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.status = query_status();
            self.last_refresh = Instant::now();
            self.automatic = query_automatic();
        }
        root_ui.ctx().request_repaint_after(Duration::from_millis(250));

        egui::Frame::new()
            .fill(Color32::from_rgb(22, 25, 29))
            .inner_margin(30.0)
            .show(root_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("GoodbyeDPI").size(27.0).strong().color(Color32::from_rgb(236, 239, 242)));
                        ui.label(RichText::new(tr(self.language, "Điều khiển kết nối", "Connection control", "Control de conexión", "Управление подключением")).size(13.0).color(BLUE));
                    });
                });
                ui.add_space(25.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.page, 0, tr(self.language, "Trang chủ", "Home", "Inicio", "Главная"));
                    ui.selectable_value(&mut self.page, 1, tr(self.language, "Cài đặt", "Settings", "Ajustes", "Настройки"));
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(24.0);

                if self.page == 1 {
                    ui.label(RichText::new(tr(self.language, "Ngôn ngữ", "Language", "Idioma", "Язык")).strong());
                    let old_language = self.language;
                    egui::ComboBox::from_id_salt("language").selected_text(LANGUAGES[self.language]).show_ui(ui, |ui| {
                        for (index, name) in LANGUAGES.iter().enumerate() { ui.selectable_value(&mut self.language, index, *name); }
                    });
                    if self.language != old_language {
                        if let Err(error) = save_language(self.language) { self.message = error.to_string(); }
                    }
                    ui.add_space(18.0);
                    ui.label(RichText::new(tr(self.language, "Preset kết nối", "Connection preset", "Perfil de conexión", "Профиль подключения")).strong());
                    egui::ComboBox::from_id_salt("preset").selected_text(if self.preset == 0 { "Compatible (-5)" } else { "Alternative (-6)" }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.preset, 0, "Compatible (-5)");
                        ui.selectable_value(&mut self.preset, 1, "Alternative (-6)");
                    });
                    ui.label(tr(self.language, "Dừng dịch vụ trước khi áp dụng preset.", "Stop the service before applying a preset.", "Detén el servicio antes de aplicar un perfil.", "Остановите службу перед применением профиля."));
                    if ui.add_enabled(self.status == AppStatus::Stopped, egui::Button::new(tr(self.language, "Áp dụng preset", "Apply preset", "Aplicar perfil", "Применить профиль"))).clicked() {
                        match apply_preset(self.preset) {
                            Ok(()) => self.message = tr(self.language, "Đã lưu preset. Hãy bật dịch vụ.", "Preset saved. Start the service.", "Perfil guardado. Inicia el servicio.", "Профиль сохранён. Запустите службу.").into(),
                            Err(error) => self.message = error,
                        }
                    }
                    ui.add_space(18.0);
                    let mut automatic = self.automatic;
                    if ui.add_enabled(self.status != AppStatus::Missing, egui::Checkbox::new(&mut automatic, tr(self.language, "Tự bật cùng Windows", "Start with Windows", "Iniciar con Windows", "Запускать с Windows"))).changed() {
                        match set_automatic(automatic) {
                            Ok(()) => self.automatic = automatic,
                            Err(error) => self.message = error,
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(RichText::new(tr(self.language, "Cập nhật", "Updates", "Actualizaciones", "Обновления")).strong());
                    ui.label(if self.update_message.is_empty() { "Checking for updates…" } else { &self.update_message });
                    if let Some(release) = &self.available_release {
                        if ui.add_enabled(self.download_receiver.is_none(), egui::Button::new(format!("Install {}", release.version))).clicked() {
                            let release = release.clone();
                            let (sender, receiver) = mpsc::channel();
                            std::thread::spawn(move || { let _ = sender.send(update::download(&release)); });
                            self.download_receiver = Some(receiver);
                            self.update_message = "Downloading and verifying installer…".into();
                        }
                    }
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr(self.language, "Gỡ cài đặt", "Uninstall", "Desinstalar", "Удаление")).strong());
                    ui.label(tr(self.language, "Xóa app và dịch vụ do bộ cài tạo.", "Remove the app and its installed service.", "Eliminar la app y su servicio.", "Удалить приложение и его службу."));
                    let installed = uninstaller_path().is_some_and(|path| path.is_file());
                    if ui.add_enabled(installed, egui::Button::new(tr(self.language, "Gỡ GoodbyeDPI Manager…", "Uninstall GoodbyeDPI Manager…", "Desinstalar GoodbyeDPI Manager…", "Удалить GoodbyeDPI Manager…"))).clicked() {
                        self.confirm_uninstall = true;
                    }
                } else {

                let panel_width = ui.available_width();
                egui::Frame::new()
                    .fill(Color32::from_rgb(33, 38, 44))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(64, 70, 77)))
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_min_width(panel_width - 50.0);
                        ui.set_min_height(135.0);
                        ui.label(RichText::new(tr(self.language, "GoodbyeDPI service", "GoodbyeDPI service", "Servicio GoodbyeDPI", "Служба GoodbyeDPI")).size(13.0).color(BLUE));
                        ui.add_space(10.0);
                                let status = match self.status {
                                    AppStatus::Running => tr(self.language, "ĐANG BẬT", "RUNNING", "ACTIVO", "РАБОТАЕТ"),
                                    AppStatus::Stopped => tr(self.language, "ĐANG TẮT", "STOPPED", "DETENIDO", "ОСТАНОВЛЕНО"),
                                    AppStatus::Pending => tr(self.language, "ĐANG XỬ LÝ", "WORKING", "PROCESANDO", "ОБРАБОТКА"),
                                    AppStatus::Missing => tr(self.language, "CHƯA CÀI DỊCH VỤ", "SERVICE NOT INSTALLED", "SERVICIO NO INSTALADO", "СЛУЖБА НЕ УСТАНОВЛЕНА"),
                                    AppStatus::Error => tr(self.language, "KHÔNG ĐỌC ĐƯỢC", "STATUS UNAVAILABLE", "ESTADO NO DISPONIBLE", "СТАТУС НЕДОСТУПЕН"),
                                };
                        ui.label(RichText::new(status).size(27.0).strong().color(self.status.color()));
                        ui.add_space(10.0);
                        let detail = if self.status == AppStatus::Running {
                            tr(self.language, "Bộ lọc mạng đang hoạt động.", "Network filtering is active.", "El filtrado de red está activo.", "Фильтрация сети включена.")
                        } else {
                            tr(self.language, "Bật dịch vụ để áp dụng bộ lọc mạng.", "Start the service to enable network filtering.", "Inicia el servicio para filtrar la red.", "Запустите службу для фильтрации сети.")
                        };
                        ui.label(RichText::new(detail).size(13.0).color(BLUE));
                    });

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if self.steam_receiver.is_some() { ui.add(egui::Spinner::new().size(16.0)); }
                    let (label, color) = match &self.steam_result {
                        None => (tr(self.language, "Đang kiểm tra trang Steam…", "Checking Steam website…", "Comprobando el sitio de Steam…", "Проверка сайта Steam…"), BLUE),
                        Some(Ok(())) => (tr(self.language, "Trang Steam truy cập được", "Steam website is reachable", "El sitio de Steam está disponible", "Сайт Steam доступен"), GREEN),
                        Some(Err(_)) => (tr(self.language, "Không truy cập được trang Steam", "Steam website is unreachable", "No se puede acceder al sitio de Steam", "Сайт Steam недоступен"), RED),
                    };
                    let response = ui.label(RichText::new(label).size(13.0).color(color));
                    if let Some(Err(error)) = &self.steam_result { response.on_hover_text(error); }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add_enabled(self.steam_receiver.is_none(), egui::Button::new(tr(self.language, "Kiểm tra lại", "Check again", "Comprobar", "Проверить"))).clicked() {
                            self.start_steam_check();
                        }
                    });
                });
                ui.add_space(6.0);
                ui.label(RichText::new(tr(self.language, "Chỉ kiểm tra website; Steam Client có thể báo lỗi riêng.", "Website check only; Steam Client may have a separate error.", "Solo comprueba el sitio; el cliente puede tener otro error.", "Проверяется только сайт; в клиенте возможна другая ошибка.")).size(11.0).color(BLUE));
                ui.add_space(20.0);
                let running = self.status == AppStatus::Running;
                let action = if self.action_receiver.is_some() {
                    tr(self.language, "Đang xử lý…", "Working…", "Procesando…", "Обработка…")
                } else if running {
                    tr(self.language, "Tắt GoodbyeDPI", "Stop GoodbyeDPI", "Detener GoodbyeDPI", "Выключить GoodbyeDPI")
                } else {
                    tr(self.language, "Bật GoodbyeDPI", "Start GoodbyeDPI", "Activar GoodbyeDPI", "Включить GoodbyeDPI")
                };
                let button = egui::Button::new(RichText::new(action).size(15.0).strong())
                    .fill(if running { Color32::from_rgb(61, 68, 76) } else { Color32::from_rgb(54, 103, 79) })
                    .min_size(Vec2::new(ui.available_width(), 48.0));
                if ui.add_enabled(self.action_receiver.is_none() && matches!(self.status, AppStatus::Running | AppStatus::Stopped), button).clicked() {
                    self.run_action(!running);
                }
                if self.action_receiver.is_some() {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(15.0));
                        ui.label(tr(self.language, "Đang đổi trạng thái dịch vụ…", "Changing service state…", "Cambiando el estado del servicio…", "Изменение состояния службы…"));
                    });
                }
                ui.add_space(18.0);
                if ui.link("Buy me a coffee").clicked() { self.show_coffee = true; }

                }

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if self.message.is_empty() { tr(self.language, "Sẵn sàng", "Ready", "Listo", "Готово") } else { &self.message }).size(13.0).color(Color32::from_rgb(203, 213, 225)));
                });
                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    ui.label(RichText::new("GoodbyeDPI Manager 1.03").size(11.0).color(Color32::from_rgb(140, 149, 159)));
                });
            });
        if self.confirm_uninstall {
            egui::Window::new(tr(self.language, "Xác nhận gỡ cài đặt", "Confirm uninstall", "Confirmar desinstalación", "Подтвердить удаление"))
                .collapsible(false).resizable(false).show(root_ui.ctx(), |ui| {
                    ui.label(tr(self.language, "GoodbyeDPI sẽ ngừng chạy nếu dịch vụ do app này cài đặt.", "GoodbyeDPI will stop if this app installed its service.", "GoodbyeDPI se detendrá si esta app instaló el servicio.", "GoodbyeDPI остановится, если служба установлена этим приложением."));
                    ui.horizontal(|ui| {
                        if ui.button(tr(self.language, "Hủy", "Cancel", "Cancelar", "Отмена")).clicked() { self.confirm_uninstall = false; }
                        if ui.button(tr(self.language, "Gỡ cài đặt", "Uninstall", "Desinstalar", "Удалить")).clicked() {
                            self.confirm_uninstall = false;
                            if let Some(path) = uninstaller_path() {
                                match Command::new(path).spawn() {
                                    Ok(_) => root_ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                                    Err(error) => self.message = error.to_string(),
                                }
                            }
                        }
                    });
                });
        }
        if self.show_coffee {
            egui::Window::new("Buy me a coffee").open(&mut self.show_coffee)
                .resizable(false).show(root_ui.ctx(), |ui| {
                    ui.label("Scan the VietQR code to support this project.");
                    ui.add(egui::Image::from_bytes("bytes://coffee.jpg", include_bytes!("../assets/coffee.jpg"))
                        .fit_to_exact_size(Vec2::new(320.0, 386.0)));
                });
        }
    }
}

impl ManagerApp {
    fn run_action(&mut self, start: bool) {
        self.status = AppStatus::Pending;
        self.message.clear();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || { let _ = sender.send((start, set_service_running(start))); });
        self.action_receiver = Some(receiver);
    }

    fn start_steam_check(&mut self) {
        self.steam_result = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || { let _ = sender.send(check_steam_website()); });
        self.steam_receiver = Some(receiver);
    }
}

fn check_steam_website() -> Result<(), String> {
    ureq::head("https://store.steampowered.com/")
        .timeout(Duration::from_secs(8))
        .call()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn open_service(access: ServiceAccess) -> windows_service::Result<windows_service::service::Service> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    manager.open_service(SERVICE_NAME, access)
}

fn query_status() -> AppStatus {
    match open_service(ServiceAccess::QUERY_STATUS) {
        Ok(service) => service
            .query_status()
            .map(|status| AppStatus::from_service_state(status.current_state))
            .unwrap_or(AppStatus::Error),
        Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(1060) => {
            AppStatus::Missing
        }
        Err(_) => AppStatus::Error,
    }
}

fn set_service_running(start: bool) -> Result<(), String> {
    let access = if start { ServiceAccess::START } else { ServiceAccess::STOP } | ServiceAccess::QUERY_STATUS;
    let service = open_service(access).map_err(|error| error.to_string())?;
    if start {
        service.start(&[] as &[&OsStr]).map_err(|error| error.to_string())?;
    } else {
        service.stop().map_err(|error| error.to_string())?;
    }
    let expected = if start { AppStatus::Running } else { AppStatus::Stopped };
    for _ in 0..50 {
        if query_status() == expected { return Ok(()); }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err("Service did not reach the requested state within 10 seconds".into())
}

fn install_service() -> Result<(), String> {
    let engine = std::env::current_exe()
        .map_err(|error| error.to_string())?
        .parent().ok_or("Missing application directory")?
        .join("engine").join("goodbyedpi.exe");
    if !engine.is_file() { return Err(format!("Engine not found: {}", engine.display())); }
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
        .map_err(|error| error.to_string())?;
    let info = ServiceInfo {
        name: SERVICE_NAME.into(), display_name: "GoodbyeDPI".into(),
        service_type: ServiceType::OWN_PROCESS, start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal, executable_path: engine,
        launch_arguments: vec!["-5".into()],
        dependencies: vec![], account_name: None, account_password: None,
    };
    manager.create_service(&info, ServiceAccess::QUERY_STATUS).map_err(|error| error.to_string())?;
    Ok(())
}

fn main() -> eframe::Result {
    if std::env::args_os().any(|arg| arg == "--migrate-service") {
        if let Err(error) = migrate_legacy_dns_service() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args_os().any(|arg| arg == "--install-service") {
        if let Err(error) = install_service() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GoodbyeDPI Manager")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("invalid app icon"))
            .with_inner_size([560.0, 590.0])
            .with_resizable(false)
            .with_maximize_button(false),
        ..Default::default()
    };

    eframe::run_native(
        "GoodbyeDPI Manager",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<ManagerApp>::default())
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_service_states() {
        assert_eq!(AppStatus::from_service_state(ServiceState::Running), AppStatus::Running);
        assert_eq!(AppStatus::from_service_state(ServiceState::Stopped), AppStatus::Stopped);
        assert_eq!(AppStatus::from_service_state(ServiceState::StartPending), AppStatus::Pending);
    }

    #[test]
    fn coffee_qr_decodes() {
        let image = image::load_from_memory(include_bytes!("../assets/coffee.jpg")).unwrap();
        assert_eq!(image.width(), 920);
        assert_eq!(image.height(), 1112);
    }

    #[test]
    fn identifies_plain_and_legacy_presets() {
        let engine = Path::new(r"C:\Program Files\GoodbyeDPI Manager\engine\goodbyedpi.exe");
        assert_eq!(preset_from_command(&format!("\"{}\" -6", engine.display())), 1);
        assert_eq!(preset_from_command(&format!("\"{}\" -5", engine.display())), 0);
        assert_eq!(legacy_dns_mode(&format!("\"{}\" -5 {LEGACY_DNS_ARGS}", engine.display()), engine), Some("-5"));
        assert_eq!(legacy_dns_mode(&format!("\"{}\" -5", engine.display()), engine), None);
    }
}

