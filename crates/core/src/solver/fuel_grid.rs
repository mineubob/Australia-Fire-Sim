//! Per-cell, per-layer fuel type management for spatially varying fuel properties.
//!
//! This module enables realistic fuel modeling where:
//! - Different cells have different vegetation types (from terrain)
//! - Each cell has vertical layers with distinct fuel types:
//!   - Surface (0-0.5m): grass, litter, dead leaves
//!   - Shrub (0.5-3m): bark, understory shrubs, saplings
//!   - Canopy (3m+): leaves, fine branches
//! - Each layer can contain a **mixture** of up to 4 fuel types with weights
//!
//! # Example: Forest Edge Cell with Mixtures
//! ```text
//! Canopy:  100% Eucalyptus leaves
//! Shrub:   60% Stringybark strips + 40% Shrubland
//! Surface: 40% Grass + 35% Dead Litter + 25% Rock
//! ```
//!
//! Effective properties are computed as weighted averages:
//! - `ignition_temp = Σ(weight_i × ignition_temp_i)`
//! - `heat_content = Σ(weight_i × heat_content_i)`
//!
//! # Scientific References
//!
//! - Scott & Burgan (2005). "Standard Fire Behavior Fuel Models"
//! - Cheney et al. (2012). "Predicting fire behaviour in dry eucalypt forest"

use crate::core_types::units::{
    Celsius, Degrees, Fraction, HeatTransferCoefficient, Hours, KgPerCubicMeter, KjPerKg, KjPerKgK,
    Meters, Percent, RatePerSecond, SurfaceAreaToVolume, ThermalConductivity, ThermalDiffusivity,
};
use crate::core_types::Fuel;

/// Maximum number of fuel types supported in the lookup table.
/// 256 types should cover all vegetation classes.
pub const MAX_FUEL_TYPES: usize = 256;

/// Number of vertical fuel layers per cell.
pub const NUM_FUEL_LAYERS: usize = 3;

/// Fuel type indices for the three vertical layers in a cell.
///
/// Each layer can have a different fuel type, enabling realistic
/// vertical fuel structure modeling.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CellFuelTypes {
    /// Fuel type index for surface layer (0-0.5m)
    /// Typical: grass, litter, dead leaves
    pub surface: u8,

    /// Fuel type index for shrub/ladder layer (0.5-3m)
    /// Typical: bark strips, understory shrubs, saplings
    pub shrub: u8,

    /// Fuel type index for canopy layer (3m+)
    /// Typical: tree leaves, fine branches
    pub canopy: u8,

    /// Padding for alignment (reserved for future use)
    #[doc(hidden)]
    padding: u8,
}

impl CellFuelTypes {
    /// Create with uniform fuel type across all layers.
    #[must_use]
    pub const fn uniform(fuel_type: u8) -> Self {
        Self {
            surface: fuel_type,
            shrub: fuel_type,
            canopy: fuel_type,
            padding: 0,
        }
    }

    /// Create with distinct fuel types per layer.
    #[must_use]
    pub const fn layered(surface: u8, shrub: u8, canopy: u8) -> Self {
        Self {
            surface,
            shrub,
            canopy,
            padding: 0,
        }
    }

    /// Create typical eucalyptus forest configuration.
    /// Surface: dead wood litter, Shrub: stringybark, Canopy: stringybark leaves
    #[must_use]
    pub const fn eucalyptus_forest() -> Self {
        Self {
            surface: FuelTypeId::DEAD_WOOD_LITTER,
            shrub: FuelTypeId::EUCALYPTUS_STRINGYBARK,
            canopy: FuelTypeId::EUCALYPTUS_STRINGYBARK,
            padding: 0,
        }
    }

    /// Create typical grassland configuration.
    /// All layers use grass fuel type.
    #[must_use]
    pub const fn grassland() -> Self {
        Self::uniform(FuelTypeId::GRASS)
    }

    /// Create shrubland configuration.
    /// Surface: grass, Shrub: shrubland, Canopy: none (uses shrubland for continuity)
    #[must_use]
    pub const fn shrubland() -> Self {
        Self {
            surface: FuelTypeId::GRASS,
            shrub: FuelTypeId::SHRUBLAND,
            canopy: FuelTypeId::SHRUBLAND,
            padding: 0,
        }
    }
}

// ============================================================================
// FUEL MIXTURE SYSTEM
// ============================================================================

/// Maximum number of fuel types that can be mixed in a single layer.
pub const MAX_FUELS_PER_LAYER: usize = 4;

/// Sentinel value indicating an unused fuel slot in a mixture.
pub const FUEL_NONE: u8 = 255;

/// Fuel mixture for a single layer (up to 4 fuel types with weights).
///
/// Enables realistic transition zones where multiple vegetation types coexist.
/// Effective properties are computed as weighted averages.
///
/// # Example
/// ```text
/// Surface layer at forest edge:
///   40% Grass + 35% Dead Litter + 25% Bare Rock
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LayerFuelMixture {
    /// Fuel type indices (use `FUEL_NONE` = 255 for unused slots).
    pub fuel_ids: [u8; MAX_FUELS_PER_LAYER],

    /// Weight fractions for each fuel type. Should sum to 1.0.
    /// Weights for unused slots (`fuel_id` = `FUEL_NONE`) are ignored.
    pub weights: [f32; MAX_FUELS_PER_LAYER],
}

impl Default for LayerFuelMixture {
    fn default() -> Self {
        Self::empty()
    }
}

