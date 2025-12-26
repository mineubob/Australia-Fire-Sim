//! Interactive Fire Simulation Demo with Ratatui UI
//!
//! A terminal-based interactive debugger for the fire simulation with enhanced UI.
//! Uses ratatui for rich terminal rendering with multiple panels.
//!
//! # Usage
//!
//! ## Interactive Mode (default)
//! ```bash
//! cargo run --package demo-interactive
//! ```
//!
//! ## Headless Mode
//! ```bash
//! cargo run --package demo-interactive -- --headless
//! # or
//! echo "50\n50\ni 100\ns 10\nq" | cargo run --package demo-interactive -- --headless
//! ```
//!
//! # Commands
//!
//! - `step [n]` or `s [n]` - Advance simulation by n timesteps (default 1)
//! - `status` or `st` - Show simulation status
//! - `weather` or `w` - Show weather conditions
//! - `element <id>` or `e <id>` - Show element details
//! - `burning` or `b` - List all burning elements
//! - `embers` or `em` - List all active embers
//! - `nearby <id>` or `n <id>` - Show elements near the specified element
//! - `ignite <id>` or `i <id>` - Manually ignite an element
//! - `ignite_position <x> <y> [radius] [amount] [filters]` or `ip` - Ignite elements in XY circle
//! - `heat <id> <temperature>` or `h` - Apply heat to an element
//! - `heat_position <x> <y> <temp> [radius] [amount] [filters]` or `hp` - Heat elements in XY circle
//! - `preset <name>` or `p <name>` - Switch weather preset
//! - `time <hours>` or `t <hours>` - Set time of day (0-24 hours)
//! - `setday <day>` or `sd <day>` - Set day of year (1-365)
//! - `reset [w] [h]` or `r` - Reset simulation
//! - `heatmap [size]` or `hm` - Generate a heatmap
//! - `help` or `?` - Show available commands
//! - `quit` or `q` - Exit the simulation
//!
//! # Filters for Position Commands
//!
//! The `ignite_position` and `heat_position` commands support optional filters:
//!
//! - `fuel=<name>` - Filter by fuel type (e.g., `fuel=eucalyptus`, `fuel=savanna`)
//! - `part=<type>` - Filter by fuel part type (e.g., `part=crown`, `part=root`, `part=groundlitter`)
//! - `minz=<height>` - Minimum height in meters (e.g., `minz=0`)
//! - `maxz=<height>` - Maximum height in meters (e.g., `maxz=20`)
//!
//! # Examples

use fire_sim_core::{
    core_types::{Celsius, Degrees, Kilograms, KilometersPerHour, Meters, Percent},
    ClimatePattern, FireSimulation, Fuel, FuelPart, TerrainData, Vec3, WeatherPreset,
    WeatherSystem,
};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::{
    io::{self, Write},
    time::Instant,
};

/// Default terrain dimensions
const DEFAULT_WIDTH: f32 = 150.0;
const DEFAULT_HEIGHT: f32 = 150.0;

/// Command information for help generation and execution
///
/// Stores metadata about a command including its name, aliases, usage, description, category,
/// and a handler function that executes the command.
#[derive(Clone, Copy)]
struct CommandInfo {
    /// Primary command name
    name: &'static str,
    /// Short alias(es) (comma-separated if multiple)
    alias: &'static str,
    /// Usage/parameters description
    usage: &'static str,
    /// Description of what the command does
    description: &'static str,
    /// Category for grouping in help
    category: &'static str,
    /// Handler function that executes this command
    handler: fn(&mut App, &[&str]),
}

impl CommandInfo {
    /// Check if this command matches the given command string (name or alias)
    fn matches(&self, cmd: &str) -> bool {
        cmd == self.name || (!self.alias.is_empty() && cmd == self.alias)
    }
}

/// All available commands in the simulation
const COMMANDS: &[CommandInfo] = &[
    // Simulation Control
    CommandInfo {
        name: "step",
        alias: "s",
        usage: "[n]",
        description: "Advance n timesteps (default 1)",
        category: "Simulation Control",
        handler: |app, parts| {
            let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            app.step_simulation(count);
        },
    },
    CommandInfo {
        name: "reset",
        alias: "r",
        usage: "[w] [h]",
        description: "Reset simulation (optional: new width/height)",
        category: "Simulation Control",
        handler: |app, parts| {
            let new_width = parts
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(app.terrain_width);
            let new_height = parts
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(app.terrain_height);
            app.reset_simulation(new_width, new_height);
        },
    },
    CommandInfo {
        name: "preset",
        alias: "p",
        usage: "<name>",
        description: "Change weather preset (perth, catastrophic, south_west, wheatbelt, goldfields, kimberley, pilbara, hot)",
        category: "Simulation Control",
        handler: |app, parts| {
            if let Some(name) = parts.get(1) {
                app.set_preset(name);
            } else {
                app.add_message(
                    "Usage: preset <perth|catastrophic|south_west|wheatbelt|goldfields|kimberley|pilbara|hot>".to_string(),
                );
            }
        },
    },
    CommandInfo {
        name: "time",
        alias: "t",
        usage: "<hours>",
        description: "Set time of day (0-24 hours)",
        category: "Simulation Control",
        handler: |app, parts| {
            if let Some(hours) = parts.get(1).and_then(|s| s.parse().ok()) {
                app.set_time(hours);
            } else {
                app.add_message("Usage: time <hours> (0-24)".to_string());
            }
        },
    },
    CommandInfo {
        name: "setday",
        alias: "sd",
        usage: "<day>",
        description: "Set day of year (1-365)",
        category: "Simulation Control",
        handler: |app, parts| {
            if let Some(day) = parts.get(1).and_then(|s| s.parse().ok()) {
                app.set_day(day);
            } else {
                app.add_message("Usage: setday <day> (1-365)".to_string());
            }
        },
    },
    // Information Commands
    CommandInfo {
        name: "status",
        alias: "st",
        usage: "",
        description: "Show simulation status",
        category: "Information Commands",
        handler: |app, _parts| {
            if app.headless {
                app.show_status_text();
            } else {
                app.view_mode = ViewMode::Status;
                app.add_message("Switched to Status view".to_string());
            }
        },
    },
    CommandInfo {
        name: "weather",
        alias: "w",
        usage: "",
        description: "Show weather conditions",
        category: "Information Commands",
        handler: |app, _parts| {
            if app.headless {
                app.show_weather_text();
            } else {
                app.view_mode = ViewMode::Weather;
                app.add_message("Switched to Weather view".to_string());
            }
        },
    },
    CommandInfo {
        name: "dashboard",
        alias: "d",
        usage: "",
        description: "Switch to dashboard view",
        category: "Information Commands",
        handler: |app, _parts| {
            if app.headless {
                app.add_message(
                    "Dashboard view is only available in interactive mode".to_string(),
                );
            } else {
                app.view_mode = ViewMode::Dashboard;
                app.add_message("Switched to Dashboard view".to_string());
            }
        },
    },
    CommandInfo {
        name: "heatmap",
        alias: "hm",
        usage: "[size]",
        description: "Show temperature heatmap (default size: 30)",
        category: "Information Commands",
        handler: |app, parts| {
            let grid_size = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
            app.heatmap_size = grid_size;
            if app.headless {
                app.show_heatmap_text(grid_size);
            } else {
                // Pre-build cache for interactive view so the heatmap is not regenerated every frame
                app.ensure_heatmap_cache(grid_size);
                app.view_mode = ViewMode::Heatmap;
                app.add_message("Switched to Heatmap view".to_string());
            }
        },
    },
    CommandInfo {
        name: "help",
        alias: "?",
        usage: "",
        description: "Show this help",
        category: "Information Commands",
        handler: |app, _parts| {
            if app.headless {
                app.show_help_text();
            } else {
                app.view_mode = ViewMode::Help;
                app.add_message("Switched to Help view".to_string());
            }
        },
    },
    // Element Commands
    CommandInfo {
        name: "element",
        alias: "e",
        usage: "<id>",
        description: "Show element details",
        category: "Element Commands",
        handler: |app, parts| {
            if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                app.show_element(id);
            } else {
                app.add_message("Usage: element <id>".to_string());
            }
        },
    },
    CommandInfo {
        name: "burning",
        alias: "b",
        usage: "",
        description: "List burning elements",
        category: "Element Commands",
        handler: |app, _parts| app.show_burning(),
    },
    CommandInfo {
        name: "embers",
        alias: "em",
        usage: "",
        description: "List active embers",
        category: "Element Commands",
        handler: |app, _parts| app.show_embers(),
    },
    CommandInfo {
        name: "nearby",
        alias: "n",
        usage: "<id>",
        description: "Show elements near <id>",
        category: "Element Commands",
        handler: |app, parts| {
            if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                app.show_nearby(id);
            } else {
                app.add_message("Usage: nearby <id>".to_string());
            }
        },
    },
    CommandInfo {
        name: "ignite",
        alias: "i",
        usage: "<id>",
        description: "Manually ignite element",
        category: "Element Commands",
        handler: |app, parts| {
            if let Some(id) = parts.get(1).and_then(|s| s.parse().ok()) {
                app.ignite_element(id);
            } else {
                app.add_message("Usage: ignite <id>".to_string());
            }
        },
    },
    CommandInfo {
        name: "heat",
        alias: "h",
        usage: "<id> <temp>",
        description: "Heat element to target temperature (°C)",
        category: "Element Commands",
        handler: |app, parts| {
            if let (Some(id), Some(temperature)) = (
                parts.get(1).and_then(|s| s.parse().ok()),
                parts.get(2).and_then(|s| s.parse().ok()),
            ) {
                app.heat_element(id, temperature);
            } else {
                app.add_message("Usage: heat <id> <temperature>".to_string());
            }
        },
    },
    // Position Commands
    CommandInfo {
        name: "ignite_position",
        alias: "ip",
        usage: "<x> <y> [radius] [amount] [filters]",
        description: "Ignite elements in an XY circle",
        category: "Position Commands",
        handler: |app, parts| app.ignite_position(parts),
    },
    CommandInfo {
        name: "heat_position",
        alias: "hp",
        usage: "<x> <y> <temp> [radius] [amount] [filters]",
        description: "Heat elements to target temperature",
        category: "Position Commands",
        handler: |app, parts| app.heat_position(parts),
    },
    // Analysis Commands
    CommandInfo {
        name: "shape",
        alias: "sh",
        usage: "",
        description: "Analyze fire shape/asymmetry (wind effect)",
        category: "Information Commands",
        handler: |app, _parts| app.analyze_fire_shape(),
    },
    // Control Commands
    CommandInfo {
        name: "quit",
        alias: "q",
        usage: "",
        description: "Exit simulation",
        category: "Controls",
        handler: |app, _parts| app.should_quit = true,
    },
];

