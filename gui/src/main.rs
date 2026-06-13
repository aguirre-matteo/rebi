// gui/src/main.rs
use iced::{window, Element, Task, Length, Alignment, Subscription, keyboard, mouse, Event};
use iced::widget::{button, column, row, text, text_input, checkbox, scrollable, container, mouse_area, pick_list, Space};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::collections::HashSet;
use std::process::Command;
use directories::ProjectDirs;

mod config;
use config::KeydConfig;

#[derive(Debug, Clone)]
struct DeviceInfo {
    name: String,
    id: String,
}

fn main() -> iced::Result {
    let get_theme = |_app: &Rebi| iced::Theme::Dark;
    
    iced::application(|| Rebi::init(), Rebi::update, Rebi::view)
        .title("Rebi - Profile Manager")
        .subscription(Rebi::subscription)
        .window(window::Settings {
            decorations: true,
            resizable: true,
            ..Default::default()
        })
        .theme(get_theme)
        .run()
}

#[derive(Debug, Clone)]
struct ProfileItem {
    name: String,
    path: PathBuf, // Now a directory
}

#[derive(Debug, Clone)]
struct DeviceConfigItem {
    name: String,
    config: KeydConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgType {
    Key,     // Records key or combination (e.g., C-s)
    Layer,   // Layer name
    Timeout, // Number in ms
    Text,    // Free text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionType {
    Simple,
    Layer,
    Overload,
    OverloadT,
    Timeout,
    Macro,
    Swap,
    Command,
}

impl ActionType {
    const ALL: [ActionType; 8] = [
        ActionType::Simple,
        ActionType::Layer,
        ActionType::Overload,
        ActionType::OverloadT,
        ActionType::Timeout,
        ActionType::Macro,
        ActionType::Swap,
        ActionType::Command,
    ];

    fn args_schema(&self) -> Vec<ArgType> {
        match self {
            ActionType::Simple => vec![ArgType::Key],
            ActionType::Layer => vec![ArgType::Layer],
            ActionType::Overload => vec![ArgType::Layer, ArgType::Key],
            ActionType::OverloadT => vec![ArgType::Layer, ArgType::Key, ArgType::Timeout],
            ActionType::Timeout => vec![ArgType::Key, ArgType::Timeout, ArgType::Key],
            ActionType::Macro => vec![ArgType::Text],
            ActionType::Swap => vec![ArgType::Layer],
            ActionType::Command => vec![ArgType::Text],
        }
    }

    fn prefix(&self) -> Option<&'static str> {
        match self {
            ActionType::Simple => None,
            ActionType::Layer => Some("layer"),
            ActionType::Overload => Some("overload"),
            ActionType::OverloadT => Some("overloadt"),
            ActionType::Timeout => Some("timeout"),
            ActionType::Macro => Some("macro"),
            ActionType::Swap => Some("swap"),
            ActionType::Command => Some("command"),
        }
    }
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
enum Message {
    SearchProfile(String),
    NewProfilePressed,
    DeleteSelectedPressed,
    ProfileSelectedForEdit(ProfileItem),
    ToggleSelection(String, bool),
    StartRename(String),
    RenameChanged(String),
    ConfirmRename,
    CancelRename,
    // Layer Messages
    SelectLayer(String),
    NewLayerPressed,
    DeleteLayerPressed(String),
    SearchLayer(String),
    StartLayerRename(String),
    LayerRenameChanged(String),
    ConfirmLayerRename,
    CancelLayerRename,
    ToggleModifiersMenu,
    ToggleModifier(char, bool),
    // Device Config Messages
    SelectDeviceConfig(String),
    NewDeviceConfigPressed,
    DeleteDeviceConfig(String),
    StartDeviceConfigRename(String),
    DeviceConfigRenameChanged(String),
    ConfirmDeviceConfigRename,
    CancelDeviceConfigRename,
    // Mapping Messages
    AddMapping,
    DeleteMapping(String),
    StartMappingRecording(String),
    StopMappingRecording,
    ChangeActionType(String, ActionType),
    // Argument Messages
    ChangeActionArg(String, usize, String),
    StartArgRecording(String, usize),
    // ID Messages
    AddId,
    DeleteId(usize),
    ChangeId(usize, String),
    RefreshDevices,
    ToggleDevicePicker,
    AddDevice(DeviceInfo),
    // Unified Input Messages
    KeyPressed(keyboard::Event),
    MousePressed(mouse::Button),
    // System Messages
    ApplyConfiguration,
}

struct Rebi {
    profiles: Vec<ProfileItem>,
    selected_profiles: HashSet<String>,
    search_query: String,
    active_profile: Option<ProfileItem>,
    device_configs: Vec<DeviceConfigItem>,
    active_device_config: Option<String>,
    active_layer: Option<String>,
    layer_search_query: String,
    status_message: String,
    renaming_profile: Option<String>,
    renaming_name: String,
    renaming_layer: Option<String>,
    renaming_layer_name: String,
    renaming_device_config: Option<String>,
    renaming_device_config_name: String,
    show_modifiers: bool,
    recording_mapping: Option<String>,
    recording_arg: Option<(String, usize)>, // (key, arg_index)
    helper_path: Option<PathBuf>,
    detected_devices: Vec<DeviceInfo>,
    show_device_picker: bool,
}

impl Rebi {
    fn split_args(s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        
        for c in s.chars() {
            match c {
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    result.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            result.push(current.trim().to_string());
        }
        result
    }

    fn init() -> (Self, Task<Message>) {
        let mut app = Self {
            profiles: Vec::new(),
            selected_profiles: HashSet::new(),
            search_query: String::new(),
            active_profile: None,
            device_configs: Vec::new(),
            active_device_config: None,
            active_layer: None,
            layer_search_query: String::new(),
            status_message: "Ready".to_string(),
            renaming_profile: None,
            renaming_name: String::new(),
            renaming_layer: None,
            renaming_layer_name: String::new(),
            renaming_device_config: None,
            renaming_device_config_name: String::new(),
            show_modifiers: false,
            recording_mapping: None,
            recording_arg: None,
            helper_path: None,
            detected_devices: Vec::new(),
            show_device_picker: false,
        };
        app.load_profiles();
        app.detected_devices = Self::list_devices();
        
        // Try to find the helper executable
        if let Ok(exe_path) = env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let h_path = dir.join("rebi-helper");
                if h_path.exists() {
                    app.helper_path = Some(h_path);
                }
            }
        }