impl LayerFuelMixture {
    /// Create an empty mixture (no fuel).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            fuel_ids: [FUEL_NONE; MAX_FUELS_PER_LAYER],
            weights: [0.0; MAX_FUELS_PER_LAYER],
        }
    }

    /// Create a mixture with a single fuel type (100% weight).
    #[must_use]
    pub const fn single(fuel_id: u8) -> Self {
        Self {
            fuel_ids: [fuel_id, FUEL_NONE, FUEL_NONE, FUEL_NONE],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// Create a mixture with two fuel types.
    #[must_use]
    pub const fn blend2(fuel_a: u8, weight_a: f32, fuel_b: u8, weight_b: f32) -> Self {
        Self {
            fuel_ids: [fuel_a, fuel_b, FUEL_NONE, FUEL_NONE],
            weights: [weight_a, weight_b, 0.0, 0.0],
        }
    }

    /// Create a mixture with three fuel types.
    #[must_use]
    pub const fn blend3(
        fuel_a: u8,
        weight_a: f32,
        fuel_b: u8,
        weight_b: f32,
        fuel_c: u8,
        weight_c: f32,
    ) -> Self {
        Self {
            fuel_ids: [fuel_a, fuel_b, fuel_c, FUEL_NONE],
            weights: [weight_a, weight_b, weight_c, 0.0],
        }
    }

    /// Create a mixture with four fuel types.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "Natural API for 4 fuel/weight pairs"
    )]
    pub const fn blend4(
        fuel_a: u8,
        weight_a: f32,
        fuel_b: u8,
        weight_b: f32,
        fuel_c: u8,
        weight_c: f32,
        fuel_d: u8,
        weight_d: f32,
    ) -> Self {
        Self {
            fuel_ids: [fuel_a, fuel_b, fuel_c, fuel_d],
            weights: [weight_a, weight_b, weight_c, weight_d],
        }
    }

    /// Check if this mixture contains any fuel.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fuel_ids[0] == FUEL_NONE
    }

    /// Get the number of fuels in this mixture (1-4).
    #[must_use]
    pub fn fuel_count(&self) -> usize {
        self.fuel_ids
            .iter()
            .take_while(|&&id| id != FUEL_NONE)
            .count()
    }

    /// Normalize weights to sum to 1.0.
    pub fn normalize(&mut self) {
        let sum: f32 = self
            .fuel_ids
            .iter()
            .zip(self.weights.iter())
            .filter(|(&id, _)| id != FUEL_NONE)
            .map(|(_, &w)| w)
            .sum();

        if sum > 0.0 {
            for (id, weight) in self.fuel_ids.iter().zip(self.weights.iter_mut()) {
                if *id != FUEL_NONE {
                    *weight /= sum;
                }
            }
        }
    }

    /// Iterate over (`fuel_id`, weight) pairs for active fuels.
    pub fn iter(&self) -> impl Iterator<Item = (u8, f32)> + '_ {
        self.fuel_ids
            .iter()
            .zip(self.weights.iter())
            .take_while(|(&id, _)| id != FUEL_NONE)
            .map(|(&id, &w)| (id, w))
    }
}

/// Per-cell fuel mixture configuration for all three vertical layers.
///
/// Enables realistic fuel modeling with mixed vegetation at each height.
#[derive(Debug, Clone, Copy, Default)]
pub struct CellFuelMixtures {
    /// Surface layer mixture (0-0.5m)
    pub surface: LayerFuelMixture,

    /// Shrub/ladder layer mixture (0.5-3m)
    pub shrub: LayerFuelMixture,

    /// Canopy layer mixture (3m+)
    pub canopy: LayerFuelMixture,
}

impl CellFuelMixtures {
    /// Create from simple `CellFuelTypes` (single fuel per layer).
    #[must_use]
    pub const fn from_simple(types: CellFuelTypes) -> Self {
        Self {
            surface: LayerFuelMixture::single(types.surface),
            shrub: LayerFuelMixture::single(types.shrub),
            canopy: LayerFuelMixture::single(types.canopy),
        }
    }

    /// Create typical eucalyptus forest with mixed surface litter.
    #[must_use]
    pub const fn eucalyptus_forest_mixed() -> Self {
        Self {
            // Surface: 50% dead litter + 30% grass + 20% bark debris
            surface: LayerFuelMixture::blend3(
                FuelTypeId::DEAD_WOOD_LITTER,
                0.5,
                FuelTypeId::GRASS,
                0.3,
                FuelTypeId::EUCALYPTUS_STRINGYBARK,
                0.2,
            ),
            // Shrub: 70% stringybark + 30% shrubland
            shrub: LayerFuelMixture::blend2(
                FuelTypeId::EUCALYPTUS_STRINGYBARK,
                0.7,
                FuelTypeId::SHRUBLAND,
                0.3,
            ),
            // Canopy: 100% stringybark
            canopy: LayerFuelMixture::single(FuelTypeId::EUCALYPTUS_STRINGYBARK),
        }
    }

