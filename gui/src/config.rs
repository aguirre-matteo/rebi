use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Layer {
    pub name: String,
    pub modifiers: Vec<char>,
    pub mappings: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct KeydConfig {
    pub ids: Vec<String>,
    pub layers: Vec<Layer>,
}

impl KeydConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Ok(Self::from_str(&content))
    }

    pub fn from_str(s: &str) -> Self {
        let mut layers = Vec::new();
        let mut ids = Vec::new();
        let mut current_layer: Option<Layer> = None;
        let mut in_ids = false;

        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                // When finding a new section, save the previous one if it exists
                if let Some(layer) = current_layer.take() {
                    layers.push(layer);
                }
                in_ids = false;

                let section_name = &line[1..line.len()-1];
                if section_name == "ids" {
                    in_ids = true;
                } else {
                    let mut parts = section_name.splitn(2, ':');
                    let name = parts.next().unwrap_or("main").to_string();
                    let modifiers_str = parts.next().unwrap_or("");
                    
                    let mut modifiers = if modifiers_str.is_empty() {
                        Vec::new()
                    } else {
                        modifiers_str.split('-').filter_map(|s| s.chars().next()).collect()
                    };
                    
                    Self::sort_modifiers(&mut modifiers);
                    current_layer = Some(Layer { name, modifiers, mappings: HashMap::new() });
                }
            } else if in_ids {
                if !line.is_empty() {
                    ids.push(line.to_string());
                }
            } else if let Some(ref mut layer) = current_layer {
                if let Some((key, value)) = line.split_once('=') {
                    layer.mappings.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        // Don't forget the last layer
        if let Some(layer) = current_layer {
            layers.push(layer);
        }

        // Ensure 'main' is always present and at the beginning
        if let Some(pos) = layers.iter().position(|l| l.name == "main") {
            let main_layer = layers.remove(pos);
            layers.insert(0, main_layer);
        } else {
            layers.insert(0, Layer { name: "main".to_string(), modifiers: Vec::new(), mappings: HashMap::new() });
        }

        // If ids is empty, default to '*'
        if ids.is_empty() {
            ids.push("*".to_string());
        }

        KeydConfig { ids, layers }
    }

    pub fn to_string(&self) -> String {
        let mut output = String::new();
        
        // [ids] section
        output.push_str("[ids]\n");
        for id in &self.ids {
            output.push_str(&format!("{}\n", id));
        }
        output.push_str("\n");

        for layer in &self.layers {
            if layer.modifiers.is_empty() {
                output.push_str(&format!("[{}]\n", layer.name));
            } else {
                let mut sorted_mods = layer.modifiers.clone();
                Self::sort_modifiers(&mut sorted_mods);
                let mods: Vec<String> = sorted_mods.iter().map(|c| c.to_string()).collect();
                output.push_str(&format!("[{}:{}]\n", layer.name, mods.join("-")));
            }
            
            let mut keys: Vec<_> = layer.mappings.keys().collect();
            keys.sort();
            for key in keys {
                output.push_str(&format!("{} = {}\n", key, layer.mappings.get(key).unwrap()));
            }
            output.push_str("\n");
        }

        output
    }

    pub fn export_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.to_string().as_bytes())?;
        Ok(())
    }

    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }

    pub fn remove_layer(&mut self, name: &str) {
        if name != "main" {
            self.layers.retain(|l| l.name != name);
        }
    }

    pub fn add_layer(&mut self, name: String) {
        if !self.layers.iter().any(|l| l.name == name) {
            self.layers.push(Layer {
                name,
                modifiers: Vec::new(),
                mappings: HashMap::new(),
            });
        }
    }

    fn sort_modifiers(mods: &mut Vec<char>) {
        let order = "CMASG";
        mods.sort_by_key(|c| order.find(*c).unwrap_or(order.len()));
    }
}