        (app, Task::none())
    }

    fn list_devices() -> Vec<DeviceInfo> {
        let mut devices = Vec::new();
        if let Ok(content) = fs::read_to_string("/proc/bus/input/devices") {
            let mut current_name = String::new();
            let mut current_id = String::new();

            for line in content.lines() {
                if line.starts_with("I: ") {
                    let mut vendor = "";
                    let mut product = "";
                    for part in line.split_whitespace() {
                        if part.starts_with("Vendor=") {
                            vendor = &part[7..];
                        } else if part.starts_with("Product=") {
                            product = &part[8..];
                        }
                    }
                    if !vendor.is_empty() && !product.is_empty() {
                        current_id = format!("{}:{}", vendor, product);
                    }
                } else if line.starts_with("N: Name=\"") {
                    current_name = line[9..line.len()-1].to_string();
                } else if line.is_empty() {
                    if !current_id.is_empty() {
                        devices.push(DeviceInfo {
                            name: if current_name.is_empty() { "Unknown Device".to_string() } else { current_name.clone() },
                            id: current_id.clone(),
                        });
                    }
                    current_name.clear();
                    current_id.clear();
                }
            }
            if !current_id.is_empty() {
                devices.push(DeviceInfo {
                    name: if current_name.is_empty() { "Unknown Device".to_string() } else { current_name.clone() },
                    id: current_id,
                });
            }
        }
        // Deduplicate and filter out some common non-keyboard things if needed, 
        // but for now let's just keep them all and deduplicate by ID and Name.
        devices.sort_by(|a, b| a.name.cmp(&b.name));
        devices.dedup_by(|a, b| a.id == b.id && a.name == b.name);
        devices
    }

    fn get_profiles_path() -> Option<PathBuf> {
        ProjectDirs::from("org", "rebi", "rebi").map(|proj_dirs| {
            let mut p = proj_dirs.config_dir().to_path_buf();
            p.push("profiles");
            p
        })
    }