    /// Create forest edge transition zone.
    #[must_use]
    pub const fn forest_edge() -> Self {
        Self {
            // Surface: grass-dominated with some litter
            surface: LayerFuelMixture::blend2(
                FuelTypeId::GRASS,
                0.6,
                FuelTypeId::DEAD_WOOD_LITTER,
                0.4,
            ),
            // Shrub: mixed shrubland and eucalyptus
            shrub: LayerFuelMixture::blend2(
                FuelTypeId::SHRUBLAND,
                0.5,
                FuelTypeId::EUCALYPTUS_STRINGYBARK,
                0.5,
            ),
            // Canopy: scattered eucalyptus
            canopy: LayerFuelMixture::blend2(
                FuelTypeId::EUCALYPTUS_STRINGYBARK,
                0.4,
                FuelTypeId::GREEN_VEGETATION,
                0.6,
            ),
        }
    }
}

/// Blend multiple Fuel types into a single effective Fuel using weighted averaging.
///
/// This is the core function for computing effective fire behavior properties
/// from fuel mixtures. All numeric properties are weighted averages; categorical
/// properties (name, bark type) use the dominant fuel.
///
/// # Arguments
/// * `fuels` - Slice of (Fuel reference, weight) pairs
///
/// # Returns
/// A new Fuel struct with blended properties.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "Fuel has many fields that must be blended"
)]
pub fn blend_fuels(fuels: &[(&Fuel, f32)]) -> Fuel {
    if fuels.is_empty() {
        return Fuel::rock(); // Fallback for empty mixture
    }

    if fuels.len() == 1 {
        return fuels[0].0.clone(); // Single fuel, no blending needed
    }

    // Normalize weights
    let total_weight: f32 = fuels.iter().map(|(_, w)| w).sum();
    let norm_fuels: Vec<(&Fuel, f32)> = if total_weight > 0.0 {
        fuels.iter().map(|(f, w)| (*f, w / total_weight)).collect()
    } else {
        return Fuel::rock();
    };

    // Find dominant fuel (highest weight) for categorical properties
    let dominant = norm_fuels
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(fuels[0].0, |(f, _)| *f);

    // Helper macro for weighted averaging of f32 values
    macro_rules! blend_f32 {
        ($field:ident) => {{
            norm_fuels.iter().map(|(f, w)| f.$field * w).sum::<f32>()
        }};
    }

    // Helper macro for weighted averaging of wrapped numeric types
    macro_rules! blend_wrapped {
        ($field:ident, $wrapper:ty, $method:ident) => {{
            let val: f32 = norm_fuels.iter().map(|(f, w)| *f.$field * w).sum();
            <$wrapper>::$method(val)
        }};
    }

    // Helper for Celsius (needs f64 - use deref twice: &Celsius -> Celsius -> f64)
    let blend_celsius = |field_fn: fn(&Fuel) -> &Celsius| -> Celsius {
        let val: f64 = norm_fuels
            .iter()
            .map(|(f, w)| {
                let celsius_val: f64 = **field_fn(f); // Deref &Celsius -> Celsius -> f64
                celsius_val * f64::from(*w)
            })
            .sum();
        Celsius::new(val)
    };

    // Helper for array of Fractions
    let blend_size_class = || -> [Fraction; 4] {
        let mut result = [0.0_f32; 4];
        for (fuel, weight) in &norm_fuels {
            for (i, frac) in fuel.size_class_distribution.iter().enumerate() {
                result[i] += **frac * weight;
            }
        }
        [
            Fraction::new(result[0]),
            Fraction::new(result[1]),
            Fraction::new(result[2]),
            Fraction::new(result[3]),
        ]
    };

    Fuel {
        // Use dominant fuel for categorical properties
        id: dominant.id,
        name: format!("Blended ({})", dominant.name),
        bark_properties: dominant.bark_properties,
        canopy_structure: dominant.canopy_structure.clone(),

        // Blend all numeric properties
        heat_content: blend_wrapped!(heat_content, KjPerKg, new),
        ignition_temperature: blend_celsius(|f| &f.ignition_temperature),
        auto_ignition_temperature: blend_celsius(|f| &f.auto_ignition_temperature),
        max_flame_temperature: blend_celsius(|f| &f.max_flame_temperature),
        specific_heat: blend_wrapped!(specific_heat, KjPerKgK, new),
        thermal_conductivity: blend_wrapped!(thermal_conductivity, ThermalConductivity, new),
        thermal_diffusivity: blend_wrapped!(thermal_diffusivity, ThermalDiffusivity, new),
        bulk_density: blend_wrapped!(bulk_density, KgPerCubicMeter, new),
        surface_area_to_volume: blend_wrapped!(surface_area_to_volume, SurfaceAreaToVolume, new),
        fuel_bed_depth: blend_wrapped!(fuel_bed_depth, Meters, new),
        base_moisture: blend_wrapped!(base_moisture, Fraction, new),
        moisture_of_extinction: blend_wrapped!(moisture_of_extinction, Fraction, new),
        burn_rate_coefficient: blend_f32!(burn_rate_coefficient),
        ember_production: blend_wrapped!(ember_production, Fraction, new),
        ember_receptivity: blend_wrapped!(ember_receptivity, Fraction, new),
        max_spotting_distance: blend_wrapped!(max_spotting_distance, Meters, new),
        ember_mass_kg: blend_f32!(ember_mass_kg),
        ember_launch_velocity_factor: blend_wrapped!(ember_launch_velocity_factor, Fraction, new),
        mineral_damping: blend_wrapped!(mineral_damping, Fraction, new),
        particle_density: blend_wrapped!(particle_density, KgPerCubicMeter, new),
        effective_heating: blend_wrapped!(effective_heating, Fraction, new),
        packing_ratio: blend_wrapped!(packing_ratio, Fraction, new),
        optimum_packing_ratio: blend_wrapped!(optimum_packing_ratio, Fraction, new),
        cooling_rate: blend_wrapped!(cooling_rate, RatePerSecond, new),
        self_heating_fraction: blend_wrapped!(self_heating_fraction, Fraction, new),
        convective_heat_coefficient: blend_wrapped!(
            convective_heat_coefficient,
            HeatTransferCoefficient,
            new
        ),
        atmospheric_heat_efficiency: blend_wrapped!(atmospheric_heat_efficiency, Fraction, new),
        wind_sensitivity: blend_wrapped!(wind_sensitivity, Fraction, new),
        crown_fire_temp_multiplier: blend_wrapped!(crown_fire_temp_multiplier, Fraction, new),
        emissivity_unburned: blend_wrapped!(emissivity_unburned, Fraction, new),
        emissivity_burning: blend_wrapped!(emissivity_burning, Fraction, new),
        temperature_response_range: blend_f32!(temperature_response_range),
        slope_uphill_factor_base: blend_wrapped!(slope_uphill_factor_base, Degrees, new),
        slope_uphill_power: blend_f32!(slope_uphill_power),
        slope_downhill_divisor: blend_wrapped!(slope_downhill_divisor, Degrees, new),
        slope_factor_minimum: blend_wrapped!(slope_factor_minimum, Fraction, new),
        combustion_efficiency: blend_wrapped!(combustion_efficiency, Fraction, new),
        surface_area_geometry_factor: blend_f32!(surface_area_geometry_factor),
        flame_area_coefficient: blend_f32!(flame_area_coefficient),
        absorption_efficiency_base: blend_wrapped!(absorption_efficiency_base, Fraction, new),
        volatile_oil_content: blend_wrapped!(volatile_oil_content, Fraction, new),
        oil_vaporization_temp: blend_celsius(|f| &f.oil_vaporization_temp),
        oil_autoignition_temp: blend_celsius(|f| &f.oil_autoignition_temp),
        bark_ladder_intensity: blend_f32!(bark_ladder_intensity),
        crown_fire_threshold: blend_f32!(crown_fire_threshold),
        crown_bulk_density: blend_wrapped!(crown_bulk_density, KgPerCubicMeter, new),
        crown_base_height: blend_wrapped!(crown_base_height, Meters, new),
        foliar_moisture_content: blend_wrapped!(foliar_moisture_content, Percent, new),
        timelag_1h: blend_wrapped!(timelag_1h, Hours, new),
        timelag_10h: blend_wrapped!(timelag_10h, Hours, new),
        timelag_100h: blend_wrapped!(timelag_100h, Hours, new),
        timelag_1000h: blend_wrapped!(timelag_1000h, Hours, new),
        size_class_distribution: blend_size_class(),
    }
}