/// Burning list sort mode
///
/// Controls how the burning elements list is sorted in the UI.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BurningSortMode {
    /// Sort by temperature (ascending) - coolest first
    TemperatureAsc,
    /// Sort by temperature (descending) - hottest first
    TemperatureDesc,
    /// Sort by time since ignition (ascending) - oldest fires first
    TimeSinceIgnitionAsc,
    /// Sort by time since ignition (descending) - newest fires first
    TimeSinceIgnitionDesc,
}

impl BurningSortMode {
    /// Get the next sort mode in the cycle when toggling
    fn next_mode(&self) -> Self {
        match self {
            BurningSortMode::TemperatureAsc => BurningSortMode::TemperatureDesc,
            BurningSortMode::TemperatureDesc => BurningSortMode::TimeSinceIgnitionAsc,
            BurningSortMode::TimeSinceIgnitionAsc => BurningSortMode::TimeSinceIgnitionDesc,
            BurningSortMode::TimeSinceIgnitionDesc => BurningSortMode::TemperatureAsc,
        }
    }
}

/// Application state for the interactive fire simulation demo
///
/// Manages the simulation, UI state, command history, and user interaction.
/// Supports both interactive TUI mode and headless mode for automation.
struct App {
    /// The fire simulation
    sim: FireSimulation,
    /// Current terrain width
    terrain_width: f32,
    /// Current terrain height
    terrain_height: f32,
    /// Current weather preset
    current_weather: WeatherPreset,
    /// Command input buffer
    input: String,
    /// Command history
    history: Vec<String>,
    /// History position for navigation
    history_pos: usize,
    /// Messages to display
    messages: Vec<String>,
    /// Message scroll offset
    message_scroll: usize,
    /// Simulation step count
    step_count: u32,
    /// Total elapsed simulation time
    elapsed_time: f32,
    /// Should quit
    should_quit: bool,
    /// Current view mode
    view_mode: ViewMode,
    /// Heatmap grid size
    heatmap_size: usize,
    /// Cached heatmap rendered output for the current simulation step
    heatmap_cache: Option<HeatmapCache>,
    /// Steps remaining to process (for non-blocking stepping)
    steps_remaining: u32,
    /// Total steps in current batch
    steps_total: u32,
    /// Headless mode (no UI)
    headless: bool,
    /// Burning list sort mode
    burning_sort_mode: BurningSortMode,
    /// Ignition times (`element_id` -> `step_count` when ignited)
    ignition_times: std::collections::HashMap<usize, u32>,
    /// Whether we're currently in stepping mode (used to filter allowed commands)
    is_stepping: bool,
}

/// Cached representation of the heatmap for fast re-rendering
/// Metadata for a single heatmap cell
#[derive(Clone, Debug)]
struct HeatmapCell {
    /// Average temperature in this cell (°C)
    temperature: f32,
    /// Number of elements in this cell
    element_count: u32,
}

struct HeatmapCache {
    /// The simulation step this cache corresponds to
    step_count: u32,
    /// The grid size used for the heatmap
    size: usize,
    /// Grid of cell metadata (row-major order: [y][x])
    cells: Vec<Vec<HeatmapCell>>,
    /// Global min temperature across all cells (for gradient scaling)
    min_temp: f32,
    /// Global max temperature across all cells (for gradient scaling)
    max_temp: f32,
}

/// UI view modes
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    /// Main dashboard view.
    /// Shows the overall simulation state, terrain, and summary statistics.
    Dashboard,
    /// Detailed status view.
    /// Displays in-depth information about burning elements and simulation internals.
    Status,
    /// Weather details view.
    /// Presents current weather conditions and allows inspection of weather parameters.
    Weather,
    /// Help view.
    /// Shows available commands and usage instructions for the interactive UI.
    Help,
    /// Heatmap view.
    /// Visualizes simulation data (e.g., temperature, intensity) as a heatmap overlay.
    Heatmap,
}

impl App {
    /// Create a new application with default settings (interactive mode)
    fn new(width: f32, height: f32) -> Self {
        Self::new_with_mode(width, height, false)
    }

    /// Create a new application with specified mode
    ///
    /// # Arguments
    /// * `width` - Terrain width in meters
    /// * `height` - Terrain height in meters
    /// * `headless` - If true, runs without TUI for automation
    fn new_with_mode(width: f32, height: f32, headless: bool) -> Self {
        let weather = WeatherPreset::perth_metro();
        let sim = create_test_simulation(width, height, weather.clone());
        let element_count = sim.get_all_elements().len();
        let using_gpu = sim.is_using_gpu();

        Self {
            sim,
            terrain_width: width,
            terrain_height: height,
            current_weather: weather,
            input: String::new(),
            history: Vec::new(),
            history_pos: 0,
            messages: vec![
                "Welcome to Bushfire Simulation!".to_string(),
                format!(
                    "Created simulation with {element_count} elements on {width}x{height} terrain with {}", 
        using_gpu.then(|| "GPU").unwrap_or("no GPU")
                ),
                "Type 'help' for available commands.".to_string(),
            ],
            message_scroll: 0,
            step_count: 0,
            elapsed_time: 0.0,
            should_quit: false,
            view_mode: ViewMode::Dashboard,
            heatmap_size: 30,
            heatmap_cache: None,
            steps_remaining: 0,
            steps_total: 0,
            headless,
            burning_sort_mode: BurningSortMode::TemperatureDesc,
            ignition_times: std::collections::HashMap::new(),
            is_stepping: false,
        }
    }

