//! Vertical fuel layer modeling for stratified fire behavior.
//!
//! Implements discrete fuel layers (Surface, Shrub, Canopy) based on
//! Scott & Burgan (2005) fuel strata concepts. This module provides
//! per-cell fuel state tracking for vertically stratified fire simulation.
//!
//! # Scientific References
//!
//! - Scott, J.H. & Burgan, R.E. (2005). "Standard Fire Behavior Fuel Models"
//!   USDA Forest Service, RMRS-GTR-153
//! - Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire"
//!   Canadian Journal of Forest Research, 7(1), 23-34

/// Discrete vertical fuel layers.
///
/// Represents the three primary fuel strata in forest ecosystems,
/// each with distinct fire behavior characteristics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FuelLayer {
    /// Surface fuels: litter, grass, herbs (0-0.5m)
    ///
    /// Primary ignition zone for most fires. Includes dead leaves,
    /// fine woody debris, and herbaceous vegetation.
    Surface = 0,

    /// Shrub/ladder fuels: understory, bark (0.5-3m)
    ///
    /// Critical for vertical fire spread. Includes shrubs, bark strips,
    /// and regenerating vegetation that bridges surface to canopy.
    Shrub = 1,

    /// Canopy fuels: tree crowns (3m+)
    ///
    /// Upper forest stratum. Crown fire occurs when fire reaches
    /// and spreads through this layer.
    Canopy = 2,
}

impl FuelLayer {
    /// Height range for this layer (min, max) in meters.
    ///
    /// Based on Scott & Burgan (2005) standard fuel model stratification.
    ///
    /// # Returns
    /// Tuple of (min height, max height) in meters
    #[must_use]
    pub const fn height_range(&self) -> (f32, f32) {
        match self {
            FuelLayer::Surface => (0.0, 0.5),
            FuelLayer::Shrub => (0.5, 3.0),
            FuelLayer::Canopy => (3.0, 50.0),
        }
    }

    /// Representative height for heat transfer calculations (meters).
    ///
    /// Returns the geometric centroid of each layer's height range,
    /// used for calculating vertical heat flux distances.
    #[must_use]
    pub const fn representative_height(&self) -> f32 {
        match self {
            FuelLayer::Surface => 0.25, // Mid-point of 0-0.5m
            FuelLayer::Shrub => 1.75,   // Mid-point of 0.5-3m
            FuelLayer::Canopy => 10.0,  // Representative crown height
        }
    }

    /// Returns the layer above this one, if any.
    #[must_use]
    pub const fn layer_above(&self) -> Option<FuelLayer> {
        match self {
            FuelLayer::Surface => Some(FuelLayer::Shrub),
            FuelLayer::Shrub => Some(FuelLayer::Canopy),
            FuelLayer::Canopy => None,
        }
    }

    /// Returns the layer below this one, if any.
    #[must_use]
    pub const fn layer_below(&self) -> Option<FuelLayer> {
        match self {
            FuelLayer::Surface => None,
            FuelLayer::Shrub => Some(FuelLayer::Surface),
            FuelLayer::Canopy => Some(FuelLayer::Shrub),
        }
    }
}

/// Per-layer fuel state.
///
/// Tracks the thermodynamic and combustion state of fuel within
/// a single vertical layer. All values are intensive (per-area)
/// or dimensionless to allow layer-independent calculations.
#[derive(Clone, Debug)]
pub struct LayerState {
    /// Fuel load remaining (kg/m²)
    pub fuel_load: f32,

    /// Moisture content (fraction, 0-1)
    ///
    /// Fraction of fuel mass that is water. Must be accounted for
    /// in heat transfer calculations as moisture absorbs latent heat
    /// (2.26 MJ/kg) before temperature can rise.
    pub moisture: f32,

    /// Temperature (K)
    ///
    /// Current temperature of fuel in this layer.
    pub temperature: f32,

    /// Is this layer currently burning?
    pub burning: bool,