/// Standard fuel type identifiers for the lookup table.
///
/// These correspond to indices in the `FuelTable` and match
/// the `Fuel::id` field of each fuel type.
pub struct FuelTypeId;

impl FuelTypeId {
    /// Eucalyptus stringybark - extreme ladder fuel, high spotting
    pub const EUCALYPTUS_STRINGYBARK: u8 = 0;

    /// Eucalyptus smooth bark - less flammable bark
    pub const EUCALYPTUS_SMOOTH_BARK: u8 = 1;

    /// Grass - fast spreading, low intensity
    pub const GRASS: u8 = 2;

    /// Shrubland - mixed understory vegetation
    pub const SHRUBLAND: u8 = 3;

    /// Dead wood and litter - surface fuel
    pub const DEAD_WOOD_LITTER: u8 = 4;

    /// Green vegetation - high moisture, fire resistant
    pub const GREEN_VEGETATION: u8 = 5;

    /// Water - non-burnable
    pub const WATER: u8 = 6;

    /// Rock - non-burnable
    pub const ROCK: u8 = 7;
}

/// Lookup table containing all fuel type definitions.
///
/// Fuel types are accessed by index (0-255). This table is shared
/// between CPU and GPU solvers for property lookups.
#[derive(Debug, Clone)]
pub struct FuelTable {
    /// Array of fuel type definitions, indexed by fuel type ID.
    fuels: Vec<Fuel>,
}

impl Default for FuelTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FuelTable {
    /// Create a new fuel table with all standard fuel types.
    #[must_use]
    pub fn new() -> Self {
        let fuels = vec![
            Fuel::eucalyptus_stringybark(), // Index 0: Eucalyptus Stringybark
            Fuel::eucalyptus_smooth_bark(), // Index 1: Eucalyptus Smooth Bark
            Fuel::dry_grass(),              // Index 2: Grass
            Fuel::shrubland(),              // Index 3: Shrubland
            Fuel::dead_wood_litter(),       // Index 4: Dead Wood Litter
            Fuel::green_vegetation(),       // Index 5: Green Vegetation
            Fuel::water(),                  // Index 6: Water
            Fuel::rock(),                   // Index 7: Rock
        ];

        Self { fuels }
    }

    /// Get a fuel type by index.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    #[must_use]
    pub fn get(&self, index: u8) -> &Fuel {
        &self.fuels[index as usize]
    }

    /// Get a fuel type by index, returning None if out of bounds.
    #[must_use]
    pub fn try_get(&self, index: u8) -> Option<&Fuel> {
        self.fuels.get(index as usize)
    }