    /// Add a message to the message log
    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        // Keep last 1000 messages to prevent unbounded growth
        if self.messages.len() > 1000 {
            self.messages.drain(0..500);
        }
    }

    /// Execute a command entered by the user
    fn execute_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return;
        }

        // Add to history
        if !command.is_empty() {
            self.history.push(command.to_string());
            self.history_pos = self.history.len();
        }

        // Check if command is allowed during stepping
        if self.is_stepping && !Self::is_command_allowed_during_stepping(parts[0]) {
            self.add_message(format!(
                "Command '{}' not allowed during stepping. Press Ctrl+C to stop stepping.",
                parts[0]
            ));
            return;
        }

        let cmd_str = parts[0].to_lowercase();

        // Find and execute the command from the COMMANDS array
        if let Some(cmd_info) = COMMANDS.iter().find(|c| c.matches(&cmd_str)) {
            (cmd_info.handler)(self, &parts);
        } else {
            // Handle special aliases not in COMMANDS (like "exit" for "quit")
            match cmd_str.as_str() {
                "exit" => self.should_quit = true,
                _ => {
                    self.add_message(format!(
                        "Unknown command: '{}'. Type 'help' for available commands.",
                        parts[0]
                    ));
                }
            }
        }
    }

    /// Step the simulation forward (sets up stepping state)
    /// Step the simulation forward by the specified number of timesteps
    ///
    /// In interactive mode, this sets up non-blocking stepping that processes
    /// one step per frame to keep the UI responsive.
    fn step_simulation(&mut self, count: u32) {
        self.add_message(format!("Stepping {count} timestep(s)..."));
        self.steps_remaining = count;
        self.steps_total = count;
        self.is_stepping = true;
    }

    /// Process one simulation step (called from event loop)
    ///
    /// Updates the simulation by one timestep and tracks newly ignited elements.
    fn process_one_step(&mut self) {
        if self.steps_remaining == 0 {
            return;
        }

        let dt = 1.0;
        let burning_before = self.sim.get_burning_elements().len();
        let embers_before = self.sim.ember_count();
        let start = Instant::now();

        // Track which elements were burning before
        let burning_ids_before: std::collections::HashSet<_> = self
            .sim
            .get_burning_elements()
            .iter()
            .map(|e| e.get_stats().id)
            .collect();

        self.sim.update(dt);
        self.step_count += 1;
        self.elapsed_time += dt;

        // Track newly ignited elements
        let burning_ids_after: std::collections::HashSet<_> = self
            .sim
            .get_burning_elements()
            .iter()
            .map(|e| e.get_stats().id)
            .collect();

        for id in burning_ids_after.difference(&burning_ids_before) {
            self.ignition_times.insert(*id, self.step_count);
        }

        let burning_after = self.sim.get_burning_elements().len();
        let embers_after = self.sim.ember_count();
        let time = start.elapsed();

        let current_step = self.steps_total - self.steps_remaining + 1;

        // Log significant changes or every 10th step or the last step
        if current_step == self.steps_total
            || current_step.is_multiple_of(10)
            || burning_after != burning_before
            || embers_after != embers_before
        {
            self.add_message(format!(
                "Step {}: Burning: {} → {}, Embers: {} → {}, Time: {}ms",
                current_step,
                burning_before,
                burning_after,
                embers_before,
                embers_after,
                time.as_millis()
            ));
        }

        self.steps_remaining -= 1;

        if self.steps_remaining == 0 {
            self.add_message("Done.".to_string());
            self.is_stepping = false;
        }
        // Invalidate heatmap cache since simulation state changed
        self.invalidate_heatmap_cache();
        // If the heatmap is visible, rebuild the cache for the new step so the UI can render cached data
        if self.view_mode == ViewMode::Heatmap {
            self.ensure_heatmap_cache(self.heatmap_size);
        }
    }

    /// Invalidate cached heatmap (call when simulation state changes)
    fn invalidate_heatmap_cache(&mut self) {
        self.heatmap_cache = None;
    }

    /// Ensure heatmap cache exists for `grid_size` and current step; build it if missing
    fn ensure_heatmap_cache(&mut self, grid_size: usize) {
        let needs_build = match &self.heatmap_cache {
            Some(c) => c.step_count != self.step_count || c.size != grid_size,
            None => true,
        };

        if needs_build {
            let (cells, min_temp, max_temp) = self.build_heatmap(grid_size);
            self.heatmap_cache = Some(HeatmapCache {
                step_count: self.step_count,
                size: grid_size,
                cells,
                min_temp,
                max_temp,
            });
        }
    }

    /// Build the heatmap representation with rich metadata
    ///
    /// Returns (`cells`, `min_temp`, `max_temp`) where:
    /// - `cells`: 2D grid of `HeatmapCell` with per-cell metadata
    /// - `min_temp`/`max_temp`: global temperature range for gradient scaling
    fn build_heatmap(&self, grid_size: usize) -> (Vec<Vec<HeatmapCell>>, f32, f32) {
        let cell_width = self.terrain_width / usize_to_f32(grid_size);
        let cell_height = self.terrain_height / usize_to_f32(grid_size);

        let mut temp_sum: Vec<Vec<f32>> = vec![vec![0.0; grid_size]; grid_size];
        let mut counts: Vec<Vec<u32>> = vec![vec![0; grid_size]; grid_size];

        // Accumulate element data into grid cells
        for e in self.sim.get_all_elements() {
            let stats = e.get_stats();
            let x = (stats.position.x / cell_width).floor() as i32;
            let y = (stats.position.y / cell_height).floor() as i32;

            if x >= 0 && x < grid_size as i32 && y >= 0 && y < grid_size as i32 {
                let ix = x as usize;
                let iy = y as usize;
                temp_sum[iy][ix] += stats.temperature;
                counts[iy][ix] += 1;
            }
        }

        // Build structured cell metadata
        let mut cells: Vec<Vec<HeatmapCell>> = Vec::with_capacity(grid_size);
        for y in 0..grid_size {
            let mut row = Vec::with_capacity(grid_size);
            for x in 0..grid_size {
                let avg_temp = if counts[y][x] > 0 {
                    temp_sum[y][x] / u32_to_f32(counts[y][x])
                } else {
                    0.0
                };
                row.push(HeatmapCell {
                    temperature: avg_temp,
                    element_count: counts[y][x],
                });
            }
            cells.push(row);
        }

        // Calculate global temperature range
        let mut min_temp = f32::MAX;
        let mut max_temp = f32::MIN;
        for row in &cells {
            for cell in row {
                if cell.element_count > 0 {
                    min_temp = min_temp.min(cell.temperature);
                    max_temp = max_temp.max(cell.temperature);
                }
            }
        }

        if min_temp == f32::MAX {
            min_temp = 0.0;
            max_temp = 0.0;
        }

        (cells, min_temp, max_temp)
    }

    /// Check if a command is allowed during stepping
    ///
    /// View commands and navigation are allowed, but simulation-modifying
    /// commands (ignite, heat, step, reset, etc.) are blocked.
    fn is_command_allowed_during_stepping(cmd: &str) -> bool {
        matches!(
            cmd.to_lowercase().as_str(),
            "status"
                | "st"
                | "weather"
                | "w"
                | "dashboard"
                | "d"
                | "help"
                | "?"
                | "element"
                | "e"
                | "burning"
                | "b"
                | "embers"
                | "em"
                | "nearby"
                | "n"
                | "heatmap"
                | "hm"
                | "quit"
                | "q"
        )
    }

    /// Show element details by ID
    fn show_element(&mut self, id: usize) {
        if let Some(e) = self.sim.get_element(id) {
            let stats = e.get_stats();
            let fuel_name = e.fuel().name.clone();
            let part_type = format!("{:?}", stats.part_type);

            self.add_message(format!("═══════════════ ELEMENT {id} ═══════════════"));
            self.add_message(format!(
                "Position: ({:.1}, {:.1}, {:.1})",
                stats.position.x, stats.position.y, stats.position.z
            ));
            self.add_message(format!("Fuel Type: {fuel_name}"));
            self.add_message(format!("Part Type: {part_type}"));
            self.add_message(format!("Temperature: {:.1}°C", stats.temperature));
            self.add_message(format!(
                "Ignition Temp: {:.1}°C",
                stats.ignition_temperature
            ));
            self.add_message(format!("Ignited: {}", stats.ignited));
            self.add_message(format!("Moisture: {:.1}%", stats.moisture_fraction * 100.0));
            self.add_message(format!("Fuel Mass: {:.2} kg", stats.fuel_remaining));
        } else {
            self.add_message(format!("Element {id} not found"));
        }
    }

    /// Show list of currently burning elements
    fn show_burning(&mut self) {
        let burning_elements = self.sim.get_burning_elements();
        if burning_elements.is_empty() {
            self.add_message("No elements are currently burning.".to_string());
        } else {
            let count = burning_elements.len();
            let messages: Vec<String> = burning_elements
                .iter()
                .take(10)
                .map(|e| {
                    let stats = e.get_stats();
                    format!(
                        "ID {:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>8.1}% {:>7.2}kg",
                        stats.id,
                        stats.position.x,
                        stats.position.y,
                        stats.position.z,
                        stats.temperature,
                        stats.moisture_fraction * 100.0,
                        stats.fuel_remaining
                    )
                })
                .collect();

            self.add_message(format!(
                "═══════════════ {count} BURNING ELEMENTS ═══════════════"
            ));
            for msg in messages {
                self.add_message(msg);
            }
            if count > 10 {
                let more = count - 10;
                self.add_message(format!("... and {more} more"));
            }
        }
    }

    /// Show list of active embers
    /// Show list of active embers
    fn show_embers(&mut self) {
        let ember_count = self.sim.ember_count();
        self.add_message(format!("Active embers: {ember_count}"));
    }

    /// Analyze fire shape to measure wind-driven asymmetry
    ///
    /// Calculates the extent of the fire in each direction relative to the
    /// centroid of burning elements, providing metrics for fire elongation.
    fn analyze_fire_shape(&mut self) {
        let burning_elements = self.sim.get_burning_elements();

        if burning_elements.is_empty() {
            self.add_message("No burning elements to analyze.".to_string());
            return;
        }

        // Collect positions of all burning elements
        let positions: Vec<Vec3> = burning_elements.iter().map(|e| *e.position()).collect();

        let count = positions.len();

        // Calculate centroid (center of mass of fire)
        #[expect(
            clippy::cast_precision_loss,
            reason = "count is bounded by element count (typically <100k), precision loss acceptable"
        )]
        let centroid = Vec3::new(
            positions.iter().map(|p| p.x).sum::<f32>() / count as f32,
            positions.iter().map(|p| p.y).sum::<f32>() / count as f32,
            positions.iter().map(|p| p.z).sum::<f32>() / count as f32,
        );

        // Calculate bounding box
        let min_x = positions.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let max_x = positions.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        let min_y = positions.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let max_y = positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);

        // Get wind direction from weather
        let wind_vec = self.sim.get_weather().wind_vector();
        let wind_speed_ms = wind_vec.magnitude();
        let wind_speed_kmh = wind_speed_ms * 3.6; // Convert m/s to km/h
                                                  // Calculate wind direction from vector (degrees, 0=N, 90=E)
        let wind_dir_rad = wind_vec.x.atan2(wind_vec.y);
        let wind_dir_deg = wind_dir_rad.to_degrees();
        let wind_dir_deg = if wind_dir_deg < 0.0 {
            wind_dir_deg + 360.0
        } else {
            wind_dir_deg
        };

        // Calculate extent in wind-aligned coordinates
        let (extent_downwind, extent_upwind, extent_left, extent_right) = if wind_speed_ms > 0.1 {
            let wind_norm = Vec3::new(wind_vec.x / wind_speed_ms, wind_vec.y / wind_speed_ms, 0.0);
            // Perpendicular to wind (left when facing downwind)
            let perp = Vec3::new(-wind_norm.y, wind_norm.x, 0.0);

            let mut dw_max = 0.0f32;
            let mut uw_max = 0.0f32;
            let mut left_max = 0.0f32;
            let mut right_max = 0.0f32;

            for pos in &positions {
                let rel = *pos - centroid;
                let along_wind = rel.x * wind_norm.x + rel.y * wind_norm.y;
                let across_wind = rel.x * perp.x + rel.y * perp.y;

                if along_wind > 0.0 {
                    dw_max = dw_max.max(along_wind);
                } else {
                    uw_max = uw_max.max(-along_wind);
                }

                if across_wind > 0.0 {
                    left_max = left_max.max(across_wind);
                } else {
                    right_max = right_max.max(-across_wind);
                }
            }

            (dw_max, uw_max, left_max, right_max)
        } else {
            // No wind - use cardinal directions
            let extent_pos_x = max_x - centroid.x;
            let extent_neg_x = centroid.x - min_x;
            let extent_pos_y = max_y - centroid.y;
            let extent_neg_y = centroid.y - min_y;
            (extent_pos_x, extent_neg_x, extent_pos_y, extent_neg_y)
        };

        // Calculate ratios
        let lateral_avg = f32::midpoint(extent_left, extent_right);
        let downwind_upwind_ratio = if extent_upwind > 0.1 {
            extent_downwind / extent_upwind
        } else {
            extent_downwind / 0.1
        };
        let downwind_lateral_ratio = if lateral_avg > 0.1 {
            extent_downwind / lateral_avg
        } else {
            extent_downwind / 0.1
        };

        // Output analysis
        self.add_message("═══════════════ FIRE SHAPE ANALYSIS ═══════════════".to_string());
        self.add_message(format!("Burning elements: {count}"));
        self.add_message(format!(
            "Centroid: ({:.1}, {:.1}, {:.1})",
            centroid.x, centroid.y, centroid.z
        ));
        self.add_message(format!(
            "Wind: {wind_speed_kmh:.1} km/h @ {wind_dir_deg:.0}°"
        ));

        // Calculate total fire dimensions
        let total_x = max_x - min_x;
        let total_y = max_y - min_y;
        self.add_message(format!(
            "Bounding box: {total_x:.1}m × {total_y:.1}m (X × Y)"
        ));

        // Calculate wind-aligned dimensions (more meaningful for fire shape)
        let along_wind_length = extent_downwind + extent_upwind;
        let cross_wind_width = extent_left + extent_right;
        let length_width_ratio = if cross_wind_width > 0.1 {
            along_wind_length / cross_wind_width
        } else {
            along_wind_length / 0.1
        };

        if wind_speed_ms > 0.1 {
            self.add_message(format!(
                "Fire shape: {along_wind_length:.1}m long × {cross_wind_width:.1}m wide (L/W = {length_width_ratio:.2})"
            ));
        }
        self.add_message(String::new());
        self.add_message("Extent from centroid:".to_string());

        if wind_speed_ms > 0.1 {
            self.add_message(format!("  Downwind:  {extent_downwind:>6.1}m"));
            self.add_message(format!("  Upwind:    {extent_upwind:>6.1}m"));
            self.add_message(format!("  Left:      {extent_left:>6.1}m"));
            self.add_message(format!("  Right:     {extent_right:>6.1}m"));
        } else {
            self.add_message(format!(
                "  +X: {extent_downwind:>6.1}m    -X: {extent_upwind:>6.1}m"
            ));
            self.add_message(format!(
                "  +Y: {extent_left:>6.1}m    -Y: {extent_right:>6.1}m"
            ));
        }

        self.add_message(String::new());
        self.add_message("Asymmetry ratios:".to_string());
        self.add_message(format!("  Downwind/Upwind:  {downwind_upwind_ratio:>5.2}x"));
        self.add_message(format!(
            "  Downwind/Lateral: {downwind_lateral_ratio:>5.2}x"
        ));
        self.add_message(String::new());

        // Provide interpretation
        if wind_speed_ms < 0.5 {
            self.add_message("Wind too light for significant asymmetry.".to_string());
        } else if downwind_upwind_ratio > 3.0 {
            self.add_message("✓ GOOD elongation - fire shape matches wind.".to_string());
        } else if downwind_upwind_ratio > 1.5 {
            self.add_message("~ Moderate elongation - some wind effect visible.".to_string());
        } else {
            self.add_message("✗ Fire shape is too circular for this wind speed.".to_string());
        }
    }

    /// Show elements nearby the specified element ID
    fn show_nearby(&mut self, id: usize) {
        if let Some(e) = self.sim.get_element(id) {
            let source_pos = *e.position();
            let nearby = self.sim.get_elements_in_radius(source_pos, 15.0);

            let messages: Vec<String> = nearby
                .iter()
                .take(10)
                .filter_map(|n| {
                    let stats = n.get_stats();
                    if stats.id == id {
                        return None;
                    }

                    let dist = (stats.position - source_pos).magnitude();
                    Some(format!(
                        "ID {:<6} ({:>5.1}, {:>5.1}, {:>4.1}) {:>7.1}°C {:>7.1}m {}",
                        stats.id,
                        stats.position.x,
                        stats.position.y,
                        stats.position.z,
                        stats.temperature,
                        dist,
                        if stats.ignited { "🔥" } else { "" }
                    ))
                })
                .collect();

            self.add_message(format!(
                "═══════════════ ELEMENTS NEAR {id} ═══════════════"
            ));
            for msg in messages {
                self.add_message(msg);
            }
        } else {
            self.add_message(format!("Element {id} not found"));
        }
    }

    /// Ignite an element by ID and track its ignition time
    fn ignite_element(&mut self, id: usize) {
        if let Some(e) = self.sim.get_element(id) {
            let stats = e.get_stats();
            let initial_temp = Celsius::new(600.0).max(Celsius::from(stats.ignition_temperature));
            self.sim.ignite_element(id, initial_temp);
            // Track ignition time
            self.ignition_times.insert(id, self.step_count);
            // Invalidate heatmap cache — ignition changes heat distribution
            self.invalidate_heatmap_cache();
            self.add_message(format!(
                "Ignited element {id} at ({:.1}, {:.1}, {:.1})",
                stats.position.x, stats.position.y, stats.position.z
            ));
        } else {
            self.add_message(format!("Element {id} not found"));
        }
    }

    /// Heat an element to a target temperature
    fn heat_element(&mut self, id: usize, target_temp: f32) {
        if let Some(e) = self.sim.get_element(id) {
            let stats = e.get_stats();
            heat_element_to_temp(&mut self.sim, id, target_temp);
            // Heating changes temperatures — invalidate heatmap cache
            self.invalidate_heatmap_cache();
            self.add_message(format!(
                "Heating element {id} to {target_temp:.1}°C (was {:.1}°C) at ({:.1}, {:.1}, {:.1})",
                stats.temperature, stats.position.x, stats.position.y, stats.position.z
            ));
        } else {
            self.add_message(format!("Element {id} not found"));
        }
    }

    /// Ignite elements at a specific position with optional filters
    ///
    /// Command format: `ignite_position <x> <y> [radius] [amount] [filters]`
    fn ignite_position(&mut self, parts: &[&str]) {
        let Some(x) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) else {
            self.add_message(
                "Usage: ignite_position <x> <y> [radius] [amount] [filters]".to_string(),
            );
            return;
        };

        let Some(y) = parts.get(2).and_then(|s| s.parse::<i32>().ok()) else {
            self.add_message(
                "Usage: ignite_position <x> <y> [radius] [amount] [filters]".to_string(),
            );
            return;
        };

        let radius = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let amount = parts
            .get(4)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(-1);

        let (fuel_filter, part_filter, min_z, max_z) = parse_filters(parts, 5);

        let center = Vec3::new(i32_to_f32(x), i32_to_f32(y), 0.0);

        let filtered = filter_elements_in_circle(
            &self.sim,
            center,
            radius,
            fuel_filter.as_deref(),
            part_filter.as_deref(),
            min_z,
            max_z,
        );

        let mut id_dist_ign: Vec<(usize, f32, Celsius, f32)> = filtered
            .into_iter()
            .filter_map(|(id, dist, z)| {
                self.sim
                    .get_element(id)
                    .map(|e| (id, dist, e.fuel().ignition_temperature, z))
            })
            .collect();

        if id_dist_ign.is_empty() {
            self.add_message(format!(
                "No elements found within radius {radius:.1} at ({x}, {y})"
            ));
        } else {
            id_dist_ign.sort_by(|a, b| {
                let z_cmp = a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal);
                if z_cmp == std::cmp::Ordering::Equal {
                    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    z_cmp
                }
            });

            let total = id_dist_ign.len();
            let to_ignite: Vec<(usize, f32, Celsius, f32)> = if amount < 0 {
                id_dist_ign.clone()
            } else {
                let amt = amount as usize;
                id_dist_ign.into_iter().take(amt).collect()
            };

            let ignite_count = if amount < 0 { total } else { to_ignite.len() };
            self.add_message(format!(
                "Found {total} element(s) within radius {radius:.1} — igniting {ignite_count} (ground-up → closest):"
            ));

            for (id, dist, ign_temp, z) in &to_ignite {
                let initial_temp = Celsius::new(600.0).max(*ign_temp);
                self.sim.ignite_element(*id, initial_temp);
                // Invalidate heatmap cache since ignition distribution changed
                self.invalidate_heatmap_cache();
                self.add_message(format!(
                    "  ID {id}: {dist:.2}m, z={z:.2} — ignition temp {ign_temp:.1}°C"
                ));
            }
        }
    }

    /// Heat elements at position
    /// Heat elements at a specific position to a target temperature
    ///
    /// Command format: `heat_position <x> <y> <temp> [radius] [amount] [filters]`
    fn heat_position(&mut self, parts: &[&str]) {
        let Some(x) = parts.get(1).and_then(|s| s.parse::<i32>().ok()) else {
            self.add_message(
                "Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]"
                    .to_string(),
            );
            return;
        };

        let Some(y) = parts.get(2).and_then(|s| s.parse::<i32>().ok()) else {
            self.add_message(
                "Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]"
                    .to_string(),
            );
            return;
        };

        let Some(temperature) = parts.get(3).and_then(|s| s.parse::<f32>().ok()) else {
            self.add_message(
                "Usage: heat_position <x> <y> <temperature> [radius] [amount] [filters]"
                    .to_string(),
            );
            return;
        };

        let radius = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let amount = parts
            .get(5)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(-1);

        let (fuel_filter, part_filter, min_z, max_z) = parse_filters(parts, 6);

        let center = Vec3::new(i32_to_f32(x), i32_to_f32(y), 0.0);

        let mut id_dist_z = filter_elements_in_circle(
            &self.sim,
            center,
            radius,
            fuel_filter.as_deref(),
            part_filter.as_deref(),
            min_z,
            max_z,
        );

        if id_dist_z.is_empty() {
            self.add_message(format!(
                "No elements found within radius {radius:.1} at ({x}, {y})"
            ));
        } else {
            id_dist_z.sort_by(|a, b| {
                let z_cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
                if z_cmp == std::cmp::Ordering::Equal {
                    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    z_cmp
                }
            });

            let total = id_dist_z.len();
            let to_heat: Vec<(usize, f32, f32)> = if amount < 0 {
                id_dist_z.clone()
            } else {
                let amt = amount as usize;
                id_dist_z.into_iter().take(amt).collect()
            };

            let heat_count = if amount < 0 { total } else { to_heat.len() };
            self.add_message(format!(
                "Found {total} element(s) within radius {radius:.1} — heating {heat_count} to {temperature:.1}°C (ground-up → closest):"
            ));

            for (id, dist, z) in &to_heat {
                heat_element_to_temp(&mut self.sim, *id, temperature);
                // Invalidate heatmap cache since heating changes temperatures
                self.invalidate_heatmap_cache();
                self.add_message(format!("  ID {id}: {dist:.2}m, z={z:.2}"));
            }
        }
    }

    /// Set the weather preset by name
    fn set_preset(&mut self, name: &str) {
        let preset = match name.to_lowercase().as_str() {
            "perth" | "perth_metro" => WeatherPreset::perth_metro(),
            "catastrophic" | "cat" => WeatherPreset::catastrophic(),
            "goldfields" => WeatherPreset::goldfields(),
            "wheatbelt" => WeatherPreset::wheatbelt(),
            "south_west" | "southwest" | "sw" => WeatherPreset::south_west(),
            "kimberley" => WeatherPreset::kimberley(),
            "pilbara" => WeatherPreset::pilbara(),
            "hot" => WeatherPreset::basic(
                "Hot",
                Celsius::new(38.0),
                Celsius::new(38.0),
                Percent::new(20.0),
                KilometersPerHour::new(35.0),
                0.15,
            ),
            _ => {
                self.add_message(format!(
                    "Unknown preset: {name}. Available: perth, catastrophic, south_west, wheatbelt, goldfields, kimberley, pilbara, hot"
                ));
                return;
            }
        };

        self.current_weather = preset.clone();
        self.sim.update_weather_preset(preset);
        // Weather change affects conditions; invalidate heatmap cache
        self.invalidate_heatmap_cache();
        let weather_name = &self.current_weather.name;
        self.add_message(format!("Weather preset changed to '{weather_name}'"));
    }

    /// Set time of day in hours (0-24)
    fn set_time(&mut self, hours: f32) {
        if !(0.0..=24.0).contains(&hours) {
            self.add_message("Time must be between 0 and 24 hours".to_string());
            return;
        }

        use fire_sim_core::core_types::Hours;
        let weather = self.sim.get_weather_mut();
        weather.set_time_of_day(Hours::new(hours));
        // Invalidate heatmap cache since time affects weather and potentially conditions
        self.invalidate_heatmap_cache();

        let time_hours = f32_to_u32(hours.floor());
        let time_minutes = f32_to_u32(((hours - u32_to_f32(time_hours)) * 60.0).round());
        self.add_message(format!(
            "Time of day set to {time_hours:02}:{time_minutes:02}"
        ));
    }

    /// Set day of year (1-365)
    fn set_day(&mut self, day: u16) {
        if !(1..=365).contains(&day) {
            self.add_message("Day must be between 1 and 365".to_string());
            return;
        }

        let weather = self.sim.get_weather_mut();
        weather.set_day_of_year(day);
        // Invalidate heatmap cache since day affects weather and potentially conditions
        self.invalidate_heatmap_cache();

        let (month, day_of_month) = day_of_year_to_month_day(day);
        self.add_message(format!("Day of year set to {day} ({month} {day_of_month})"));
    }

    /// Reset simulation
    /// Reset the simulation with new terrain dimensions
    fn reset_simulation(&mut self, width: f32, height: f32) {
        self.sim = create_test_simulation(width, height, self.current_weather.clone());
        self.terrain_width = width;
        self.terrain_height = height;
        self.step_count = 0;
        self.elapsed_time = 0.0;
        self.ignition_times.clear(); // Clear ignition tracking from previous simulation

        // Reset any cached visualizations
        self.invalidate_heatmap_cache();

        self.add_message(format!(
            "Simulation reset! Created {} elements on {}x{} terrain",
            self.sim.get_all_elements().len(),
            width,
            height
        ));
    }

    /// Show help text (for headless mode)
    fn show_help_text(&mut self) {
        self.add_message("═══════════════ AVAILABLE COMMANDS ═══════════════".to_string());
        self.add_message(String::new());

        let mut current_category = "";
        for cmd in COMMANDS {
            // Add category header when it changes
            if cmd.category != current_category {
                if !current_category.is_empty() {
                    self.add_message(String::new());
                }
                self.add_message(format!("{}:", cmd.category));
                current_category = cmd.category;
            }

            // Format the command line - put usage with name, alias separate
            let usage_part = if cmd.usage.is_empty() {
                String::new()
            } else {
                format!(" {}", cmd.usage)
            };

            let alias_part = if cmd.alias.is_empty() {
                String::new()
            } else {
                format!(", {}", cmd.alias)
            };

            let cmd_text = format!("  {}{}{}", cmd.name, usage_part, alias_part);
            let padding = 32_usize.saturating_sub(cmd_text.len());
            let spaces = " ".repeat(padding.max(1));

            self.add_message(format!("{}{}- {}", cmd_text, spaces, cmd.description));
        }

        self.add_message(String::new());
        self.add_message("Filters (for position commands):".to_string());
        self.add_message(
            "  fuel=<name>              - Filter by fuel type (e.g., fuel=eucalyptus)".to_string(),
        );
        self.add_message(
            "  part=<type>              - Filter by fuel part (e.g., part=crown)".to_string(),
        );
        self.add_message(
            "  minz=<height>            - Minimum height in meters (e.g., minz=0)".to_string(),
        );
        self.add_message(
            "  maxz=<height>            - Maximum height in meters (e.g., maxz=20)".to_string(),
        );
        self.add_message("  Example: ip 100 100 50 5 fuel=eucalyptus minz=0 maxz=10".to_string());
    }

    /// Show status as text (for headless mode)
    fn show_status_text(&mut self) {
        let burning: Vec<_> = self
            .sim
            .get_all_elements()
            .iter()
            .map(|e| e.get_stats())
            .collect();

        self.add_message("═══════════════ SIMULATION STATUS ═══════════════".to_string());
        self.add_message(format!(
            "Total elements:    {}",
            self.sim.get_all_elements().len()
        ));
        self.add_message(format!(
            "Burning elements:  {}",
            self.sim.get_burning_elements().len()
        ));
        self.add_message(format!("Active embers:     {}", self.sim.ember_count()));

        if !burning.is_empty() {
            let min_temp = burning
                .iter()
                .map(|e| e.temperature)
                .fold(f32::MAX, f32::min);
            let max_temp = burning
                .iter()
                .map(|e| e.temperature)
                .fold(f32::MIN, f32::max);
            let avg_temp: f32 =
                burning.iter().map(|e| e.temperature).sum::<f32>() / usize_to_f32(burning.len());

            self.add_message(String::new());
            self.add_message("Element temperatures:".to_string());
            self.add_message(format!("  Min: {min_temp:.1}°C"));
            self.add_message(format!("  Max: {max_temp:.1}°C"));
            self.add_message(format!("  Avg: {avg_temp:.1}°C"));
        }
    }

    /// Show weather conditions as text (for headless mode)
    fn show_weather_text(&mut self) {
        let w = self.sim.get_weather().get_stats();
        let (month, day) = day_of_year_to_month_day(w.day_of_year);
        let time_hours = f32_to_u32(*w.time_of_day);
        let time_minutes = f32_to_u32((*w.time_of_day - u32_to_f32(time_hours)) * 60.0);

        self.add_message("═══════════════ WEATHER CONDITIONS ═══════════════".to_string());
        self.add_message(format!(
            "Date & Time:     {month} {day} {time_hours:02}:{time_minutes:02}"
        ));
        self.add_message(format!("Temperature:     {:.1}", w.temperature));
        self.add_message(format!("Humidity:        {:.1}", w.humidity));
        self.add_message(format!(
            "Wind Speed:      {:.1} ({:.1})",
            w.wind_speed,
            w.wind_speed.to_mps()
        ));
        self.add_message(format!("Wind Direction:  {:.0}", w.wind_direction));
        self.add_message(format!("Drought Factor:  {:.1}", w.drought_factor));
        self.add_message(String::new());
        self.add_message(format!("FFDI:            {:.1}", w.ffdi));
        self.add_message(format!("Fire Danger:     {}", w.fire_danger_rating));
        self.add_message(format!("Spread Mult:     {:.2}x", w.spread_rate_multiplier));
    }

    /// Show heatmap as text (for headless mode)
    ///
    /// Generates an ASCII representation of the temperature heatmap
    fn show_heatmap_text(&mut self, grid_size: usize) {
        self.ensure_heatmap_cache(grid_size);

        // Extract data from cache to avoid borrow checker issues
        // Cache is guaranteed to exist after ensure_heatmap_cache
        let cache = self
            .heatmap_cache
            .as_ref()
            .expect("cache should exist after ensure_heatmap_cache");
        let cell_height = self.terrain_height / usize_to_f32(grid_size);
        let (min_temp, max_temp, cells) = (cache.min_temp, cache.max_temp, cache.cells.clone());

        let ambient_temp = *self.sim.get_weather().temperature() as f32;
        const MIN_TEMP_WARM: f32 = 100.0;
        const MIN_TEMP_HOT: f32 = 200.0;
        const MIN_TEMP_VERY_HOT: f32 = 350.0;

        let temp_range = max_temp - min_temp;
        let threshold_very_hot = (min_temp + temp_range * 0.75).max(MIN_TEMP_VERY_HOT);
        let threshold_hot = (min_temp + temp_range * 0.50).max(MIN_TEMP_HOT);
        let threshold_warm = (min_temp + temp_range * 0.25).max(MIN_TEMP_WARM);
        let threshold_cool = ambient_temp.max(50.0); // Use ambient if it's hotter than 50°C

        self.add_message("═══════════════ TEMPERATURE HEATMAP ═══════════════".to_string());
        self.add_message(String::new());
        self.add_message(format!("Legend: · = empty/below {threshold_cool:.0}°C"));
        self.add_message(format!(
            "        ░ >{threshold_cool:.0}°C  ▒ >{threshold_warm:.0}°C  ▓ >{threshold_hot:.0}°C  █ >{threshold_very_hot:.0}°C"
        ));
        self.add_message(format!(
            "Temperature range: {min_temp:.0}°C - {max_temp:.0}°C"
        ));
        self.add_message(String::new());

        for y in (0..grid_size).rev() {
            let y_label = (usize_to_f32(y) * cell_height) as i32;
            let mut line = format!("{y_label:>5} │ "); // Right-align with 5 chars for readability
            for cell in &cells[y] {
                if cell.element_count == 0 {
                    line.push_str("· ");
                } else {
                    let c = if cell.temperature >= threshold_very_hot {
                        '█'
                    } else if cell.temperature >= threshold_hot {
                        '▓'
                    } else if cell.temperature >= threshold_warm {
                        '▒'
                    } else if cell.temperature >= threshold_cool {
                        '░'
                    } else {
                        '·'
                    };
                    line.push(c);
                    line.push(' ');
                }
            }
            self.add_message(line);
        }

        let hot_cells: usize = cells
            .iter()
            .flatten()
            .filter(|c| c.element_count > 0 && c.temperature >= threshold_cool)
            .count();
        if hot_cells > 0 {
            let total_cells = grid_size * grid_size;
            self.add_message(String::new());
            self.add_message(format!(
                "Cells above {threshold_cool:.0}°C: {hot_cells} / {total_cells}"
            ));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber (respects RUST_LOG / default env filter)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    // Check for headless mode flag
    let headless = std::env::args().any(|arg| arg == "--headless");

    if headless {
        run_headless()
    } else {
        run_interactive()
    }
}

/// Run in headless mode (no UI, just command processing and log output)
fn run_headless() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      Bushfire Simulation - Headless Mode                   ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Read terrain dimensions from stdin
    let (width, height) = prompt_terrain_dimensions();

    // Create app in headless mode
    let mut app = App::new_with_mode(width, height, true);

    println!(
        "Created simulation with {} elements on {width}x{height} terrain with {}",
        app.sim.get_all_elements().len(),
        app.sim.is_using_gpu().then(|| "GPU").unwrap_or("no GPU")
    );
    println!("Enter commands (type 'help' for available commands, 'quit' to exit):");
    println!();

    // Process commands from stdin
    let stdin = io::stdin();
    for line in stdin.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Execute command
        app.execute_command(line);

        // Print all new messages
        for msg in &app.messages {
            if !msg.is_empty() {
                println!("{msg}");
            }
        }
        // Clear messages after printing
        app.messages.clear();

        // Process any pending steps
        while app.steps_remaining > 0 {
            app.process_one_step();
            // Print step messages
            for msg in &app.messages {
                if !msg.is_empty() {
                    println!("{msg}");
                }
            }
            app.messages.clear();
        }

        if app.should_quit {
            break;
        }
    }

    println!();
    println!("Goodbye!");
    Ok(())
}