    /// Heat received from lower layers this timestep (J/m²)
    ///
    /// Accumulated heat flux from burning layers below.
    /// Reset each timestep after being applied.
    pub heat_received: f32,
}

impl Default for LayerState {
    fn default() -> Self {
        Self {
            fuel_load: 0.0,
            moisture: 0.1,       // 10% default moisture
            temperature: 293.15, // ~20°C ambient
            burning: false,
            heat_received: 0.0,
        }
    }
}

impl LayerState {
    /// Create a new layer state with specified fuel load and moisture.
    ///
    /// # Arguments
    /// * `fuel_load` - Fuel load in kg/m²
    /// * `moisture` - Moisture content as fraction (0-1)
    #[must_use]
    pub fn new(fuel_load: f32, moisture: f32) -> Self {
        Self {
            fuel_load,
            moisture: moisture.clamp(0.0, 1.0),
            temperature: 293.15,
            burning: false,
            heat_received: 0.0,
        }
    }

    /// Check if this layer has fuel available for combustion.
    #[must_use]
    pub fn has_fuel(&self) -> bool {
        self.fuel_load > 1e-6
    }
}

/// Complete layered fuel cell with three strata.
///
/// Represents a single spatial cell's vertical fuel structure,
/// enabling stratified fire behavior simulation where ground-level
/// fire does not immediately affect elevated fuels.
#[derive(Clone, Debug)]
pub struct LayeredFuelCell {
    /// Surface layer state (0-0.5m)
    pub surface: LayerState,

    /// Shrub layer state (0.5-3m)
    pub shrub: LayerState,

    /// Canopy layer state (3m+)
    pub canopy: LayerState,
}