    /// Number of fuel types in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fuels.len()
    }

    /// Check if the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fuels.is_empty()
    }

    /// Add a custom fuel type to the table.
    ///
    /// Returns the index of the newly added fuel type.
    pub fn add(&mut self, fuel: Fuel) -> u8 {
        let index = self.fuels.len();
        assert!(index < MAX_FUEL_TYPES, "Fuel table overflow");
        self.fuels.push(fuel);
        index as u8
    }

    /// Get all fuels as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[Fuel] {
        &self.fuels
    }

    /// Blend a `LayerFuelMixture` into a single effective Fuel.
    ///
    /// Computes weighted average of all fuel properties.
    #[must_use]
    pub fn blend_mixture(&self, mixture: &LayerFuelMixture) -> Fuel {
        if mixture.is_empty() {
            return Fuel::rock();
        }

        let fuels_with_weights: Vec<(&Fuel, f32)> = mixture
            .iter()
            .filter_map(|(id, weight)| self.try_get(id).map(|fuel| (fuel, weight)))
            .collect();

        blend_fuels(&fuels_with_weights)
    }
}

/// Grid of per-cell, per-layer fuel type assignments.
///
/// This structure maps spatial cells to their fuel type configuration,
/// enabling spatially varying fuel properties across the simulation domain.
#[derive(Debug, Clone)]
pub struct FuelGrid {
    /// Width of the grid in cells.
    width: usize,

    /// Height of the grid in cells.
    height: usize,

    /// Per-cell fuel type assignments (row-major order).
    /// Each cell has fuel types for surface, shrub, and canopy layers.
    cell_fuel_types: Vec<CellFuelTypes>,

    /// Lookup table for fuel properties.
    fuel_table: FuelTable,
}

impl FuelGrid {
    /// Create a new fuel grid with uniform fuel types.
    ///
    /// # Arguments
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `default_fuel_types` - Default fuel configuration for all cells
    #[must_use]
    pub fn new(width: usize, height: usize, default_fuel_types: CellFuelTypes) -> Self {
        let num_cells = width * height;
        Self {
            width,
            height,
            cell_fuel_types: vec![default_fuel_types; num_cells],
            fuel_table: FuelTable::new(),
        }
    }

    /// Create a fuel grid for eucalyptus forest.
    #[must_use]
    pub fn eucalyptus_forest(width: usize, height: usize) -> Self {
        Self::new(width, height, CellFuelTypes::eucalyptus_forest())
    }

    /// Create a fuel grid for grassland.
    #[must_use]
    pub fn grassland(width: usize, height: usize) -> Self {
        Self::new(width, height, CellFuelTypes::grassland())
    }

    /// Get grid dimensions.
    #[must_use]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get the fuel table.
    #[must_use]
    pub fn fuel_table(&self) -> &FuelTable {
        &self.fuel_table
    }

    /// Get mutable access to the fuel table.
    pub fn fuel_table_mut(&mut self) -> &mut FuelTable {
        &mut self.fuel_table
    }

    /// Get the fuel type configuration for a cell.
    ///
    /// # Arguments
    /// * `x` - Cell x coordinate
    /// * `y` - Cell y coordinate
    #[must_use]
    pub fn get_cell_fuel_types(&self, x: usize, y: usize) -> CellFuelTypes {
        let idx = y * self.width + x;
        self.cell_fuel_types[idx]
    }

    /// Set the fuel type configuration for a cell.
    ///
    /// # Arguments
    /// * `x` - Cell x coordinate
    /// * `y` - Cell y coordinate
    /// * `fuel_types` - Fuel type configuration to set
    pub fn set_cell_fuel_types(&mut self, x: usize, y: usize, fuel_types: CellFuelTypes) {
        let idx = y * self.width + x;
        self.cell_fuel_types[idx] = fuel_types;
    }

    /// Get the Fuel properties for a specific cell and layer.
    ///
    /// # Arguments
    /// * `x` - Cell x coordinate
    /// * `y` - Cell y coordinate
    /// * `layer` - Fuel layer (0=surface, 1=shrub, 2=canopy)
    #[must_use]
    pub fn get_fuel(&self, x: usize, y: usize, layer: usize) -> &Fuel {
        let cell_types = self.get_cell_fuel_types(x, y);
        let fuel_id = match layer {
            1 => cell_types.shrub,
            2 => cell_types.canopy,
            _ => cell_types.surface, // Default to surface (layer 0 and any invalid)
        };
        self.fuel_table.get(fuel_id)
    }

    /// Get the surface layer Fuel for a cell.
    #[must_use]
    pub fn get_surface_fuel(&self, x: usize, y: usize) -> &Fuel {
        self.get_fuel(x, y, 0)
    }

    /// Get the shrub layer Fuel for a cell.
    #[must_use]
    pub fn get_shrub_fuel(&self, x: usize, y: usize) -> &Fuel {
        self.get_fuel(x, y, 1)
    }

    /// Get the canopy layer Fuel for a cell.
    #[must_use]
    pub fn get_canopy_fuel(&self, x: usize, y: usize) -> &Fuel {
        self.get_fuel(x, y, 2)
    }

    // ========================================================================
    // MIXTURE-BASED FUEL LOOKUPS
    // ========================================================================

    /// Get blended Fuel from a mixture for a specific layer.
    ///
    /// This computes effective fuel properties as weighted averages of all
    /// fuels in the mixture.
    ///
    /// # Arguments
    /// * `mixture` - The fuel mixture for a single layer
    ///
    /// # Returns
    /// A new Fuel with blended properties
    #[must_use]
    pub fn blend_layer_mixture(&self, mixture: &LayerFuelMixture) -> Fuel {
        self.fuel_table.blend_mixture(mixture)
    }