    fn load_profiles(&mut self) {
        if let Some(path) = Self::get_profiles_path() {
            let _ = fs::create_dir_all(&path);

            if let Ok(entries) = fs::read_dir(path) {
                self.profiles = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| ProfileItem {
                        name: e.file_name().to_string_lossy().into_owned(),
                        path: e.path(),
                    })
                    .collect();
            }
        }
    }

    fn save_active_config(&self) {
        if let (Some(profile), Some(device_config_name)) = (&self.active_profile, &self.active_device_config) {
            if let Some(item) = self.device_configs.iter().find(|d| &d.name == device_config_name) {
                let file_path = profile.path.join(format!("{}.conf", device_config_name));
                let _ = item.config.export_to_file(file_path);
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchProfile(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::NewProfilePressed => {
                if let Some(base_path) = Self::get_profiles_path() {
                    let mut i = 1;
                    let mut new_name = format!("New profile {}", i);
                    let mut dir_path = base_path.join(&new_name);

                    while dir_path.exists() {
                        i += 1;
                        new_name = format!("New profile {}", i);
                        dir_path = base_path.join(&new_name);
                    }

                    if let Err(e) = fs::create_dir_all(&dir_path) {
                        self.status_message = format!("Error creating profile: {}", e);
                    } else {
                        // Create a default device config
                        let default_config_path = dir_path.join("default.conf");
                        let _ = fs::write(default_config_path, "[ids]\n*\n\n[main]\n");
                        
                        self.status_message = format!("Profile '{}' created.", new_name);
                        self.load_profiles();
                    }
                }
                Task::none()
            }
            Message::DeleteSelectedPressed => {
                let mut errors = 0;
                for name in &self.selected_profiles {
                    if let Some(item) = self.profiles.iter().find(|p| &p.name == name) {
                        if fs::remove_dir_all(&item.path).is_err() {
                            errors += 1;
                        }
                    }
                }
                
                if errors > 0 {
                    self.status_message = format!("Errors occurred while deleting {} profile(s)", errors);
                } else {
                    self.status_message = "Selected profiles deleted.".to_string();
                }

                self.selected_profiles.clear();
                if let Some(ref active) = self.active_profile {
                    if !active.path.exists() {
                        self.active_profile = None;
                        self.device_configs.clear();
                        self.active_device_config = None;
                    }
                }
                self.load_profiles();
                Task::none()
            }
            Message::ProfileSelectedForEdit(profile) => {
                self.active_profile = Some(profile.clone());
                self.device_configs.clear();
                self.active_device_config = None;
                self.active_layer = None;

                if let Ok(entries) = fs::read_dir(&profile.path) {
                    self.device_configs = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |ext| ext == "conf"))
                        .filter_map(|e| {
                            let name = e.path().file_stem().unwrap().to_string_lossy().into_owned();
                            KeydConfig::from_file(e.path()).ok().map(|config| DeviceConfigItem { name, config })
                        })
                        .collect();
                    
                    self.device_configs.sort_by(|a, b| a.name.cmp(&b.name));
                    
                    if let Some(first) = self.device_configs.first() {
                        self.active_device_config = Some(first.name.clone());
                        self.active_layer = Some("main".to_string());
                    }
                }
                Task::none()
            }
            Message::ToggleSelection(name, checked) => {
                if checked {
                    self.selected_profiles.insert(name);
                } else {
                    self.selected_profiles.remove(&name);
                }
                Task::none()
            }
            Message::StartRename(name) => {
                self.renaming_profile = Some(name.clone());
                self.renaming_name = name.clone();
                self.status_message = format!("Renaming '{}'...", name);
                Task::none()
            }
            Message::RenameChanged(new_name) => {
                self.renaming_name = new_name;
                Task::none()
            }
            Message::ConfirmRename => {
                if let Some(old_name) = self.renaming_profile.take() {
                    let new_name = self.renaming_name.trim().to_string();
                    if !new_name.is_empty() && new_name != old_name {
                        if let Some(item) = self.profiles.iter().find(|p| p.name == old_name) {
                            let mut new_path = item.path.clone();
                            new_path.set_file_name(new_name.clone());
                            
                            if new_path.exists() {
                                self.status_message = format!("Error: Profile '{}' already exists.", new_name);
                            } else if let Err(e) = fs::rename(&item.path, &new_path) {
                                self.status_message = format!("Error renaming: {}", e);
                            } else {
                                self.status_message = format!("Profile '{}' renamed to '{}'.", old_name, new_name);
                                
                                if self.selected_profiles.remove(&old_name) {
                                    self.selected_profiles.insert(new_name.clone());
                                }

                                if let Some(ref mut active) = self.active_profile {
                                    if active.name == old_name {
                                        active.name = new_name.clone();
                                        active.path = new_path;
                                    }
                                }
                                
                                self.load_profiles();
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CancelRename => {
                self.renaming_profile = None;
                Task::none()
            }
            Message::SelectDeviceConfig(name) => {
                self.active_device_config = Some(name);
                self.active_layer = Some("main".to_string());
                Task::none()
            }
            Message::NewDeviceConfigPressed => {
                if let Some(profile) = &self.active_profile {
                    let mut i = 1;
                    let mut new_name = format!("device_{}", i);
                    while self.device_configs.iter().any(|d| d.name == new_name) {
                        i += 1;
                        new_name = format!("device_{}", i);
                    }
                    
                    let config = KeydConfig::from_str("[ids]\n*\n\n[main]\n");
                    let file_path = profile.path.join(format!("{}.conf", new_name));
                    let _ = config.export_to_file(file_path);
                    
                    self.device_configs.push(DeviceConfigItem { name: new_name.clone(), config });
                    self.device_configs.sort_by(|a, b| a.name.cmp(&b.name));
                    self.active_device_config = Some(new_name);
                    self.active_layer = Some("main".to_string());
                }
                Task::none()
            }
            Message::DeleteDeviceConfig(name) => {
                if let Some(profile) = &self.active_profile {
                    let file_path = profile.path.join(format!("{}.conf", name));
                    let _ = fs::remove_file(file_path);
                    self.device_configs.retain(|d| d.name != name);
                    if self.active_device_config.as_ref() == Some(&name) {
                        if let Some(first) = self.device_configs.first() {
                            self.active_device_config = Some(first.name.clone());
                            self.active_layer = Some("main".to_string());
                        } else {
                            self.active_device_config = None;
                            self.active_layer = None;
                        }
                    }
                }
                Task::none()
            }
            Message::StartDeviceConfigRename(name) => {
                self.renaming_device_config = Some(name.clone());
                self.renaming_device_config_name = name;
                Task::none()
            }
            Message::DeviceConfigRenameChanged(new_name) => {
                self.renaming_device_config_name = new_name;
                Task::none()
            }
            Message::ConfirmDeviceConfigRename => {
                if let Some(old_name) = self.renaming_device_config.take() {
                    let new_name = self.renaming_device_config_name.trim().to_string();
                    if !new_name.is_empty() && new_name != old_name {
                        if let Some(profile) = &self.active_profile {
                            if self.device_configs.iter().any(|d| d.name == new_name) {
                                self.status_message = format!("Error: Device config '{}' already exists.", new_name);
                            } else {
                                let old_path = profile.path.join(format!("{}.conf", old_name));
                                let new_path = profile.path.join(format!("{}.conf", new_name));
                                if let Err(e) = fs::rename(old_path, new_path) {
                                    self.status_message = format!("Error renaming: {}", e);
                                } else {
                                    if let Some(item) = self.device_configs.iter_mut().find(|d| d.name == old_name) {
                                        item.name = new_name.clone();
                                    }
                                    if self.active_device_config.as_ref() == Some(&old_name) {
                                        self.active_device_config = Some(new_name);
                                    }
                                    self.device_configs.sort_by(|a, b| a.name.cmp(&b.name));
                                }
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CancelDeviceConfigRename => {
                self.renaming_device_config = None;
                Task::none()
            }
            Message::SelectLayer(name) => {
                self.active_layer = Some(name);
                Task::none()
            }
            Message::SearchLayer(query) => {
                self.layer_search_query = query;
                Task::none()
            }
            Message::NewLayerPressed => {
                if let (Some(device_config_name), Some(_profile)) = (&self.active_device_config, &self.active_profile) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        let mut i = 1;
                        let mut new_name = format!("New layer {}", i);
                        while item.config.layers.iter().any(|l| l.name == new_name) {
                            i += 1;
                            new_name = format!("New layer {}", i);
                        }
                        item.config.add_layer(new_name.clone());
                        self.active_layer = Some(new_name);
                        self.save_active_config();
                    }
                }
                Task::none()
            }
            Message::DeleteLayerPressed(name) => {
                if name != "main" {
                    if let Some(device_config_name) = &self.active_device_config {
                        if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                            item.config.remove_layer(&name);
                            if self.active_layer.as_ref() == Some(&name) {
                                self.active_layer = Some("main".to_string());
                            }
                            self.save_active_config();
                        }
                    }
                }
                Task::none()
            }
            Message::StartLayerRename(name) => {
                if name != "main" {
                    self.renaming_layer = Some(name.clone());
                    self.renaming_layer_name = name;
                }
                Task::none()
            }
            Message::LayerRenameChanged(new_name) => {
                self.renaming_layer_name = new_name;
                Task::none()
            }
            Message::ConfirmLayerRename => {
                if let Some(old_name) = self.renaming_layer.take() {
                    let new_name = self.renaming_layer_name.trim().to_string();
                    if new_name.contains(':') {
                        self.status_message = "Error: Layer name cannot contain ':'".to_string();
                    } else if !new_name.is_empty() && new_name != old_name {
                        if let Some(device_config_name) = &self.active_device_config {
                            if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                                if item.config.layers.iter().any(|l| l.name == new_name) {
                                    self.status_message = format!("Error: Layer '{}' already exists.", new_name);
                                } else if let Some(layer) = item.config.get_layer_mut(&old_name) {
                                    layer.name = new_name.clone();
                                    if self.active_layer.as_ref() == Some(&old_name) {
                                        self.active_layer = Some(new_name.clone());
                                    }
                                    self.save_active_config();
                                    self.status_message = format!("Layer '{}' renamed to '{}'.", old_name, new_name);
                                }
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CancelLayerRename => {
                self.renaming_layer = None;
                Task::none()
            }
            Message::ToggleModifiersMenu => {
                self.show_modifiers = !self.show_modifiers;
                Task::none()
            }
            Message::ToggleModifier(m, checked) => {
                if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                            if checked {
                                if !layer.modifiers.contains(&m) {
                                    layer.modifiers.push(m);
                                }
                            } else {
                                layer.modifiers.retain(|&x| x != m);
                            }
                            self.save_active_config();
                        }
                    }
                }
                Task::none()
            }
            Message::AddMapping => {
                if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                            let mut i = 1;
                            let mut new_key = format!("key_{}", i);
                            while layer.mappings.contains_key(&new_key) {
                                i += 1;
                                new_key = format!("key_{}", i);
                            }
                            layer.mappings.insert(new_key, "void".to_string());
                            self.save_active_config();
                        }
                    }
                }
                Task::none()
            }
            Message::DeleteMapping(key) => {
                if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                            layer.mappings.remove(&key);
                            self.save_active_config();
                        }
                    }
                }
                Task::none()
            }
            Message::StartMappingRecording(key) => {
                self.recording_mapping = Some(key);
                Task::none()
            }
            Message::StopMappingRecording => {
                self.recording_mapping = None;
                self.recording_arg = None;
                Task::none()
            }
            Message::ChangeActionType(key, action_type) => {
                if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                            if let Some(value) = layer.mappings.get_mut(&key) {
                                let content = if let Some(start) = value.find('(') {
                                    if let Some(end) = value.rfind(')') {
                                        &value[start+1..end]
                                    } else {
                                        ""
                                    }
                                } else {
                                    value.as_str()
                                };

                                let args = Self::split_args(content);
                                let schema = action_type.args_schema();
                                let mut new_args = Vec::new();

                                for i in 0..schema.len() {
                                    if i < args.len() && !args[i].is_empty() {
                                        new_args.push(args[i].to_string());
                                    } else {
                                        new_args.push(match schema[i] {
                                            ArgType::Key => "void".to_string(),
                                            ArgType::Layer => "main".to_string(),
                                            ArgType::Timeout => "200".to_string(),
                                            ArgType::Text => "type_here".to_string(),
                                        });
                                    }
                                }

                                if let Some(prefix) = action_type.prefix() {
                                    *value = format!("{}({})", prefix, new_args.join(", "));
                                } else {
                                    *value = new_args.first().cloned().unwrap_or_else(|| "void".to_string());
                                }
                                self.save_active_config();
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ChangeActionArg(key, index, new_value) => {
                if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                            if let Some(value) = layer.mappings.get_mut(&key) {
                                if let Some(start) = value.find('(') {
                                    let prefix = &value[..start+1];
                                    let last_rparen = value.rfind(')').unwrap_or(value.len());
                                    let suffix = &value[last_rparen..];
                                    let content = &value[start+1..last_rparen];
                                    let mut args = Self::split_args(content);
                                    
                                    while args.len() <= index {
                                        args.push(String::new());
                                    }

                                    args[index] = new_value;
                                    *value = format!("{}{}{}", prefix, args.join(", "), suffix);
                                } else if index == 0 {
                                    *value = new_value;
                                }
                                self.save_active_config();
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::StartArgRecording(key, index) => {
                self.recording_arg = Some((key, index));
                Task::none()
            }
            Message::AddId => {
                if let Some(device_config_name) = &self.active_device_config {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        item.config.ids.push("".to_string());
                        self.save_active_config();
                    }
                }
                Task::none()
            }
            Message::DeleteId(index) => {
                if let Some(device_config_name) = &self.active_device_config {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if item.config.ids.len() > 1 {
                            item.config.ids.remove(index);
                        } else {
                            item.config.ids[0] = "*".to_string();
                        }
                        self.save_active_config();
                    }
                }
                Task::none()
            }
            Message::ChangeId(index, new_id) => {
                if let Some(device_config_name) = &self.active_device_config {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if index < item.config.ids.len() {
                            item.config.ids[index] = new_id;
                            self.save_active_config();
                        }
                    }
                }
                Task::none()
            }
            Message::RefreshDevices => {
                self.detected_devices = Self::list_devices();
                Task::none()
            }
            Message::ToggleDevicePicker => {
                self.show_device_picker = !self.show_device_picker;
                if self.show_device_picker {
                    self.detected_devices = Self::list_devices();
                }
                Task::none()
            }
            Message::AddDevice(device) => {
                if let Some(device_config_name) = &self.active_device_config {
                    if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                        if item.config.ids.len() == 1 && item.config.ids[0] == "*" {
                            item.config.ids[0] = device.id;
                        } else if !item.config.ids.contains(&device.id) {
                            item.config.ids.push(device.id);
                        }
                        self.save_active_config();
                    }
                }
                self.show_device_picker = false;
                Task::none()
            }
            Message::KeyPressed(event) => {
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                    if self.recording_mapping.is_some() || self.recording_arg.is_some() {
                        let mut parts = Vec::new();
                        if modifiers.control() { parts.push("C"); }
                        if modifiers.logo() { parts.push("M"); }
                        if modifiers.alt() { parts.push("A"); }
                        if modifiers.shift() { parts.push("S"); }

                        let key_str = match key {
                            keyboard::Key::Named(n) => format!("{:?}", n).to_lowercase(),
                            keyboard::Key::Character(c) => c.to_lowercase().to_string(),
                            _ => return Task::none(),
                        };

                        let final_key = if parts.is_empty() {
                            key_str
                        } else {
                            format!("{}-{}", parts.join("-"), key_str)
                        };

                        if let Some(old_key) = self.recording_mapping.take() {
                            if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                                if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                                    if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                                        if let Some(value) = layer.mappings.remove(&old_key) {
                                            layer.mappings.insert(final_key, value);
                                            self.save_active_config();
                                        }
                                    }
                                }
                            }
                        } else if let Some((key, index)) = self.recording_arg.take() {
                            return Task::done(Message::ChangeActionArg(key, index, final_key));
                        }
                    }
                }
                Task::none()
            }
            Message::MousePressed(button) => {
                if self.recording_mapping.is_some() || self.recording_arg.is_some() {
                    let button_str = match button {
                        mouse::Button::Left => "leftmouse".to_string(),
                        mouse::Button::Right => "rightmouse".to_string(),
                        mouse::Button::Middle => "middlemouse".to_string(),
                        mouse::Button::Back => "mouse1".to_string(),
                        mouse::Button::Forward => "mouse2".to_string(),
                        mouse::Button::Other(n) => format!("mouse{}", n),
                    };

                    if let Some(old_key) = self.recording_mapping.take() {
                        if let (Some(device_config_name), Some(active_layer_name)) = (&self.active_device_config, &self.active_layer) {
                            if let Some(item) = self.device_configs.iter_mut().find(|d| &d.name == device_config_name) {
                                if let Some(layer) = item.config.get_layer_mut(active_layer_name) {
                                    if let Some(value) = layer.mappings.remove(&old_key) {
                                        layer.mappings.insert(button_str, value);
                                        self.save_active_config();
                                    }
                                }
                            }
                        }
                    } else if let Some((key, index)) = self.recording_arg.take() {
                        return Task::done(Message::ChangeActionArg(key, index, button_str));
                    }
                }
                Task::none()
            }
            Message::ApplyConfiguration => {
                if let Some(active_profile) = &self.active_profile {
                    let helper_env = env::var("REBI_HELPER_PATH").ok();
                    let helper = helper_env.as_ref()
                        .filter(|p| Path::new(p).exists())
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| {
                            self.helper_path.as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "rebi-helper".to_string())
                        });

                    let result = Command::new("pkexec")
                        .arg(helper)
                        .arg(&active_profile.path)
                        .status();

                    match result {
                        Ok(status) if status.success() => {
                            self.status_message = format!("Profile '{}' applied successfully to /etc/keyd", active_profile.name);
                        }
                        Ok(status) => {
                            self.status_message = format!("Error applying (root): Code {:?}", status.code());
                        }
                        Err(e) => {
                            self.status_message = format!("Error executing pkexec: {}", e);
                        }
                    }
                } else {
                    self.status_message = "Error: No profile selected to apply".to_string();
                }
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let needs_input = self.renaming_profile.is_some() ||
                            self.renaming_layer.is_some() ||
                            self.renaming_device_config.is_some() ||
                            self.recording_mapping.is_some() ||
                            self.recording_arg.is_some();

        if needs_input {
            iced::event::listen().filter_map(|event| {
                match event {
                    Event::Keyboard(kb_event) => Some(Message::KeyPressed(kb_event)),
                    Event::Mouse(mouse::Event::ButtonPressed(button)) => Some(Message::MousePressed(button)),
                    _ => None,
                }
            })
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let new_button = button("+ New").on_press(Message::NewProfilePressed).padding(8);
        
        let mut delete_button = button(" X ").padding(8);
        if !self.selected_profiles.is_empty() {
            delete_button = delete_button.on_press(Message::DeleteSelectedPressed);
        }

        let sidebar_top_bar = row![new_button, delete_button].spacing(10);
        let search_input = text_input("Search profile...", &self.search_query)
            .on_input(Message::SearchProfile)
            .padding(8);

        let mut profile_list_col = column![].spacing(8).width(Length::Fill);

        let filtered_profiles = self.profiles.iter().filter(|p| {
            self.search_query.is_empty() || p.name.to_lowercase().contains(&self.search_query.to_lowercase())
        });

        for profile in filtered_profiles {
            let is_checked = self.selected_profiles.contains(&profile.name);
            let is_renaming = self.renaming_profile.as_ref() == Some(&profile.name);
            let is_active = self.active_profile.as_ref().map(|p| &p.name) == Some(&profile.name);

            let profile_content: Element<Message> = if is_renaming {
                text_input("", &self.renaming_name)
                    .on_input(Message::RenameChanged)
                    .on_submit(Message::ConfirmRename)
                    .padding(5)
                    .width(Length::Fill)
                    .into()
            } else {
                mouse_area(
                    container(text(&profile.name).size(16))
                        .padding(5)
                        .width(Length::Fill)
                )
                .on_press(Message::ProfileSelectedForEdit(profile.clone()))
                .on_double_click(Message::StartRename(profile.name.clone()))
                .into()
            };
            
            let profile_row = container(row![
                checkbox(is_checked).on_toggle({
                    let name = profile.name.clone();
                    move |checked| Message::ToggleSelection(name.clone(), checked)
                }),
                profile_content
            ]
            .align_y(Alignment::Center)
            .spacing(5))
            .padding(iced::Padding::new(4.0))
            .style(move |theme| {
                if is_active {
                    container::Style {
                        background: Some(theme.palette().primary.into()),
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            });

            profile_list_col = profile_list_col.push(profile_row);
        }

        let sidebar = column![
            sidebar_top_bar,
            search_input,
            scrollable(profile_list_col).height(Length::Fill)
        ]
        .spacing(15)
        .width(Length::Fixed(200.0))
        .padding(10);

        let middle_content: Element<Message> = if self.active_profile.is_some() {
            let mut dev_list_col = column![].spacing(5).width(Length::Fill);
            for dev in &self.device_configs {
                let is_active_dev = self.active_device_config.as_ref() == Some(&dev.name);
                let is_renaming_dev = self.renaming_device_config.as_ref() == Some(&dev.name);
                
                let dev_content: Element<Message> = if is_renaming_dev {
                    text_input("", &self.renaming_device_config_name)
                        .on_input(Message::DeviceConfigRenameChanged)
                        .on_submit(Message::ConfirmDeviceConfigRename)
                        .padding(2)
                        .width(Length::Fill)
                        .into()
                } else {
                    mouse_area(
                        container(text(&dev.name).size(14))
                            .padding(5)
                            .width(Length::Fill)
                    )
                    .on_press(Message::SelectDeviceConfig(dev.name.clone()))
                    .on_double_click(Message::StartDeviceConfigRename(dev.name.clone()))
                    .into()
                };

                let dev_row = container(row![
                    dev_content,
                    button(text(" X ").size(12))
                        .on_press(Message::DeleteDeviceConfig(dev.name.clone()))
                        .style(button::danger)
                ].spacing(5).align_y(Alignment::Center))
                .padding(iced::Padding::new(2.0).left(8.0))
                .style(move |theme| {
                    if is_active_dev {
                        container::Style {
                            background: Some(theme.palette().primary.into()),
                            ..Default::default()
                        }
                    } else {
                        Default::default()
                    }
                });
                
                dev_list_col = dev_list_col.push(dev_row);
            }

            column![
                text("Devices").size(18),
                button("+ New Device").on_press(Message::NewDeviceConfigPressed).padding(8).width(Length::Fill),
                scrollable(dev_list_col).height(Length::Fill)
            ]
            .spacing(10)
            .width(Length::Fixed(180.0))
            .padding(10)
            .into()
        } else {
            container(text("")).width(Length::Fixed(0.0)).into()
        };

        let right_content: Element<Message> = match (&self.active_profile, &self.active_device_config) {
            (Some(_profile), Some(device_config_name)) => {
                let item = self.device_configs.iter().find(|d| &d.name == device_config_name).unwrap();
                let config = &item.config;

                let layer_search_input = text_input("Search layer...", &self.layer_search_query)
                    .on_input(Message::SearchLayer)
                    .padding(8);

                let new_layer_button = button("+ New Layer")
                    .on_press(Message::NewLayerPressed)
                    .padding(8)
                    .width(Length::Fill);

                let mut layer_list_col = column![].spacing(5).width(Length::Fill);

                // Always show [main] first
                let layer_names: Vec<String> = {
                    let mut rest: Vec<String> = config.layers.iter()
                        .map(|l| l.name.clone())
                        .filter(|n| n != "main")
                        .filter(|n| self.layer_search_query.is_empty() || n.to_lowercase().contains(&self.layer_search_query.to_lowercase()))
                        .collect();
                    rest.sort();
                    let mut final_list = vec!["main".to_string()];
                    final_list.extend(rest);
                    final_list
                };

                for name in layer_names {
                    let is_active_layer = self.active_layer.as_ref() == Some(&name);
                    let is_renaming_layer = self.renaming_layer.as_ref() == Some(&name);
                    
                    let display_name = name.clone();

                    let layer_content: Element<Message> = if is_renaming_layer {
                        text_input("", &self.renaming_layer_name)
                            .on_input(Message::LayerRenameChanged)
                            .on_submit(Message::ConfirmLayerRename)
                            .padding(2)
                            .width(Length::Fill)
                            .into()
                    } else {
                        mouse_area(
                            container(text(display_name).size(14))
                                .padding(5)
                                .width(Length::Fill)
                        )
                        .on_press(Message::SelectLayer(name.clone()))
                        .on_double_click(Message::StartLayerRename(name.clone()))
                        .into()
                    };
                    
                    let layer_row = container(row![
                        layer_content,
                        if name != "main" {
                            Some(button(text(" X ").size(12))
                                .on_press(Message::DeleteLayerPressed(name.clone()))
                                .style(button::danger))
                        } else {
                            None
                        }
                    ].spacing(5)
                    .align_y(Alignment::Center)
                    .padding(iced::Padding::new(2.0).left(8.0)))
                    .style(move |theme| {
                        if is_active_layer {
                            container::Style {
                                background: Some(theme.palette().primary.into()),
                                ..Default::default()
                            }
                        } else {
                            Default::default()
                        }
                    });

                    layer_list_col = layer_list_col.push(layer_row);
                }

                let mut ids_col = column![].spacing(5);
                for (i, id) in config.ids.iter().enumerate() {
                    ids_col = ids_col.push(
                        row![
                            text_input("ID...", id)
                                .on_input(move |v| Message::ChangeId(i, v))
                                .padding(5),
                            button(text(" X ").size(12))
                                .on_press(Message::DeleteId(i))
                                .style(button::danger)
                        ].spacing(5).align_y(Alignment::Center)
                    );
                }
                ids_col = ids_col.push(
                    row![
                        button("+ Manual ID").on_press(Message::AddId).padding(5).width(Length::Fill),
                        button("Detect...").on_press(Message::ToggleDevicePicker).padding(5).width(Length::Fill)
                    ].spacing(5)
                );

                if self.show_device_picker {
                    let mut device_list = column![].spacing(5);
                    for device in &self.detected_devices {
                        let dev = device.clone();
                        device_list = device_list.push(
                            button(
                                column![
                                    text(&device.name).size(14),
                                    text(&device.id).size(12).color([0.5, 0.5, 0.5])
                                ]
                            )
                            .on_press(Message::AddDevice(dev))
                            .width(Length::Fill)
                            .padding(5)
                        );
                    }
                    
                    ids_col = ids_col.push(
                        container(
                            scrollable(device_list).height(Length::Fixed(200.0))
                        )
                        .padding(5)
                        .style(|_theme| container::Style {
                            background: Some(iced::Color::from_rgb(0.2, 0.2, 0.2).into()),
                            ..Default::default()
                        })
                    );
                }

                let sidebar_layers = column![
                    text("Target Devices (IDs)").size(18),
                    ids_col,
                    Space::new().height(Length::Fixed(20.0)),
                    text("Layers").size(18),
                    layer_search_input,
                    new_layer_button,
                    scrollable(layer_list_col).height(Length::Fill)
                ]
                .spacing(10)
                .width(Length::Fixed(200.0))
                .padding(10);

                let layer_editor_area = container(
                    match &self.active_layer {
                        Some(layer_name) => {
                            let layer = config.layers.iter().find(|l| l.name == *layer_name);
                            let modifiers_vec = layer.map(|l| &l.modifiers).cloned().unwrap_or_default();
                            let current_modifiers_str = if modifiers_vec.is_empty() {
                                String::new()
                            } else {
                                modifiers_vec.iter().map(|c| c.to_string()).collect::<Vec<String>>().join("-")
                            };
                            
                            let mut column_content = column![
                                row![
                                    text(format!("Editing: {} > [{}]", device_config_name, layer_name)).size(20),
                                    if !current_modifiers_str.is_empty() {
                                        text(format!(" (Modifiers: {})", current_modifiers_str)).size(16).color([0.5, 0.5, 0.8])
                                    } else {
                                        text("")
                                    },
                                    Space::new().width(Length::Fill),
                                    button("Apply ALL to System")
                                        .on_press(Message::ApplyConfiguration)
                                        .padding(10)
                                        .style(button::primary)
                                ].align_y(Alignment::Center).spacing(10),
                            ].spacing(10);

                            let mut mods_area = column![].spacing(10);
                            let mod_button = if layer_name != "main" {
                                button("Modifiers").on_press(Message::ToggleModifiersMenu).padding(8)
                            } else {
                                button("Modifiers").padding(8)
                            };
                            mods_area = mods_area.push(mod_button);

                            if layer_name != "main" && self.show_modifiers {
                                let mods_row = row![
                                    row![checkbox(modifiers_vec.contains(&'C')).on_toggle(|c| Message::ToggleModifier('C', c)), text("Control (C)")].spacing(5).align_y(Alignment::Center),
                                    row![checkbox(modifiers_vec.contains(&'M')).on_toggle(|c| Message::ToggleModifier('M', c)), text("Meta (M)")].spacing(5).align_y(Alignment::Center),
                                    row![checkbox(modifiers_vec.contains(&'A')).on_toggle(|c| Message::ToggleModifier('A', c)), text("Alt (A)")].spacing(5).align_y(Alignment::Center),
                                    row![checkbox(modifiers_vec.contains(&'S')).on_toggle(|c| Message::ToggleModifier('S', c)), text("Shift (S)")].spacing(5).align_y(Alignment::Center),
                                    row![checkbox(modifiers_vec.contains(&'G')).on_toggle(|c| Message::ToggleModifier('G', c)), text("AltGr (G)")].spacing(5).align_y(Alignment::Center),
                                ].spacing(20);
                                
                                mods_area = mods_area.push(
                                    container(mods_row)
                                        .padding(15)
                                        .style(|_theme| container::Style {
                                            background: Some(iced::Color::from_rgb(0.2, 0.2, 0.2).into()),
                                            ..Default::default()
                                        })
                                );
                            } else {
                                mods_area = mods_area.push(container(text("")).height(Length::Fixed(0.0)));
                            }

                            column_content = column_content.push(
                                container(mods_area).height(Length::Shrink)
                            ).push(Space::new().height(Length::Fixed(10.0))).push(text("Mapping Rules").size(18));

                            let mut mappings_list = column![].spacing(10);
                            if let Some(layer) = layer {
                                let mut sorted_keys: Vec<_> = layer.mappings.keys().collect();
                                sorted_keys.sort();

                                for key in sorted_keys {
                                    let value = layer.mappings.get(key).unwrap();
                                    
                                    let current_type = if value.starts_with("overloadt(") { ActionType::OverloadT }
                                        else if value.starts_with("overload(") { ActionType::Overload }
                                        else if value.starts_with("timeout(") { ActionType::Timeout }
                                        else if value.starts_with("macro(") { ActionType::Macro }
                                        else if value.starts_with("layer(") { ActionType::Layer }
                                        else if value.starts_with("swap(") { ActionType::Swap }
                                        else if value.starts_with("command(") { ActionType::Command }
                                        else { ActionType::Simple };

                                    let raw_args = if let Some(start) = value.find('(') {
                                        let end = value.rfind(')').unwrap_or(value.len());
                                        &value[start+1..end]
                                    } else {
                                        value.as_str()
                                    };
                                    let args_vec = Self::split_args(raw_args);

                                    let is_recording_key = self.recording_mapping.as_ref() == Some(key);

                                    let mut mapping_row = row![
                                        button(text(if is_recording_key { "Recording..." } else { key })).on_press(Message::StartMappingRecording(key.clone())).width(Length::Fixed(120.0)),
                                        text("=").size(20),
                                        pick_list(
                                            &ActionType::ALL[..],
                                            Some(current_type),
                                            {
                                                let k = key.clone();
                                                move |action_type| Message::ChangeActionType(k.clone(), action_type)
                                            }
                                        ).width(Length::Fixed(120.0)),
                                    ].spacing(10).align_y(Alignment::Center);

                                    let schema = current_type.args_schema();
                                    for (i, arg_type) in schema.iter().enumerate() {
                                        let arg_value = args_vec.get(i).cloned().unwrap_or_default();
                                        let is_recording_arg = self.recording_arg.as_ref() == Some(&(key.clone(), i));

                                        let arg_widget: Element<Message> = match arg_type {
                                            ArgType::Key => {
                                                button(text(if is_recording_arg { "Press key...".to_string() } else if arg_value.is_empty() { "void".to_string() } else { arg_value.clone() }))
                                                    .on_press(Message::StartArgRecording(key.clone(), i))
                                                    .into()
                                            }
                                            ArgType::Layer => {
                                                text_input("Layer...", &arg_value)
                                                    .on_input({
                                                        let k = key.clone();
                                                        move |v| Message::ChangeActionArg(k.clone(), i, v)
                                                    })
                                                    .padding(5)
                                                    .width(Length::Fixed(100.0))
                                                    .into()
                                            }
                                            ArgType::Timeout => {
                                                text_input("ms", &arg_value)
                                                    .on_input({
                                                        let k = key.clone();
                                                        move |v| Message::ChangeActionArg(k.clone(), i, v)
                                                    })
                                                    .padding(5)
                                                    .width(Length::Fixed(60.0))
                                                    .into()
                                            }
                                            ArgType::Text => {
                                                text_input("Value...", &arg_value)
                                                    .on_input({
                                                        let k = key.clone();
                                                        move |v| Message::ChangeActionArg(k.clone(), i, v)
                                                    })
                                                    .padding(5)
                                                    .width(Length::Fill)
                                                    .into()
                                            }
                                        };
                                        mapping_row = mapping_row.push(arg_widget);
                                    }

                                    mapping_row = mapping_row.push(Space::new().width(Length::Fill)).push(
                                        button(text(" X ")).on_press(Message::DeleteMapping(key.clone())).style(button::danger)
                                    ).padding(iced::Padding::new(0.0).right(20.0));

                                    mappings_list = mappings_list.push(mapping_row);
                                }
                            }

                            column_content = column_content.push(
                                container(
                                    scrollable(
                                        column![
                                            mappings_list,
                                            button("+ Add Rule").on_press(Message::AddMapping).padding(10)
                                        ]
                                        .spacing(20)
                                        .width(Length::Fill)
                                    )
                                    .height(Length::Fill)
                                    .width(Length::Fill)
                                )
                                .padding(10)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .style(|_theme| container::Style {
                                    background: Some(iced::Color::from_rgb(0.12, 0.12, 0.12).into()),
                                    ..Default::default()
                                })
                            );

                            column_content
                        }
                        None => column![text("Select a layer to edit")]
                    }
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20);

                row![
                    sidebar_layers,
                    container(layer_editor_area).style(|_theme| container::Style {
                        background: Some(iced::Color::from_rgb(0.15, 0.15, 0.15).into()),
                        ..Default::default()
                    })
                ]
                .into()
            }
            (Some(_profile), None) => {
                container(text("Select a device to start editing.")).center_x(Length::Fill).center_y(Length::Fill).into()
            }
            _ => {
                container(text("Select a profile from the left or create a new one to start editing."))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
        };

        let body = row![
            sidebar,
            middle_content,
            container(right_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(container::dark)
        ]
        .height(Length::Fill);

        column![
            body,
            row![text(&self.status_message).size(12)].padding(5)
        ]
        .height(Length::Fill)
        .into()
    }
}