/// Run in interactive mode with TUI
fn run_interactive() -> Result<(), Box<dyn std::error::Error>> {
    // Prompt for terrain dimensions before entering TUI mode
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      Bushfire Simulation - Interactive Debugger            ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    let (width, height) = prompt_terrain_dimensions();

    // Setup terminal
    let mut terminal = ratatui::init();

    // Create app
    let mut app = App::new(width, height);

    // Run app
    let res = run_app(&mut terminal, &mut app);

    ratatui::restore();

    res?;

    println!("Goodbye!");
    Ok(())
}

/// Run the application event loop
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if app.should_quit {
            break;
        }

        // Process one simulation step if stepping is in progress
        if app.steps_remaining > 0 {
            app.process_one_step();
        }

        // Poll for events - use shorter timeout during stepping for responsiveness
        let timeout = if app.steps_remaining > 0 {
            std::time::Duration::from_millis(10)
        } else {
            std::time::Duration::from_millis(100)
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+C stops stepping if in progress, otherwise quits
                        if app.steps_remaining > 0 {
                            app.steps_remaining = 0;
                            app.steps_total = 0;
                            app.is_stepping = false;
                            app.add_message("Stepping interrupted by user.".to_string());
                        } else {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Char('t' | 'T') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Toggle burning list sort mode
                        app.burning_sort_mode = app.burning_sort_mode.next_mode();
                        let mode_name = match app.burning_sort_mode {
                            BurningSortMode::TemperatureAsc => "Temperature (ascending)",
                            BurningSortMode::TemperatureDesc => "Temperature (descending)",
                            BurningSortMode::TimeSinceIgnitionAsc => {
                                "Time Since Ignition (ascending)"
                            }
                            BurningSortMode::TimeSinceIgnitionDesc => {
                                "Time Since Ignition (descending)"
                            }
                        };
                        app.add_message(format!("Burning list sort mode: {mode_name}"));
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Enter => {
                        let command = app.input.clone();
                        app.input.clear();
                        app.execute_command(&command);
                    }
                    KeyCode::Up => {
                        if !app.history.is_empty() && app.history_pos > 0 {
                            app.history_pos -= 1;
                            app.input = app.history[app.history_pos].clone();
                        }
                    }
                    KeyCode::Down => {
                        if !app.history.is_empty() && app.history_pos < app.history.len() - 1 {
                            app.history_pos += 1;
                            app.input = app.history[app.history_pos].clone();
                        } else if app.history_pos == app.history.len() - 1 {
                            app.history_pos = app.history.len();
                            app.input.clear();
                        }
                    }
                    KeyCode::PageUp => {
                        if app.message_scroll < app.messages.len().saturating_sub(1) {
                            app.message_scroll = app.message_scroll.saturating_add(10);
                        }
                    }
                    KeyCode::PageDown => {
                        app.message_scroll = app.message_scroll.saturating_sub(10);
                    }
                    KeyCode::Home => {
                        app.message_scroll = app.messages.len().saturating_sub(1);
                    }
                    KeyCode::End => {
                        app.message_scroll = 0;
                    }
                    KeyCode::Esc => {
                        app.view_mode = ViewMode::Dashboard;
                        app.message_scroll = 0; // Reset scroll when returning to dashboard
                    }
                    KeyCode::F(1) => {
                        app.view_mode = ViewMode::Help;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Draw the UI
fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main area
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Header
    draw_header(f, app, chunks[0]);

    // Main area based on view mode
    match app.view_mode {
        ViewMode::Dashboard => draw_dashboard(f, app, chunks[1]),
        ViewMode::Status => draw_status(f, app, chunks[1]),
        ViewMode::Weather => draw_weather(f, app, chunks[1]),
        ViewMode::Help => draw_help(f, chunks[1]),
        ViewMode::Heatmap => draw_heatmap(f, app, chunks[1]),
    }

    // Input area
    draw_input(f, app, chunks[2]);
}

/// Draw the header
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mut header_text = format!(
        " Fire Simulation | GPU: {} | Step: {} | Time: {:.1}s | Elements: {} | Burning: {} | Embers: {} ",
        app.sim.is_using_gpu().then(|| "ON").unwrap_or("OFF"),
        app.step_count,
        app.elapsed_time,
        app.sim.get_all_elements().len(),
        app.sim.get_burning_elements().len(),
        app.sim.ember_count()
    );

    // Add stepping progress indicator
    if app.steps_remaining > 0 {
        let progress = app.steps_total - app.steps_remaining + 1;
        use std::fmt::Write;
        let _ = write!(
            header_text,
            " | Stepping: {}/{} ",
            progress, app.steps_total
        );
    }

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

/// Draw the dashboard view
fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Messages
            Constraint::Percentage(30), // Burning elements
        ])
        .split(area);

    // Messages
    draw_messages(f, app, chunks[0]);

    // Burning elements
    draw_burning_list(f, app, chunks[1]);
}