    /// Get blended Fuel for all layers from cell mixtures.
    ///
    /// # Arguments
    /// * `mixtures` - The fuel mixtures for a cell
    ///
    /// # Returns
    /// Tuple of (surface, shrub, canopy) blended Fuels
    #[must_use]
    pub fn blend_cell_mixtures(&self, mixtures: &CellFuelMixtures) -> (Fuel, Fuel, Fuel) {
        (
            self.blend_layer_mixture(&mixtures.surface),
            self.blend_layer_mixture(&mixtures.shrub),
            self.blend_layer_mixture(&mixtures.canopy),
        )
    }

    /// Get all cell fuel types as a slice (for GPU upload).
    #[must_use]
    pub fn cell_fuel_types_slice(&self) -> &[CellFuelTypes] {
        &self.cell_fuel_types
    }

    /// Set fuel types for a rectangular region.
    ///
    /// # Arguments
    /// * `x_start`, `y_start` - Start coordinates (inclusive)
    /// * `x_end`, `y_end` - End coordinates (exclusive)
    /// * `fuel_types` - Fuel configuration to apply
    pub fn set_region(
        &mut self,
        x_start: usize,
        y_start: usize,
        x_end: usize,
        y_end: usize,
        fuel_types: CellFuelTypes,
    ) {
        let x_end = x_end.min(self.width);
        let y_end = y_end.min(self.height);

        for y in y_start..y_end {
            for x in x_start..x_end {
                self.set_cell_fuel_types(x, y, fuel_types);
            }
        }
    }

    /// Initialize fuel grid from terrain height data.
    ///
    /// Assigns fuel types based on elevation:
    /// - < 0m: Water
    /// - 0-500m: Eucalyptus forest
    /// - 500-1000m: Mixed forest/shrubland
    /// - > 1000m: Grassland/alpine
    ///
    /// # Arguments
    /// * `elevation_data` - Elevation values in meters (row-major)
    pub fn initialize_from_elevation(&mut self, elevation_data: &[f32]) {
        for (idx, &elevation) in elevation_data.iter().enumerate() {
            if idx >= self.cell_fuel_types.len() {
                break;
            }

            let fuel_types = if elevation < 0.0 {
                CellFuelTypes::uniform(FuelTypeId::WATER)
            } else if elevation < 500.0 {
                CellFuelTypes::eucalyptus_forest()
            } else if elevation < 1000.0 {
                CellFuelTypes::shrubland()
            } else {
                CellFuelTypes::grassland()
            };

            self.cell_fuel_types[idx] = fuel_types;
        }
    }

    /// Create packed fuel property arrays for GPU upload.
    ///
    /// Returns arrays of key fuel properties indexed by cell, suitable
    /// for GPU storage buffers.
    ///
    /// # Returns
    /// Tuple of (`ignition_temp_k`, `moisture_extinction`, `heat_content_kj`,
    ///           `thermal_diffusivity`, `specific_heat_j`, `burn_rate_coeff`)
    /// Each array has `width * height * 3` elements (3 layers per cell).
    #[must_use]
    pub fn pack_fuel_properties_for_gpu(&self) -> FuelPropertyBuffers {
        let num_elements = self.width * self.height * NUM_FUEL_LAYERS;

        let mut ignition_temp_k = Vec::with_capacity(num_elements);
        let mut moisture_extinction = Vec::with_capacity(num_elements);
        let mut heat_content_kj = Vec::with_capacity(num_elements);
        let mut thermal_diffusivity = Vec::with_capacity(num_elements);
        let mut specific_heat_j = Vec::with_capacity(num_elements);
        let mut burn_rate_coeff = Vec::with_capacity(num_elements);
        let mut self_heating_frac = Vec::with_capacity(num_elements);

        for cell_types in &self.cell_fuel_types {
            // Pack properties for all 3 layers of this cell
            for layer in 0..NUM_FUEL_LAYERS {
                let fuel_id = match layer {
                    0 => cell_types.surface,
                    1 => cell_types.shrub,
                    _ => cell_types.canopy,
                };
                let fuel = self.fuel_table.get(fuel_id);

                ignition_temp_k.push((*fuel.ignition_temperature + 273.15) as f32);
                moisture_extinction.push(*fuel.moisture_of_extinction);
                heat_content_kj.push(*fuel.heat_content);
                thermal_diffusivity.push(*fuel.thermal_diffusivity);
                specific_heat_j.push(*fuel.specific_heat * 1000.0); // kJ to J
                burn_rate_coeff.push(fuel.burn_rate_coefficient);
                self_heating_frac.push(*fuel.self_heating_fraction);
            }
        }

        FuelPropertyBuffers {
            ignition_temp_k,
            moisture_extinction,
            heat_content_kj,
            thermal_diffusivity,
            specific_heat_j,
            burn_rate_coeff,
            self_heating_frac,
        }
    }
}

/// Packed fuel property buffers for GPU upload.
///
/// Each buffer contains `width * height * 3` elements
/// (3 layers per cell, in cell-major order: `[cell0_surf, cell0_shrub, cell0_canopy, cell1_surf, ...]`)
#[derive(Debug, Clone)]
pub struct FuelPropertyBuffers {
    /// Ignition temperature in Kelvin per cell per layer.
    pub ignition_temp_k: Vec<f32>,