impl Default for LayeredFuelCell {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredFuelCell {
    /// Create with default empty layers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            surface: LayerState::default(),
            shrub: LayerState::default(),
            canopy: LayerState::default(),
        }
    }

    /// Create from surface fuel load only (legacy compatibility).
    ///
    /// Useful for converting single-layer fuel data to the layered model.
    /// All fuel is placed in the surface layer.
    ///
    /// # Arguments
    /// * `fuel_load` - Surface fuel load in kg/m²
    /// * `moisture` - Moisture content as fraction (0-1)
    #[must_use]
    pub fn from_surface_fuel(fuel_load: f32, moisture: f32) -> Self {
        Self {
            surface: LayerState::new(fuel_load, moisture),
            shrub: LayerState::default(),
            canopy: LayerState::default(),
        }
    }

    /// Get layer by enum reference.
    #[must_use]
    pub fn layer(&self, layer: FuelLayer) -> &LayerState {
        match layer {
            FuelLayer::Surface => &self.surface,
            FuelLayer::Shrub => &self.shrub,
            FuelLayer::Canopy => &self.canopy,
        }
    }

    /// Get mutable layer by enum.
    pub fn layer_mut(&mut self, layer: FuelLayer) -> &mut LayerState {
        match layer {
            FuelLayer::Surface => &mut self.surface,
            FuelLayer::Shrub => &mut self.shrub,
            FuelLayer::Canopy => &mut self.canopy,
        }
    }

    /// Check if shrub should ignite based on surface fire intensity.
    ///
    /// Shrub layer ignites when surface fire intensity exceeds approximately
    /// 500 kW/m, representing the threshold for vertical flame propagation.
    ///
    /// # Arguments
    /// * `surface_intensity_kw_m` - Surface fire intensity in kW/m (Byram's intensity)
    ///
    /// # Scientific Basis
    /// Threshold based on empirical observations of ladder fuel ignition
    /// in Australian eucalypt forests. See: Cheney et al. (2012).
    pub fn check_shrub_ignition(&mut self, surface_intensity_kw_m: f32) {
        const SHRUB_IGNITION_THRESHOLD_KW_M: f32 = 500.0;

        if !self.shrub.burning
            && self.shrub.has_fuel()
            && surface_intensity_kw_m >= SHRUB_IGNITION_THRESHOLD_KW_M
        {
            self.shrub.burning = true;
        }
    }

    /// Check if canopy should ignite using Van Wagner (1977) criterion.
    ///
    /// Crown fire initiation occurs when surface fire intensity exceeds
    /// the critical intensity calculated from canopy properties.
    ///
    /// # Van Wagner (1977) Critical Intensity Formula
    ///
    /// ```text
    /// I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
    /// ```
    ///
    /// Where:
    /// - CBH: Canopy base height (m)
    /// - FMC: Foliar moisture content (%)
    ///
    /// # Arguments
    /// * `surface_intensity_kw_m` - Surface fire intensity in kW/m
    /// * `canopy_base_height_m` - Height to canopy base in meters
    /// * `foliar_moisture_percent` - Foliar moisture content in percent (not fraction)
    pub fn check_canopy_ignition(
        &mut self,
        surface_intensity_kw_m: f32,
        canopy_base_height_m: f32,
        foliar_moisture_percent: f32,
    ) {
        // Guard against invalid inputs
        if canopy_base_height_m <= 0.0 || !self.canopy.has_fuel() || self.canopy.burning {
            return;
        }

        // Van Wagner (1977) critical surface intensity for crown fire initiation
        // I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
        let critical_intensity =
            (0.010 * canopy_base_height_m * (460.0 + 25.9 * foliar_moisture_percent)).powf(1.5);

        if surface_intensity_kw_m >= critical_intensity {
            self.canopy.burning = true;
        }
    }

    /// Get total fuel load across all layers (kg/m²).
    #[must_use]
    pub fn total_fuel_load(&self) -> f32 {
        self.surface.fuel_load + self.shrub.fuel_load + self.canopy.fuel_load
    }

    /// Check if any layer is currently burning.
    #[must_use]
    pub fn is_burning(&self) -> bool {
        self.surface.burning || self.shrub.burning || self.canopy.burning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuel_layer_heights() {
        // Surface: 0-0.5m
        let (min, max) = FuelLayer::Surface.height_range();
        assert!((min - 0.0).abs() < f32::EPSILON);
        assert!((max - 0.5).abs() < f32::EPSILON);

        // Shrub: 0.5-3m
        let (min, max) = FuelLayer::Shrub.height_range();
        assert!((min - 0.5).abs() < f32::EPSILON);
        assert!((max - 3.0).abs() < f32::EPSILON);

        // Canopy: 3-50m
        let (min, max) = FuelLayer::Canopy.height_range();
        assert!((min - 3.0).abs() < f32::EPSILON);
        assert!((max - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fuel_layer_representative_heights() {
        // Representative heights should be within layer bounds
        for layer in [FuelLayer::Surface, FuelLayer::Shrub, FuelLayer::Canopy] {
            let (min, max) = layer.height_range();
            let rep = layer.representative_height();
            assert!(
                rep >= min && rep <= max,
                "Layer {layer:?} representative height {rep} not in range [{min}, {max}]",
            );
        }
    }

    #[test]
    fn shrub_ignition_threshold() {
        let mut cell = LayeredFuelCell::new();
        cell.shrub.fuel_load = 1.0; // 1 kg/m²

        // Below threshold - should NOT ignite
        cell.check_shrub_ignition(499.0);
        assert!(
            !cell.shrub.burning,
            "Shrub should not ignite below 500 kW/m"
        );

        // At threshold - should ignite
        cell.check_shrub_ignition(500.0);
        assert!(cell.shrub.burning, "Shrub should ignite at 500 kW/m");
    }

    #[test]
    fn shrub_ignition_requires_fuel() {
        let mut cell = LayeredFuelCell::new();
        cell.shrub.fuel_load = 0.0; // No fuel

        // Above threshold but no fuel
        cell.check_shrub_ignition(600.0);
        assert!(!cell.shrub.burning, "Shrub should not ignite without fuel");
    }

    #[test]
    fn canopy_ignition_van_wagner() {
        let mut cell = LayeredFuelCell::new();
        cell.canopy.fuel_load = 1.0;

        // Van Wagner (1977): I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
        // CBH = 5m, FMC = 100%
        // I_critical = (0.010 × 5 × (460 + 25.9 × 100))^1.5
        //            = (0.05 × 3050)^1.5
        //            = 152.5^1.5
        //            ≈ 1884 kW/m
        let cbh: f32 = 5.0;
        let fmc: f32 = 100.0;
        let expected_critical = (0.010_f32 * cbh * (460.0 + 25.9 * fmc)).powf(1.5);

        // Below critical - should NOT ignite
        cell.check_canopy_ignition(expected_critical - 1.0, cbh, fmc);
        assert!(
            !cell.canopy.burning,
            "Canopy should not ignite below critical intensity"
        );

        // At critical - should ignite
        cell.check_canopy_ignition(expected_critical, cbh, fmc);
        assert!(
            cell.canopy.burning,
            "Canopy should ignite at Van Wagner critical intensity"
        );
    }

    #[test]
    fn canopy_ignition_requires_fuel() {
        let mut cell = LayeredFuelCell::new();
        cell.canopy.fuel_load = 0.0; // No fuel

        // Very high intensity but no fuel
        cell.check_canopy_ignition(10000.0, 5.0, 100.0);
        assert!(
            !cell.canopy.burning,
            "Canopy should not ignite without fuel"
        );
    }

    #[test]
    fn canopy_ignition_requires_valid_cbh() {
        let mut cell = LayeredFuelCell::new();
        cell.canopy.fuel_load = 1.0;

        // Zero CBH should not cause ignition (invalid input)
        cell.check_canopy_ignition(10000.0, 0.0, 100.0);
        assert!(
            !cell.canopy.burning,
            "Canopy should not ignite with zero CBH"
        );

        // Negative CBH should not cause ignition (invalid input)
        cell.check_canopy_ignition(10000.0, -5.0, 100.0);
        assert!(
            !cell.canopy.burning,
            "Canopy should not ignite with negative CBH"
        );
    }

    #[test]
    fn layer_navigation() {
        // Test layer_above
        assert_eq!(FuelLayer::Surface.layer_above(), Some(FuelLayer::Shrub));
        assert_eq!(FuelLayer::Shrub.layer_above(), Some(FuelLayer::Canopy));
        assert_eq!(FuelLayer::Canopy.layer_above(), None);

        // Test layer_below
        assert_eq!(FuelLayer::Surface.layer_below(), None);
        assert_eq!(FuelLayer::Shrub.layer_below(), Some(FuelLayer::Surface));
        assert_eq!(FuelLayer::Canopy.layer_below(), Some(FuelLayer::Shrub));
    }

    #[test]
    fn layered_fuel_cell_accessors() {
        let mut cell = LayeredFuelCell::new();
        cell.surface.fuel_load = 1.0;
        cell.shrub.fuel_load = 2.0;
        cell.canopy.fuel_load = 3.0;

        // Test immutable access
        assert!((cell.layer(FuelLayer::Surface).fuel_load - 1.0).abs() < f32::EPSILON);
        assert!((cell.layer(FuelLayer::Shrub).fuel_load - 2.0).abs() < f32::EPSILON);
        assert!((cell.layer(FuelLayer::Canopy).fuel_load - 3.0).abs() < f32::EPSILON);

        // Test mutable access
        cell.layer_mut(FuelLayer::Surface).fuel_load = 4.0;
        assert!((cell.surface.fuel_load - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn total_fuel_load() {
        let mut cell = LayeredFuelCell::new();
        cell.surface.fuel_load = 1.0;
        cell.shrub.fuel_load = 2.0;
        cell.canopy.fuel_load = 3.0;

        assert!((cell.total_fuel_load() - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn is_burning() {
        let mut cell = LayeredFuelCell::new();
        assert!(!cell.is_burning());

        cell.surface.burning = true;
        assert!(cell.is_burning());

        cell.surface.burning = false;
        cell.shrub.burning = true;
        assert!(cell.is_burning());

        cell.shrub.burning = false;
        cell.canopy.burning = true;
        assert!(cell.is_burning());
    }
}