/// Draw messages
fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_messages = app.messages.len();

    // Calculate which messages to show based on scroll offset
    let start_idx = if app.message_scroll >= total_messages {
        0
    } else {
        total_messages.saturating_sub(app.message_scroll + visible_height)
    };
    let end_idx = total_messages.saturating_sub(app.message_scroll);

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .skip(start_idx)
        .take(end_idx.saturating_sub(start_idx))
        .map(|m| {
            let style = if m.contains("Error") || m.contains("not found") {
                Style::default().fg(Color::Red)
            } else if m.contains("Ignited") || m.contains("Step") {
                Style::default().fg(Color::Green)
            } else if m.contains("═══") {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(m.as_str()).style(style)
        })
        .collect();

    let scroll_indicator = if app.message_scroll > 0 {
        format!(" Messages (↑{}) ", app.message_scroll)
    } else {
        " Messages ".to_string()
    };

    let messages_list = List::new(messages).block(
        Block::default()
            .borders(Borders::ALL)
            .title(scroll_indicator)
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(messages_list, area);
}

/// Draw burning elements list
fn draw_burning_list(f: &mut Frame, app: &App, area: Rect) {
    let burning_elements = app.sim.get_burning_elements();

    // Extract stats once before sorting to improve performance
    let mut elements_with_stats: Vec<_> = burning_elements
        .iter()
        .map(|e| {
            let stats = e.get_stats();
            let ignition_time = app.ignition_times.get(&stats.id).copied();
            (e, stats, ignition_time)
        })
        .collect();

    // Sort based on current sort mode
    match app.burning_sort_mode {
        BurningSortMode::TemperatureAsc => {
            // Sort by temperature ascending (coolest first)
            elements_with_stats.sort_by(|(_, stats_a, _), (_, stats_b, _)| {
                stats_a
                    .temperature
                    .partial_cmp(&stats_b.temperature)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        BurningSortMode::TemperatureDesc => {
            // Sort by temperature descending (hottest first)
            elements_with_stats.sort_by(|(_, stats_a, _), (_, stats_b, _)| {
                stats_b
                    .temperature
                    .partial_cmp(&stats_a.temperature)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        BurningSortMode::TimeSinceIgnitionAsc => {
            // Sort by time since ignition ascending (oldest fires first)
            elements_with_stats.sort_by(|(_, _, time_a), (_, _, time_b)| {
                let time_a = time_a.unwrap_or(u32::MAX);
                let time_b = time_b.unwrap_or(u32::MAX);
                time_a.cmp(&time_b)
            });
        }
        BurningSortMode::TimeSinceIgnitionDesc => {
            // Sort by time since ignition descending (newest fires first)
            elements_with_stats.sort_by(|(_, _, time_a), (_, _, time_b)| {
                let time_a = time_a.unwrap_or(u32::MAX);
                let time_b = time_b.unwrap_or(u32::MAX);
                time_b.cmp(&time_a)
            });
        }
    }

    let items: Vec<ListItem> = elements_with_stats
        .iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(_, stats, ignition_time)| {
            let temp_color = if stats.temperature > 800.0 {
                Color::Red
            } else if stats.temperature > 400.0 {
                Color::Yellow
            } else {
                Color::White
            };

            let time_info = if let Some(ignition_step) = ignition_time {
                let steps_burning = app.step_count.saturating_sub(*ignition_step);
                format!(" | {steps_burning}s")
            } else {
                String::new()
            };

            let text = format!(
                "ID {:>4} | {:.0}°C{} | ({:.0}, {:.0}, {:.0})",
                stats.id,
                stats.temperature,
                time_info,
                stats.position.x,
                stats.position.y,
                stats.position.z
            );

            ListItem::new(text).style(Style::default().fg(temp_color))
        })
        .collect();

    let sort_indicator = match app.burning_sort_mode {
        BurningSortMode::TemperatureAsc => "↑Temp",
        BurningSortMode::TemperatureDesc => "↓Temp",
        BurningSortMode::TimeSinceIgnitionAsc => "↑Time",
        BurningSortMode::TimeSinceIgnitionDesc => "↓Time",
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " 🔥 Burning ({}) [{}] ",
                elements_with_stats.len(),
                sort_indicator
            ))
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(list, area);
}

/// Draw the status view
fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let burning: Vec<_> = app
        .sim
        .get_all_elements()
        .iter()
        .map(|e| e.get_stats())
        .collect();

    let mut text = vec![
        Line::from(Span::styled(
            "═══════════════ SIMULATION STATUS ═══════════════",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Total elements:    {}",
            app.sim.get_all_elements().len()
        )),
        Line::from(format!(
            "Burning elements:  {}",
            app.sim.get_burning_elements().len()
        )),
        Line::from(format!("Active embers:     {}", app.sim.ember_count())),
        Line::from(""),
    ];

    if !burning.is_empty() {
        let min_temp = burning
            .iter()
            .map(|e| e.temperature)
            .fold(f32::MAX, f32::min);
        let max_temp = burning
            .iter()
            .map(|e| e.temperature)
            .fold(f32::MIN, f32::max);
        let avg_temp: f32 =
            burning.iter().map(|e| e.temperature).sum::<f32>() / usize_to_f32(burning.len());

        text.push(Line::from("Element temperatures:"));
        text.push(Line::from(format!("  Min: {min_temp:.1}°C")));
        text.push(Line::from(format!("  Max: {max_temp:.1}°C")));
        text.push(Line::from(format!("  Avg: {avg_temp:.1}°C")));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Press ESC to return to dashboard",
        Style::default().fg(Color::Yellow),
    )));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Status View ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the weather view
fn draw_weather(f: &mut Frame, app: &App, area: Rect) {
    let w = app.sim.get_weather().get_stats();

    let (month, day) = day_of_year_to_month_day(w.day_of_year);
    let time_hours = f32_to_u32(*w.time_of_day);
    let time_minutes = f32_to_u32((*w.time_of_day - u32_to_f32(time_hours)) * 60.0);

    let text = vec![
        Line::from(Span::styled(
            "═══════════════ WEATHER CONDITIONS ═══════════════",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Date & Time:     {month} {day} {time_hours:02}:{time_minutes:02}"
        )),
        Line::from(format!("Temperature:     {:.1}", w.temperature)),
        Line::from(format!("Humidity:        {:.1}", w.humidity)),
        Line::from(format!(
            "Wind Speed:      {:.1} ({:.1})",
            w.wind_speed,
            w.wind_speed.to_mps()
        )),
        Line::from(format!("Wind Direction:  {:.0}", w.wind_direction)),
        Line::from(format!("Drought Factor:  {:.1}", w.drought_factor)),
        Line::from(""),
        Line::from(Span::styled(
            format!("FFDI:            {:.1}", w.ffdi),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Fire Danger:     {}", w.fire_danger_rating)),
        Line::from(format!("Spread Mult:     {:.2}x", w.spread_rate_multiplier)),
        Line::from(""),
        Line::from(Span::styled(
            "Press ESC to return to dashboard",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Weather View ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the help view
fn draw_help(f: &mut Frame, area: Rect) {
    let mut text = vec![
        Line::from(Span::styled(
            "═══════════════ AVAILABLE COMMANDS ═══════════════",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let mut current_category = "";
    for cmd in COMMANDS {
        // Add category header when it changes
        if cmd.category != current_category {
            if !current_category.is_empty() {
                text.push(Line::from(""));
            }
            text.push(Line::from(Span::styled(
                format!("{}:", cmd.category),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            current_category = cmd.category;
        }

        // Format the command line - put usage with name, alias separate
        let usage_part = if cmd.usage.is_empty() {
            String::new()
        } else {
            format!(" {}", cmd.usage)
        };

        let alias_part = if cmd.alias.is_empty() {
            String::new()
        } else {
            format!(", {}", cmd.alias)
        };

        let cmd_text = format!("  {}{}{}", cmd.name, usage_part, alias_part);
        let padding = 32_usize.saturating_sub(cmd_text.len());
        let spaces = " ".repeat(padding.max(1));

        text.push(Line::from(format!(
            "{}{}- {}",
            cmd_text, spaces, cmd.description
        )));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Filters (for position commands):",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(
        "  fuel=<name>              - Filter by fuel type (e.g., fuel=eucalyptus)",
    ));
    text.push(Line::from(
        "  part=<type>              - Filter by fuel part (e.g., part=crown, part=root)",
    ));
    text.push(Line::from(
        "  minz=<height>            - Minimum height in meters (e.g., minz=5.0)",
    ));
    text.push(Line::from(
        "  maxz=<height>            - Maximum height in meters (e.g., maxz=20.0)",
    ));
    text.push(Line::from(
        "  Examples: ip 100 100 50 5 fuel=eucalyptus minz=0 maxz=10",
    ));
    text.push(Line::from("            hp 100 100 600 50 part=crown"));
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Controls:",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(
        "  Ctrl+C                   - Stop stepping (if active) or quit simulation",
    ));
    text.push(Line::from(
        "  Up/Down arrows           - Navigate command history",
    ));
    text.push(Line::from(
        "  Ctrl+T                   - Toggle burning list sort (Temperature/Time)",
    ));
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "During Stepping:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(
        "  While stepping is in progress, you can still use:",
    ));
    text.push(Line::from(
        "  - View commands (status, weather, dashboard, help)",
    ));
    text.push(Line::from(
        "  - Query commands (element, burning, embers, nearby, heatmap)",
    ));
    text.push(Line::from("  - Ctrl+C to interrupt stepping at any time"));
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "Press ESC to return to dashboard",
        Style::default().fg(Color::Yellow),
    )));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the heatmap view
/// Render the heatmap view (enhanced with smooth RGB gradients)
fn draw_heatmap(f: &mut Frame, app: &App, area: Rect) {
    let grid_size = app.heatmap_size;

    // Cache should always be populated by ensure_heatmap_cache before viewing
    let Some(cache) = &app.heatmap_cache else {
        let text = vec![
            Line::from(Span::styled(
                "═══════════════ TEMPERATURE HEATMAP ═══════════════",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "No heatmap data. Step the simulation first.",
                Style::default().fg(Color::Yellow),
            )),
        ];
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Heatmap View ")
                    .style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    };

    if cache.step_count == app.step_count && cache.size == grid_size {
        // Helper: convert temperature to smooth RGB gradient color
        let temp_to_color = |temp: f32, min: f32, max: f32| -> Color {
            if max <= min {
                return Color::DarkGray;
            }
            // Normalize temperature to 0.0-1.0 range
            let normalized = ((temp - min) / (max - min)).clamp(0.0, 1.0);

            // Multi-stop gradient: blue → cyan → green → yellow → orange → red
            let (red, green, blue) = if normalized < 0.2 {
                // Blue → Cyan (0.0 - 0.2)
                let local_t = normalized / 0.2;
                (0, (100.0 + local_t * 155.0) as u8, 255)
            } else if normalized < 0.4 {
                // Cyan → Green (0.2 - 0.4)
                let local_t = (normalized - 0.2) / 0.2;
                (0, 255, (255.0 * (1.0 - local_t)) as u8)
            } else if normalized < 0.6 {
                // Green → Yellow (0.4 - 0.6)
                let local_t = (normalized - 0.4) / 0.2;
                ((255.0 * local_t) as u8, 255, 0)
            } else if normalized < 0.8 {
                // Yellow → Orange (0.6 - 0.8)
                let local_t = (normalized - 0.6) / 0.2;
                (255, (255.0 * (1.0 - local_t * 0.5)) as u8, 0)
            } else {
                // Orange → Red (0.8 - 1.0)
                let local_t = (normalized - 0.8) / 0.2;
                (255, (128.0 * (1.0 - local_t)) as u8, 0)
            };
            Color::Rgb(red, green, blue)
        };

        let ambient_temp = *app.sim.get_weather().temperature() as f32;
        let threshold_cool = ambient_temp.max(50.0); // Use ambient if it's hotter than 50°C

        // Use threshold_cool as effective minimum for gradient
        let effective_min = threshold_cool.max(cache.min_temp);

        // Build styled lines with background colors for cells
        let mut tui_lines = Vec::new();

        // Header
        tui_lines.push(Line::from(Span::styled(
            "═══════════════ TEMPERATURE HEATMAP ═══════════════",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        tui_lines.push(Line::from(""));

        tui_lines.push(Line::from(vec![
            Span::raw("Temperature ranges (cells ≥"),
            Span::styled(
                format!("{threshold_cool:.0}°C"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("):"),
        ]));

        // Calculate color stops for legend
        let temp_span = cache.max_temp - effective_min;
        let blue_max = effective_min + temp_span * 0.2;
        let cyan_max = effective_min + temp_span * 0.4;
        let green_max = effective_min + temp_span * 0.6;
        let yellow_max = effective_min + temp_span * 0.8;

        tui_lines.push(Line::from(vec![
            Span::styled("█", Style::default().fg(Color::Rgb(0, 100, 255))),
            Span::raw(format!(" Blue {effective_min:.0}-{blue_max:.0}°C   ")),
            Span::styled("█", Style::default().fg(Color::Rgb(0, 255, 255))),
            Span::raw(format!(" Cyan {blue_max:.0}-{cyan_max:.0}°C   ")),
            Span::styled("█", Style::default().fg(Color::Rgb(0, 255, 0))),
            Span::raw(format!(" Green {cyan_max:.0}-{green_max:.0}°C")),
        ]));
        tui_lines.push(Line::from(vec![
            Span::styled("█", Style::default().fg(Color::Rgb(255, 255, 0))),
            Span::raw(format!(" Yellow {green_max:.0}-{yellow_max:.0}°C  ")),
            Span::styled("█", Style::default().fg(Color::Rgb(255, 150, 0))),
            Span::raw(format!(
                " Orange {:.0}-{:.0}°C  ",
                yellow_max,
                cache.max_temp * 0.9
            )),
            Span::styled("█", Style::default().fg(Color::Rgb(255, 0, 0))),
            Span::raw(format!(
                " Red {:.0}-{:.0}°C",
                cache.max_temp * 0.9,
                cache.max_temp
            )),
        ]));
        tui_lines.push(Line::from(""));

        // Grid lines with color-coded cells
        let cell_height = app.terrain_height / usize_to_f32(grid_size);

        // Calculate max label width for proper alignment
        let max_y_label = (f64::from((grid_size - 1) as u32) * f64::from(cell_height)) as i32;
        let _label_width = max_y_label.to_string().len().max(3);

        // Build table rows instead of lines
        let mut table_rows = Vec::new();
        for y in (0..grid_size).rev() {
            let y_label = (usize_to_f32(y) * cell_height) as i32;

            // Build cell content
            let mut cell_spans = Vec::new();
            for x in 0..grid_size {
                let cell = &cache.cells[y][x];

                if cell.element_count == 0 || cell.temperature < threshold_cool {
                    cell_spans.push(Span::styled("· ", Style::default().fg(Color::DarkGray)));
                } else {
                    let bg_color = temp_to_color(cell.temperature, effective_min, cache.max_temp);
                    let temp_normalized =
                        (cell.temperature - effective_min) / (cache.max_temp - effective_min);
                    let symbol = if temp_normalized > 0.75 {
                        "█"
                    } else if temp_normalized > 0.5 {
                        "▓"
                    } else if temp_normalized > 0.25 {
                        "▒"
                    } else {
                        "░"
                    };
                    cell_spans.push(Span::styled(
                        format!("{symbol} "),
                        Style::default().bg(bg_color).fg(bg_color),
                    ));
                }
            }

            table_rows.push(Row::new(vec![
                Line::from(y_label.to_string()),
                Line::from(cell_spans),
            ]));
        }

        // Footer
        let hot_cells: usize = cache
            .cells
            .iter()
            .flatten()
            .filter(|c| c.element_count > 0 && c.temperature >= threshold_cool)
            .count();
        if hot_cells > 0 {
            let total_cells = grid_size * grid_size;
            tui_lines.push(Line::from(""));
            tui_lines.push(Line::from(format!(
                "Cells above {threshold_cool:.0}°C: {hot_cells} / {total_cells}"
            )));
        }
        tui_lines.push(Line::from(""));
        tui_lines.push(Line::from(Span::styled(
            "Press ESC to return to dashboard",
            Style::default().fg(Color::Yellow),
        )));

        // Create table with proper column alignment
        let table =
            Table::new(table_rows, [Constraint::Length(5), Constraint::Fill(1)]).column_spacing(1);

        // Split area: header (legend), table (grid), footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Fill(1),
                Constraint::Length(3),
            ])
            .split(area);

        // Render header in top chunk
        let header_lines: Vec<Line> = tui_lines.drain(0..7).collect();
        let header_paragraph = Paragraph::new(header_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Heatmap View (Enhanced RGB) ")
                .style(Style::default().fg(Color::White)),
        );
        f.render_widget(header_paragraph, chunks[0]);

        // Render table in middle chunk
        f.render_widget(table, chunks[1]);

        // Render footer in bottom chunk
        let footer_paragraph = Paragraph::new(tui_lines);
        f.render_widget(footer_paragraph, chunks[2]);
        return;
    }

    // Cache exists but is stale - shouldn't happen, but handle gracefully
    let text = vec![
        Line::from(Span::styled(
            "═══════════════ TEMPERATURE HEATMAP ═══════════════",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Cache mismatch. Try switching views or stepping.",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Heatmap View ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the input area
fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let input_text = format!("fire> {}", app.input);
    let title = if app.message_scroll > 0 {
        " Command Input (F1 for help | PgUp/PgDn to scroll) "
    } else {
        " Command Input (F1 for help) "
    };
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);
}

// Helper functions from the original implementation

/// Prompt user for terrain dimensions at startup
fn prompt_terrain_dimensions() -> (f32, f32) {
    println!("Enter terrain dimensions (or press Enter for defaults):");

    // Width
    print!("  Width in meters [{DEFAULT_WIDTH}]: ");
    io::stdout().flush().unwrap();
    let mut width_str = String::new();
    io::stdin().read_line(&mut width_str).unwrap();
    let width: f32 = width_str.trim().parse().unwrap_or(DEFAULT_WIDTH);

    // Height
    print!("  Height in meters [{DEFAULT_HEIGHT}]: ");
    io::stdout().flush().unwrap();
    let mut height_str = String::new();
    io::stdin().read_line(&mut height_str).unwrap();
    let height: f32 = height_str.trim().parse().unwrap_or(DEFAULT_HEIGHT);

    // Validate dimensions
    let width = width.clamp(10.0, 1000.0);
    let height = height.clamp(10.0, 1000.0);

    println!();
    (width, height)
}

/// Parse filter tokens from command arguments
fn parse_filters(
    parts: &[&str],
    start_idx: usize,
) -> (Option<String>, Option<String>, Option<f32>, Option<f32>) {
    let mut fuel_filter: Option<String> = None;
    let mut part_filter: Option<String> = None;
    let mut min_z: Option<f32> = None;
    let mut max_z: Option<f32> = None;

    for token in parts.iter().skip(start_idx) {
        if let Some((key, val)) = token.split_once('=') {
            match key.to_lowercase().as_str() {
                "fuel" => fuel_filter = Some(val.to_lowercase()),
                "part" => part_filter = Some(val.to_lowercase()),
                "minz" => min_z = val.parse::<f32>().ok(),
                "maxz" => max_z = val.parse::<f32>().ok(),
                _ => {}
            }
        }
    }

    (fuel_filter, part_filter, min_z, max_z)
}

/// Get part name as a string for filtering
fn get_part_name(part: &fire_sim_core::core_types::element::FuelPart) -> String {
    match part {
        fire_sim_core::core_types::element::FuelPart::Root => "root".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkLower => "trunklower".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkMiddle => "trunkmiddle".to_string(),
        fire_sim_core::core_types::element::FuelPart::TrunkUpper => "trunkupper".to_string(),
        fire_sim_core::core_types::element::FuelPart::BarkLayer(h) => {
            format!("barklayer({h:.0})")
        }
        fire_sim_core::core_types::element::FuelPart::Branch { height, angle: _ } => {
            format!("branch({height:.0})")
        }
        fire_sim_core::core_types::element::FuelPart::Crown => "crown".to_string(),
        fire_sim_core::core_types::element::FuelPart::GroundLitter => "groundlitter".to_string(),
        fire_sim_core::core_types::element::FuelPart::GroundVegetation => {
            "groundvegetation".to_string()
        }
        fire_sim_core::core_types::element::FuelPart::BuildingWall { floor } => {
            format!("buildingwall({floor})")
        }
        fire_sim_core::core_types::element::FuelPart::BuildingRoof => "buildingroof".to_string(),
        fire_sim_core::core_types::element::FuelPart::BuildingInterior => {
            "buildinginterior".to_string()
        }
        fire_sim_core::core_types::element::FuelPart::Vehicle => "vehicle".to_string(),
        fire_sim_core::core_types::element::FuelPart::Surface => "surface".to_string(),
    }
}

/// Filter elements within a 2D circle radius, applying optional fuel/part/z filters
fn filter_elements_in_circle(
    sim: &FireSimulation,
    center: Vec3,
    radius: f32,
    fuel_filter: Option<&str>,
    part_filter: Option<&str>,
    min_z: Option<f32>,
    max_z: Option<f32>,
) -> Vec<(usize, f32, f32)> {
    let candidates = sim.get_elements_in_radius(center, radius);

    candidates
        .into_iter()
        .filter_map(|e| {
            let dx = e.position().x - center.x;
            let dy = e.position().y - center.y;
            let dist2d = (dx * dx + dy * dy).sqrt();

            if dist2d <= radius {
                // Apply fuel filter
                if let Some(f) = fuel_filter {
                    let fuel_name = e.fuel().name.to_lowercase();
                    if !fuel_name.contains(f) {
                        return None;
                    }
                }

                // Apply part filter
                if let Some(p) = part_filter {
                    let part_name = get_part_name(&e.part_type());
                    if !part_name.to_lowercase().contains(p) {
                        return None;
                    }
                }

                // Apply min z filter
                if let Some(minz) = min_z {
                    if e.position().z < minz {
                        return None;
                    }
                }

                // Apply max z filter
                if let Some(maxz) = max_z {
                    if e.position().z > maxz {
                        return None;
                    }
                }

                Some((e.id(), dist2d, e.position().z))
            } else {
                None
            }
        })
        .collect()
}

/// Create a test simulation
fn create_test_simulation(
    width: f32,
    height: f32,
    weather_preset: WeatherPreset,
) -> FireSimulation {
    let mut sim = FireSimulation::new(5.0, &TerrainData::flat(width, height, 5.0, 0.0));

    let step = 1;
    for x in (0..(width as i32)).step_by(step) {
        for y in (0..(height as i32)).step_by(step) {
            let fuel = if (x + y) % 20 == 0 {
                Fuel::dead_wood_litter()
            } else {
                Fuel::dry_grass()
            };

            let id = sim.add_fuel_element(
                Vec3::new(i32_to_f32(x), i32_to_f32(y), 0.0),
                fuel,
                Kilograms::new(3.0),
                FuelPart::GroundVegetation,
            );

            // Add some trees (every 15m)
            if x % 15 == 0 && y % 15 == 0 {
                create_tree(&mut sim, i32_to_f32(x), i32_to_f32(y), id);
            }
        }
    }

    // Set weather conditions
    let weather = WeatherSystem::from_preset(
        weather_preset,
        3,    // January 3
        14.0, // 2pm
        ClimatePattern::Neutral,
    );
    sim.set_weather(weather);

    sim
}

/// Create a tree
fn create_tree(sim: &mut FireSimulation, x: f32, y: f32, _ground_id: usize) {
    // Trunk
    sim.add_fuel_element(
        Vec3::new(x, y, 2.0),
        Fuel::eucalyptus_stringybark(),
        Kilograms::new(10.0),
        FuelPart::TrunkLower,
    );

    // Lower branches
    sim.add_fuel_element(
        Vec3::new(x - 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        Kilograms::new(3.0),
        FuelPart::Branch {
            height: Meters::new(4.0),
            angle: Degrees::new(0.0),
        },
    );
    sim.add_fuel_element(
        Vec3::new(x + 1.0, y, 4.0),
        Fuel::eucalyptus_stringybark(),
        Kilograms::new(3.0),
        FuelPart::Branch {
            height: Meters::new(4.0),
            angle: Degrees::new(180.0),
        },
    );

    // Crown
    sim.add_fuel_element(
        Vec3::new(x, y, 8.0),
        Fuel::eucalyptus_stringybark(),
        Kilograms::new(5.0),
        FuelPart::Crown,
    );
}

/// Helper function to heat an element to a target temperature
fn heat_element_to_temp(sim: &mut FireSimulation, id: usize, target_temp: f32) {
    if let Some(e) = sim.get_element(id) {
        let stats = e.get_stats();
        let current_temp = stats.temperature;
        let fuel_mass = stats.fuel_remaining;
        let specific_heat = e.fuel().specific_heat;

        if target_temp > current_temp {
            // Calculate heat needed: Q = m × c × ΔT
            let temp_rise = target_temp - current_temp;
            let specific_heat_val: f32 = specific_heat.into();
            let heat_kj = fuel_mass * specific_heat_val * temp_rise;

            // Apply heat over 1 second timestep (no pilot flame - external heat source)
            sim.apply_heat_to_element(id, heat_kj, 1.0, false);
        }
    }
}

/// Convert day of year (1-365) to month name and day
fn day_of_year_to_month_day(day_of_year: u16) -> (&'static str, u16) {
    const DAYS_IN_MONTHS: [(u16, &str); 12] = [
        (31, "January"),
        (28, "February"),
        (31, "March"),
        (30, "April"),
        (31, "May"),
        (30, "June"),
        (31, "July"),
        (31, "August"),
        (30, "September"),
        (31, "October"),
        (30, "November"),
        (31, "December"),
    ];

    let mut remaining_days = day_of_year;
    for (days_in_month, month_name) in &DAYS_IN_MONTHS {
        if remaining_days <= *days_in_month {
            return (month_name, remaining_days);
        }
        remaining_days -= days_in_month;
    }

    ("December", 31)
}

// Small helpers for deliberate integer↔float casts
#[inline]
#[expect(clippy::cast_precision_loss)]
fn i32_to_f32(v: i32) -> f32 {
    v as f32
}

#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}

#[inline]
#[expect(clippy::cast_precision_loss)]
fn u32_to_f32(v: u32) -> f32 {
    v as f32
}

#[inline]
#[expect(clippy::cast_possible_truncation)]
fn f32_to_u32(v: f32) -> u32 {
    v as u32
}