    /// Moisture of extinction (fraction 0-1) per cell per layer.
    pub moisture_extinction: Vec<f32>,

    /// Heat content in kJ/kg per cell per layer.
    pub heat_content_kj: Vec<f32>,

    /// Thermal diffusivity in m²/s per cell per layer.
    pub thermal_diffusivity: Vec<f32>,

    /// Specific heat in J/(kg·K) per cell per layer.
    pub specific_heat_j: Vec<f32>,

    /// Burn rate coefficient per cell per layer.
    pub burn_rate_coeff: Vec<f32>,

    /// Self-heating fraction (0-1) per cell per layer.
    pub self_heating_frac: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_table_creation() {
        let table = FuelTable::new();
        assert_eq!(table.len(), 8);
        assert!(!table.is_empty());
    }

    #[test]
    fn test_fuel_table_lookup() {
        let table = FuelTable::new();

        // Array indices correspond to FuelTypeId constants (0-7)
        // The actual Fuel::id values are different (1, 2, 3, etc.)
        let stringybark = table.get(FuelTypeId::EUCALYPTUS_STRINGYBARK);
        assert_eq!(stringybark.name, "Eucalyptus Stringybark");

        let grass = table.get(FuelTypeId::GRASS);
        assert_eq!(grass.name, "Dry Grass");
    }

    #[test]
    fn test_cell_fuel_types_presets() {
        let forest = CellFuelTypes::eucalyptus_forest();
        assert_eq!(forest.surface, FuelTypeId::DEAD_WOOD_LITTER);
        assert_eq!(forest.shrub, FuelTypeId::EUCALYPTUS_STRINGYBARK);
        assert_eq!(forest.canopy, FuelTypeId::EUCALYPTUS_STRINGYBARK);

        let grassland = CellFuelTypes::grassland();
        assert_eq!(grassland.surface, FuelTypeId::GRASS);
        assert_eq!(grassland.shrub, FuelTypeId::GRASS);
        assert_eq!(grassland.canopy, FuelTypeId::GRASS);
    }

    #[test]
    fn test_fuel_grid_creation() {
        let grid = FuelGrid::eucalyptus_forest(10, 10);
        assert_eq!(grid.dimensions(), (10, 10));

        let cell = grid.get_cell_fuel_types(5, 5);
        assert_eq!(cell.surface, FuelTypeId::DEAD_WOOD_LITTER);
    }

    #[test]
    fn test_fuel_grid_set_region() {
        let mut grid = FuelGrid::eucalyptus_forest(20, 20);

        // Create a grassland region in the middle
        grid.set_region(5, 5, 15, 15, CellFuelTypes::grassland());

        // Check inside region
        let inside = grid.get_cell_fuel_types(10, 10);
        assert_eq!(inside.surface, FuelTypeId::GRASS);

        // Check outside region
        let outside = grid.get_cell_fuel_types(0, 0);
        assert_eq!(outside.surface, FuelTypeId::DEAD_WOOD_LITTER);
    }

    #[test]
    fn test_fuel_property_lookup() {
        let grid = FuelGrid::eucalyptus_forest(10, 10);

        let surface_fuel = grid.get_surface_fuel(5, 5);
        // Surface is dead wood litter (FuelTypeId::DEAD_WOOD_LITTER = 4)
        assert_eq!(surface_fuel.name, "Dead Wood/Litter");

        let canopy_fuel = grid.get_canopy_fuel(5, 5);
        // Canopy is stringybark (FuelTypeId::EUCALYPTUS_STRINGYBARK = 0)
        assert_eq!(canopy_fuel.name, "Eucalyptus Stringybark");
    }

    #[test]
    fn test_pack_fuel_properties() {
        let grid = FuelGrid::eucalyptus_forest(2, 2);
        let props = grid.pack_fuel_properties_for_gpu();

        // 2x2 grid = 4 cells, 3 layers each = 12 elements
        assert_eq!(props.ignition_temp_k.len(), 12);
        assert_eq!(props.heat_content_kj.len(), 12);

        // All values should be positive
        for val in &props.ignition_temp_k {
            assert!(*val > 0.0);
        }
    }

    #[test]
    fn test_initialize_from_elevation() {
        let mut grid = FuelGrid::grassland(4, 4);

        // Create elevation data with variety
        let elevations: Vec<f32> = vec![
            -5.0, 100.0, 200.0, 300.0, // Row 0: water, forest, forest, forest
            400.0, 600.0, 800.0, 900.0, // Row 1: forest, shrub, shrub, shrub
            1100.0, 1200.0, 500.0, 450.0, // Row 2: grass, grass, shrub, forest
            -10.0, 50.0, 750.0, 1500.0, // Row 3: water, forest, shrub, grass
        ];

        grid.initialize_from_elevation(&elevations);

        // Check water cell
        let water_cell = grid.get_cell_fuel_types(0, 0);
        assert_eq!(water_cell.surface, FuelTypeId::WATER);

        // Check forest cell
        let forest_cell = grid.get_cell_fuel_types(1, 0);
        assert_eq!(forest_cell.shrub, FuelTypeId::EUCALYPTUS_STRINGYBARK);

        // Check high elevation grassland
        let grass_cell = grid.get_cell_fuel_types(0, 2);
        assert_eq!(grass_cell.surface, FuelTypeId::GRASS);
    }

    // ========================================================================
    // MIXTURE SYSTEM TESTS
    // ========================================================================

    #[test]
    fn test_layer_fuel_mixture_single() {
        let mixture = LayerFuelMixture::single(FuelTypeId::GRASS);
        assert_eq!(mixture.fuel_count(), 1);
        assert!(!mixture.is_empty());

        let mut count = 0;
        for (id, weight) in mixture.iter() {
            assert_eq!(id, FuelTypeId::GRASS);
            assert!((weight - 1.0).abs() < 0.001);
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_layer_fuel_mixture_blend2() {
        let mixture =
            LayerFuelMixture::blend2(FuelTypeId::GRASS, 0.6, FuelTypeId::DEAD_WOOD_LITTER, 0.4);
        assert_eq!(mixture.fuel_count(), 2);

        let pairs: Vec<_> = mixture.iter().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, FuelTypeId::GRASS);
        assert!((pairs[0].1 - 0.6).abs() < 0.001);
        assert_eq!(pairs[1].0, FuelTypeId::DEAD_WOOD_LITTER);
        assert!((pairs[1].1 - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_layer_fuel_mixture_normalize() {
        let mut mixture =
            LayerFuelMixture::blend2(FuelTypeId::GRASS, 3.0, FuelTypeId::SHRUBLAND, 2.0);
        mixture.normalize();

        let pairs: Vec<_> = mixture.iter().collect();
        assert!((pairs[0].1 - 0.6).abs() < 0.001); // 3/5
        assert!((pairs[1].1 - 0.4).abs() < 0.001); // 2/5
    }

    #[test]
    fn test_blend_fuels_single() {
        let table = FuelTable::new();
        let grass = table.get(FuelTypeId::GRASS);

        let blended = blend_fuels(&[(grass, 1.0)]);
        assert_eq!(blended.name, grass.name);
        assert!((*blended.heat_content - *grass.heat_content).abs() < 0.1);
    }

    #[test]
    fn test_blend_fuels_two() {
        let table = FuelTable::new();
        let grass = table.get(FuelTypeId::GRASS);
        let litter = table.get(FuelTypeId::DEAD_WOOD_LITTER);

        // 50/50 blend
        let blended = blend_fuels(&[(grass, 0.5), (litter, 0.5)]);

        // Heat content should be average
        let expected_heat = f32::midpoint(*grass.heat_content, *litter.heat_content);
        assert!((*blended.heat_content - expected_heat).abs() < 0.1);

        // Ignition temp should be average
        let expected_ignition =
            f64::midpoint(*grass.ignition_temperature, *litter.ignition_temperature);
        assert!((*blended.ignition_temperature - expected_ignition).abs() < 0.1);
    }

    #[test]
    fn test_fuel_table_blend_mixture() {
        let table = FuelTable::new();

        let mixture = LayerFuelMixture::blend2(
            FuelTypeId::EUCALYPTUS_STRINGYBARK,
            0.7,
            FuelTypeId::GRASS,
            0.3,
        );
        let blended = table.blend_mixture(&mixture);

        // Should be weighted toward stringybark
        let stringybark = table.get(FuelTypeId::EUCALYPTUS_STRINGYBARK);
        let grass = table.get(FuelTypeId::GRASS);
        let expected_heat = *stringybark.heat_content * 0.7 + *grass.heat_content * 0.3;
        assert!((*blended.heat_content - expected_heat).abs() < 0.1);
    }

    #[test]
    fn test_cell_fuel_mixtures_from_simple() {
        let simple = CellFuelTypes::eucalyptus_forest();
        let mixtures = CellFuelMixtures::from_simple(simple);

        assert_eq!(mixtures.surface.fuel_count(), 1);
        assert_eq!(mixtures.surface.fuel_ids[0], FuelTypeId::DEAD_WOOD_LITTER);
        assert_eq!(
            mixtures.shrub.fuel_ids[0],
            FuelTypeId::EUCALYPTUS_STRINGYBARK
        );
        assert_eq!(
            mixtures.canopy.fuel_ids[0],
            FuelTypeId::EUCALYPTUS_STRINGYBARK
        );
    }

    #[test]
    fn test_cell_fuel_mixtures_forest_mixed() {
        let mixtures = CellFuelMixtures::eucalyptus_forest_mixed();

        // Surface should have 3 fuels
        assert_eq!(mixtures.surface.fuel_count(), 3);

        // Shrub should have 2 fuels
        assert_eq!(mixtures.shrub.fuel_count(), 2);

        // Canopy should have 1 fuel
        assert_eq!(mixtures.canopy.fuel_count(), 1);
    }

    #[test]
    fn test_fuel_grid_blend_cell_mixtures() {
        let grid = FuelGrid::eucalyptus_forest(10, 10);
        let mixtures = CellFuelMixtures::eucalyptus_forest_mixed();

        let (surface, shrub, canopy) = grid.blend_cell_mixtures(&mixtures);

        // All should be valid fuels with positive heat content
        assert!(*surface.heat_content > 0.0);
        assert!(*shrub.heat_content > 0.0);
        assert!(*canopy.heat_content > 0.0);

        // Blended name should indicate it's a blend
        assert!(surface.name.starts_with("Blended"));
        assert!(shrub.name.starts_with("Blended"));

        // Canopy is single fuel, so not "Blended"
        assert_eq!(canopy.name, "Eucalyptus Stringybark");
    }
}
