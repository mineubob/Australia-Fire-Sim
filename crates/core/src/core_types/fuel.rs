use super::units::{
    Celsius, Degrees, Fraction, HeatTransferCoefficient, Hours, KgPerCubicMeter, KjPerKg, KjPerKgK,
    Meters, Percent, RatePerSecond, SurfaceAreaToVolume, ThermalConductivity, ThermalDiffusivity,
};
use serde::{Deserialize, Serialize};

/// Bark properties that affect fire behavior
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BarkProperties {
    /// Numeric ID for the bark type
    pub bark_type_id: u8,
    /// 0-1 scale, how much it acts as ladder fuel
    pub ladder_fuel_factor: Fraction,
    /// 0-1 scale, ignition ease
    pub flammability: Fraction,
    /// 0-1 scale, how much bark sheds as embers
    pub shedding_rate: Fraction,
    /// 0-1 scale, protection of inner wood
    pub insulation_factor: Fraction,
    /// Affects airflow and heat retention (0-1)
    pub surface_roughness: Fraction,
}

impl BarkProperties {
    /// Smooth bark - minimal ladder fuel
    pub const SMOOTH: BarkProperties = BarkProperties {
        bark_type_id: 0,
        ladder_fuel_factor: Fraction::new(0.1),
        flammability: Fraction::new(0.3),
        shedding_rate: Fraction::new(0.1),
        insulation_factor: Fraction::new(0.2),
        surface_roughness: Fraction::new(0.2),
    };

    /// Fibrous bark - moderate ladder fuel
    pub const FIBROUS: BarkProperties = BarkProperties {
        bark_type_id: 1,
        ladder_fuel_factor: Fraction::new(0.5),
        flammability: Fraction::new(0.6),
        shedding_rate: Fraction::new(0.4),
        insulation_factor: Fraction::new(0.5),
        surface_roughness: Fraction::new(0.6),
    };

    /// Stringybark - EXTREME ladder fuel (causes crown fires)
    pub const STRINGYBARK: BarkProperties = BarkProperties {
        bark_type_id: 2,
        ladder_fuel_factor: Fraction::new(1.0),
        flammability: Fraction::new(0.9),
        shedding_rate: Fraction::new(0.8),
        insulation_factor: Fraction::new(0.4),
        surface_roughness: Fraction::new(0.9),
    };

    /// Ironbark - dense, slow burning
    pub const IRONBARK: BarkProperties = BarkProperties {
        bark_type_id: 3,
        ladder_fuel_factor: Fraction::new(0.2),
        flammability: Fraction::new(0.4),
        shedding_rate: Fraction::new(0.2),
        insulation_factor: Fraction::new(0.8),
        surface_roughness: Fraction::new(0.4),
    };

    /// Paperbark - highly flammable
    pub const PAPERBARK: BarkProperties = BarkProperties {
        bark_type_id: 4,
        ladder_fuel_factor: Fraction::new(0.7),
        flammability: Fraction::new(0.95),
        shedding_rate: Fraction::new(0.9),
        insulation_factor: Fraction::new(0.3),
        surface_roughness: Fraction::new(0.5),
    };

    /// Non-bark (for non-tree fuels)
    pub const NONE: BarkProperties = BarkProperties {
        bark_type_id: 255,
        ladder_fuel_factor: Fraction::new(0.0),
        flammability: Fraction::new(0.0),
        shedding_rate: Fraction::new(0.0),
        insulation_factor: Fraction::new(0.0),
        surface_roughness: Fraction::new(0.1),
    };

    /// Get bark type name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self.bark_type_id {
            0 => "Smooth",
            1 => "Fibrous",
            2 => "Stringybark",
            3 => "Ironbark",
            4 => "Paperbark",
            _ => "None",
        }
    }
}

/// Comprehensive fuel type with scientifically accurate properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fuel {
    // Identification
    /// Unique fuel type identifier
    pub id: u8,
    /// Human-readable name
    pub name: String,

    // Thermal properties
    /// Heat content (kJ/kg) - 18,000-22,000 typical for wood
    pub heat_content: KjPerKg,
    /// Piloted ignition temperature (°C) - with external flame/spark present
    /// Reference: Janssens (1991) "Piloted ignition of wood: a review"
    /// Typical range: 200-365°C for wood, lower for fine fuels
    pub ignition_temperature: Celsius,
    /// Auto-ignition temperature (°C) - spontaneous combustion from radiant heat only
    /// Reference: Dietenberger (2016) "Wood Products: Thermal Degradation and Fire"
    /// Typically 100-150°C higher than piloted ignition
    /// Used when no adjacent burning element provides a pilot flame
    pub auto_ignition_temperature: Celsius,
    /// Maximum flame temperature (°C) - 800-1500 based on fuel
    pub max_flame_temperature: Celsius,
    /// Specific heat capacity (kJ/(kg·K)) - CRITICAL for thermal calculations
    pub specific_heat: KjPerKgK,
    /// Thermal conductivity (W/(m·K)) - heat conduction through fuel bed
    /// Wood: 0.1-0.2, Grass: 0.04-0.06, Litter: 0.05-0.1
    /// Reference: Incropera et al. (2002) "Fundamentals of Heat and Mass Transfer"
    pub thermal_conductivity: ThermalConductivity,
    /// Thermal diffusivity (m²/s) - rate of temperature change
    /// α = k/(ρ·c) where k=conductivity, ρ=density, c=specific heat
    /// Wood: ~1e-7, Grass: ~2e-7, dry fuel: higher values
    /// Reference: "Fire Dynamics" (Drysdale, 2011)
    pub thermal_diffusivity: ThermalDiffusivity,

    // Physical properties
    /// Bulk density (kg/m³)
    pub bulk_density: KgPerCubicMeter,
    /// Surface area to volume ratio (m²/m³) for heat transfer
    pub surface_area_to_volume: SurfaceAreaToVolume,
    /// Fuel bed depth (meters)
    pub fuel_bed_depth: Meters,

    // Moisture properties
    /// Base moisture content (0-1 fraction)
    pub base_moisture: Fraction,
    /// Moisture of extinction - won't burn above this fraction
    pub moisture_of_extinction: Fraction,

    // Fire behavior
    /// Burn rate coefficient (unitless)
    pub burn_rate_coefficient: f32,
    /// Ember production probability per second (stringybark=0.9, grass=0.1)
    pub ember_production: Fraction,
    /// Ember receptivity (0-1) - how easily spot fires ignite
    pub ember_receptivity: Fraction,
    /// Maximum spotting distance (meters)
    pub max_spotting_distance: Meters,

    // Ember physics properties (fuel-specific)
    /// Typical ember mass (kg) - stringybark=0.001, grass=0.0002
    pub ember_mass_kg: f32,
    /// Horizontal velocity fraction (0-1, bark=0.5, grass=0.3)
    pub ember_launch_velocity_factor: Fraction,

    // Rothermel model parameters (fuel-specific)
    /// Mineral damping (0-1) - mineral/ash content effect, wood=0.41, grass=0.7-0.9
    pub mineral_damping: Fraction,
    /// Particle density (kg/m³) - `ρ_p`, softwood=450, hardwood=550, grass=300
    pub particle_density: KgPerCubicMeter,
    /// Effective heating fraction (0-1) - fine=0.5-0.6, medium=0.4-0.5, coarse=0.3-0.4
    pub effective_heating: Fraction,
    /// Packing ratio (0-1) - β, actual/optimum, compacted=0.8, loose=0.5
    pub packing_ratio: Fraction,
    /// Optimum packing ratio - `β_op`, grass=0.35, shrub=0.30, forest=0.25
    pub optimum_packing_ratio: Fraction,

    // Thermal behavior coefficients (fuel-specific, not hardcoded)
    /// Newton's cooling coefficient (per second, grass=0.15, forest=0.05)
    pub cooling_rate: RatePerSecond,
    /// Fraction of combustion heat retained (0-1, grass=0.25, forest=0.40)
    pub self_heating_fraction: Fraction,
    /// Convective heat transfer coefficient h (W/(m²·K), grass=600, forest=400)
    pub convective_heat_coefficient: HeatTransferCoefficient,
    /// Atmospheric heat transfer efficiency (0-1, grass=0.85, forest=0.70)
    pub atmospheric_heat_efficiency: Fraction,
    /// Wind sensitivity multiplier (grass=1.0, forest=0.6)
    pub wind_sensitivity: Fraction,
    /// Crown fire temperature boost (0-1, stringybark=0.95)
    pub crown_fire_temp_multiplier: Fraction,

    // Radiative properties for Stefan-Boltzmann heat transfer
    /// Emissivity of unburned fuel (0-1) for radiative heat transfer
    /// Dry grass: 0.6-0.7, Eucalyptus bark: 0.75-0.85, Charcoal: 0.8-0.95
    /// Reference: Incropera et al. (2002) "Fundamentals of Heat and Mass Transfer"
    pub emissivity_unburned: Fraction,
    /// Emissivity of burning fuel/flames (0-1) for radiative heat transfer
    /// Active flames: 0.9-0.95 (high emissivity due to soot particles)
    /// Reference: Drysdale (2011) "An Introduction to Fire Dynamics"
    pub emissivity_burning: Fraction,

    // Combustion temperature response (fuel-specific)
    /// Temperature range for combustion rate normalization (K above ignition temp)
    /// Fine fuels (grass): 300-400K (rapid response)
    /// Medium fuels (shrubs): 400-500K (moderate response)
    /// Coarse fuels (logs): 500-700K (slow response)
    /// Used in combustion rate calculation: `temp_factor = (T - T_ig) / response_range`
    pub temperature_response_range: f32,

    // Terrain slope response coefficients (fuel-specific)
    #[expect(
        clippy::doc_markdown,
        reason = "McArthur is a scientific author name, not a code identifier"
    )]
    /// Base slope angle for uphill spread enhancement (degrees)
    /// Eucalyptus: 10°, Grass: 8°, Shrubs: 12°
    /// Reference: McArthur (1967) "Fire behaviour in eucalypt forests"
    pub slope_uphill_factor_base: Degrees,
    /// Power exponent for uphill slope effect (dimensionless)
    /// Controls how aggressively spread increases with slope
    /// Eucalyptus: 1.5, Grass: 1.8 (more sensitive), Heavy fuels: 1.3
    pub slope_uphill_power: f32,
    /// Slope angle divisor for downhill spread reduction (degrees)
    /// Eucalyptus: 30°, Grass: 25°, Heavy fuels: 35°
    pub slope_downhill_divisor: Degrees,
    /// Minimum slope factor for downhill spread (0-1)
    /// Prevents unrealistic spread slowdown on steep downhill
    /// Eucalyptus: 0.3, Grass: 0.4, Heavy fuels: 0.25
    pub slope_factor_minimum: Fraction,

    // Combustion and geometry properties (fuel-specific)
    /// Combustion efficiency (0-1) - fraction of fuel fully combusted
    pub combustion_efficiency: Fraction,
    /// Geometry multiplier for surface area calculation (wood=0.1, grass=0.15)
    pub surface_area_geometry_factor: f32,
    /// Flame area coefficient for view factor calculation
    /// Grass fires: 8-10 (wide, short flames)
    /// Forest fires: 4-6 (tall, narrow flames)
    /// Reference: Byram (1959) flame geometry, Drysdale (2011) planar radiator model
    pub flame_area_coefficient: f32,
    /// Base absorption efficiency (0-1) for radiant/convective heat
    /// Fine fuels: 0.85-0.95 (high surface area)
    /// Coarse fuels: 0.65-0.75 (lower surface area)
    /// Scales with sqrt(SAV) for realistic heat absorption
    /// Reference: Butler & Cohen (1998), Drysdale (2011) radiative transfer
    pub absorption_efficiency_base: Fraction,

    // Australian-specific
    /// Volatile oil content (kg/kg) - eucalypts: 0.02-0.05
    pub volatile_oil_content: Fraction,
    /// Oil vaporization temperature (°C) - 170 for eucalyptus
    pub oil_vaporization_temp: Celsius,
    /// Oil auto-ignition temperature (°C) - 232 for eucalyptus
    pub oil_autoignition_temp: Celsius,
    /// Bark characteristics for ladder fuels
    pub bark_properties: BarkProperties,
    /// Bark ladder intensity (kW/m) for stringybark
    pub bark_ladder_intensity: f32,
    /// Crown fire threshold intensity (kW/m)
    pub crown_fire_threshold: f32,

    // Van Wagner Crown Fire Model (1977, 1993)
    /// Crown bulk density (kg/m³) - CBD, typical 0.05-0.3
    pub crown_bulk_density: KgPerCubicMeter,
    /// Crown base height (m) - CBH, height to live crown base, typical 2-15
    pub crown_base_height: Meters,
    /// Foliar moisture content (%) - FMC, typical 80-120 for live foliage
    pub foliar_moisture_content: Percent,

    // Nelson Fuel Moisture Timelag System (2000)
    /// 1-hour timelag (fine fuels <6mm, grass/leaves)
    pub timelag_1h: Hours,
    /// 10-hour timelag (medium fuels 6-25mm, twigs)
    pub timelag_10h: Hours,
    /// 100-hour timelag (coarse fuels 25-75mm, branches)
    pub timelag_100h: Hours,
    /// 1000-hour timelag (very coarse fuels >75mm, logs)
    pub timelag_1000h: Hours,
    /// Fraction in each timelag class [1h, 10h, 100h, 1000h]
    pub size_class_distribution: [Fraction; 4],

    // Canopy structure for fire transition modeling
    pub canopy_structure: crate::physics::CanopyStructure,
}

impl Fuel {
    /// Create Eucalyptus Stringybark - extreme ladder fuel
    ///
    /// # Scientific References
    /// - Pausas et al. (2017) - "Fuelbed ignition potential and bark morphology"
    /// - Forest Education Foundation - "Eucalypts and Fire"
    /// - Black Saturday 2009 Royal Commission (25km spotting observations)
    /// - Oil properties: eucalyptol vaporization and autoignition temperatures
    #[must_use]
    pub fn eucalyptus_stringybark() -> Self {
        Fuel {
            id: 1,
            name: "Eucalyptus Stringybark".to_string(),
            heat_content: KjPerKg::new(21000.0),
            // Scientific basis: Stringybark has extremely fibrous, oil-impregnated bark
            // Eucalyptus oils (eucalyptol) have flash point ~46°C and autoignition ~250°C
            // Combined with low-density fibrous structure, piloted ignition occurs at 220-240°C
            // Reference: CSIRO Bushfire CRC, Pausas et al. (2017) bark flammability studies
            ignition_temperature: Celsius::new(228.0),
            // Auto-ignition: ~110-120°C higher than piloted for oil-rich bark
            // Radiant heat alone must pyrolyze fuel AND generate sufficient flammable gas concentration
            // Reference: Dietenberger (2016), surface temps 300-400°C prior to auto-ignition
            auto_ignition_temperature: Celsius::new(340.0),
            max_flame_temperature: Celsius::new(1400.0),
            specific_heat: KjPerKgK::new(1.5),
            thermal_conductivity: ThermalConductivity::new(0.12), // W/(m·K) - fibrous bark
            thermal_diffusivity: ThermalDiffusivity::new(1.5e-7), // m²/s - coarse wood fuel
            bulk_density: KgPerCubicMeter::new(550.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(150.0), // Research: 50-200 m²/m³ for fibrous bark strips (CSIRO)
            fuel_bed_depth: Meters::new(0.5),
            base_moisture: Fraction::new(0.10),
            moisture_of_extinction: Fraction::new(0.35),
            burn_rate_coefficient: 0.08,
            ember_production: Fraction::new(0.9), // EXTREME: 90% chance per second
            ember_receptivity: Fraction::new(0.6),
            max_spotting_distance: Meters::new(25000.0), // 25km spotting!

            // Ember physics (stringybark produces large bark embers)
            ember_mass_kg: 0.001, // 1g embers (large bark pieces)
            ember_launch_velocity_factor: Fraction::new(0.5), // Moderate horizontal launch

            // Rothermel parameters (eucalyptus hardwood)
            mineral_damping: Fraction::new(0.41), // Moderate mineral content (wood)
            particle_density: KgPerCubicMeter::new(550.0), // kg/m³ (eucalyptus hardwood)
            effective_heating: Fraction::new(0.45), // Medium-coarse fuel
            packing_ratio: Fraction::new(0.6),    // Fibrous bark, moderately packed
            optimum_packing_ratio: Fraction::new(0.25), // Coarse fuel optimal compaction

            // Thermal behavior (coarse fuel - retains heat well)
            cooling_rate: RatePerSecond::new(0.05), // Slow cooling (thick bark)
            self_heating_fraction: Fraction::new(0.4), // Retains 40% of combustion heat
            convective_heat_coefficient: HeatTransferCoefficient::new(400.0), // Moderate convection
            atmospheric_heat_efficiency: Fraction::new(0.7), // 70% heat to atmosphere
            wind_sensitivity: Fraction::new(0.6),   // Moderate wind effect (sheltered by canopy)
            crown_fire_temp_multiplier: Fraction::new(0.95), // Very hot crown fires

            // Radiative properties (eucalyptus bark ~0.8-0.85 unburned)
            emissivity_unburned: Fraction::new(0.82), // Eucalyptus bark emissivity
            emissivity_burning: Fraction::new(0.93),  // Active flames with soot

            // Combustion temperature response (coarse wood fuel)
            temperature_response_range: 550.0, // Kelvin above ignition

            // McArthur slope coefficients (eucalyptus forest)
            slope_uphill_factor_base: Degrees::new(10.0), // degrees
            slope_uphill_power: 1.5,                      // power exponent
            slope_downhill_divisor: Degrees::new(30.0),   // degrees
            slope_factor_minimum: Fraction::new(0.3),     // minimum factor

            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.92), // High efficiency (dry hardwood)
            surface_area_geometry_factor: 0.12,         // Irregular bark strips increase area
            flame_area_coefficient: 5.0, // Moderate-tall flames (forest fire geometry)
            absorption_efficiency_base: Fraction::new(0.70), // Coarse fuel, moderate absorption

            volatile_oil_content: Fraction::new(0.04),
            oil_vaporization_temp: Celsius::new(170.0),
            oil_autoignition_temp: Celsius::new(232.0),
            bark_properties: BarkProperties::STRINGYBARK,
            bark_ladder_intensity: 650.0, // Very high ladder fuel intensity
            crown_fire_threshold: 300.0,  // Low threshold (30% of normal)

            // Van Wagner Crown Fire Model parameters (stringybark eucalypt)
            crown_bulk_density: KgPerCubicMeter::new(0.2), // kg/m³ (high for eucalypts with dense canopy)
            crown_base_height: Meters::new(3.0),           // m (low due to ladder fuels)
            foliar_moisture_content: Percent::new(90.0),   // % (typical for eucalyptus foliage)

            // Nelson Timelag parameters (mixed size classes)
            timelag_1h: Hours::new(1.0),   // Fine bark strips and leaves
            timelag_10h: Hours::new(10.0), // Small twigs
            timelag_100h: Hours::new(100.0), // Medium branches
            timelag_1000h: Hours::new(1000.0), // Large branches and trunk
            size_class_distribution: [
                Fraction::new(0.15),
                Fraction::new(0.25),
                Fraction::new(0.35),
                Fraction::new(0.25),
            ], // Mixed with emphasis on 100h

            // Canopy structure (high ladder fuel continuity)
            canopy_structure: crate::physics::CanopyStructure::eucalyptus_stringybark(),
        }
    }

    /// Create Eucalyptus Smooth Bark - less ladder fuel
    #[must_use]
    pub fn eucalyptus_smooth_bark() -> Self {
        Fuel {
            id: 2,
            name: "Eucalyptus Smooth Bark".to_string(),
            heat_content: KjPerKg::new(20000.0),
            // Scientific basis: Smooth bark eucalypts have less exposed surface area
            // Still contain volatile oils but denser bark structure requires more heat
            // Piloted ignition ~260-280°C (higher than stringybark)
            // Reference: CSIRO fire behavior research, Australian bushfire literature
            ignition_temperature: Celsius::new(268.0),
            // Auto-ignition: ~120°C higher for denser smooth bark
            auto_ignition_temperature: Celsius::new(388.0),
            max_flame_temperature: Celsius::new(1300.0),
            specific_heat: KjPerKgK::new(1.5),
            thermal_conductivity: ThermalConductivity::new(0.15), // W/(m·K) - denser bark
            thermal_diffusivity: ThermalDiffusivity::new(1.2e-7), // m²/s - dense wood fuel
            bulk_density: KgPerCubicMeter::new(600.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(80.0), // Research: 50-100 m²/m³ for smooth bark (CSIRO)
            fuel_bed_depth: Meters::new(0.3),
            base_moisture: Fraction::new(0.12),
            moisture_of_extinction: Fraction::new(0.35),
            burn_rate_coefficient: 0.06,
            ember_production: Fraction::new(0.35), // Moderate: 35% chance per second
            ember_receptivity: Fraction::new(0.5),
            max_spotting_distance: Meters::new(10000.0), // 10km

            // Ember physics (smooth bark produces fewer, smaller embers)
            ember_mass_kg: 0.0007,                            // 0.7g embers
            ember_launch_velocity_factor: Fraction::new(0.4), // Lower launch velocity

            // Rothermel parameters (eucalyptus hardwood, denser)
            mineral_damping: Fraction::new(0.41), // Moderate mineral content
            particle_density: KgPerCubicMeter::new(600.0), // kg/m³ (dense eucalyptus)
            effective_heating: Fraction::new(0.40), // Coarse fuel
            packing_ratio: Fraction::new(0.5),    // Smooth bark, loosely packed
            optimum_packing_ratio: Fraction::new(0.28), // Medium fuel optimal compaction

            // Thermal behavior (medium fuel)
            cooling_rate: RatePerSecond::new(0.08), // Moderate cooling
            self_heating_fraction: Fraction::new(0.35), // Retains 35% of combustion heat
            convective_heat_coefficient: HeatTransferCoefficient::new(450.0), // Good convection
            atmospheric_heat_efficiency: Fraction::new(0.75), // 75% heat to atmosphere
            wind_sensitivity: Fraction::new(0.7),   // Moderate-high wind effect
            crown_fire_temp_multiplier: Fraction::new(0.90), // Hot crown fires

            // Radiative properties (smooth eucalyptus bark ~0.75-0.80)
            emissivity_unburned: Fraction::new(0.78), // Smooth bark emissivity
            emissivity_burning: Fraction::new(0.92),  // Active flames

            // Combustion temperature response (medium wood fuel)
            temperature_response_range: 500.0, // Kelvin above ignition

            // McArthur slope coefficients (eucalyptus forest, smooth bark less ladder fuel)
            slope_uphill_factor_base: Degrees::new(10.0), // degrees
            slope_uphill_power: 1.4,                      // slightly less sensitive
            slope_downhill_divisor: Degrees::new(32.0),   // degrees
            slope_factor_minimum: Fraction::new(0.32),    // minimum factor

            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.93), // High efficiency (dense hardwood)
            surface_area_geometry_factor: 0.08,         // Smooth bark, lower surface area
            flame_area_coefficient: 4.5, // Tall, narrow flames (forest fire geometry)
            absorption_efficiency_base: Fraction::new(0.65), // Dense coarse fuel, lower absorption

            volatile_oil_content: Fraction::new(0.02),
            oil_vaporization_temp: Celsius::new(170.0),
            oil_autoignition_temp: Celsius::new(232.0),
            bark_properties: BarkProperties::SMOOTH,
            bark_ladder_intensity: 200.0,
            crown_fire_threshold: 1000.0, // Normal threshold

            // Van Wagner Crown Fire Model parameters (smooth bark eucalypt)
            crown_bulk_density: KgPerCubicMeter::new(0.12), // kg/m³ (moderate for eucalypts)
            crown_base_height: Meters::new(8.0),            // m (higher, less ladder fuel)
            foliar_moisture_content: Percent::new(100.0),   // % (typical for eucalyptus)

            // Nelson Timelag parameters (coarser fuels)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.10),
                Fraction::new(0.20),
                Fraction::new(0.40),
                Fraction::new(0.30),
            ], // Emphasis on larger fuels

            // Canopy structure (low ladder fuel continuity)
            canopy_structure: crate::physics::CanopyStructure::eucalyptus_smooth_bark(),
        }
    }

    /// Create Dry Grass - fast ignition
    #[must_use]
    pub fn dry_grass() -> Self {
        Fuel {
            id: 3,
            name: "Dry Grass".to_string(),
            heat_content: KjPerKg::new(18500.0),
            // Scientific basis: Cured/dry grass has very low moisture (<8%)
            // Fine fuel structure with high surface-area-to-volume ratio
            // Piloted ignition at 220-260°C depending on curing level
            // Reference: Fons (1950), University of Canterbury grassland ignition studies
            ignition_temperature: Celsius::new(232.0),
            // Auto-ignition: Fine grass fuel auto-ignites ~100-110°C higher
            // Fine structure means faster heating but still needs higher temp without pilot
            auto_ignition_temperature: Celsius::new(338.0),
            max_flame_temperature: Celsius::new(900.0),
            specific_heat: KjPerKgK::new(2.1), // Higher specific heat
            thermal_conductivity: ThermalConductivity::new(0.05), // W/(m·K) - fine fuel with air gaps
            thermal_diffusivity: ThermalDiffusivity::new(2.0e-7), // m²/s - fine dry fuel heats faster
            bulk_density: KgPerCubicMeter::new(200.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(3500.0), // Fine grass - Rothermel typical value for herbaceous
            fuel_bed_depth: Meters::new(0.1),
            base_moisture: Fraction::new(0.05), // Very dry
            moisture_of_extinction: Fraction::new(0.25),
            burn_rate_coefficient: 0.15,                // Burns fast
            ember_production: Fraction::new(0.1),       // Low: 10% chance per second
            ember_receptivity: Fraction::new(0.8),      // Easy to ignite
            max_spotting_distance: Meters::new(1000.0), // 1km

            // Ember physics (grass produces very light embers)
            ember_mass_kg: 0.0002, // 0.2g embers (light grass)
            ember_launch_velocity_factor: Fraction::new(0.3), // Low horizontal component

            // Rothermel parameters (fine herbaceous fuel)
            mineral_damping: Fraction::new(0.85), // Low mineral content (grass)
            particle_density: KgPerCubicMeter::new(300.0), // kg/m³ (light herbaceous)
            effective_heating: Fraction::new(0.55), // Fine fuel heats quickly
            packing_ratio: Fraction::new(0.8),    // Compacted grass bed
            optimum_packing_ratio: Fraction::new(0.35), // Fine fuel optimal compaction

            // Thermal behavior (fine fuel - rapid heat exchange)
            cooling_rate: RatePerSecond::new(0.15), // Fast cooling (high surface area)
            self_heating_fraction: Fraction::new(0.25), // Radiates 75% away (fine)
            convective_heat_coefficient: HeatTransferCoefficient::new(600.0), // High convection (fine)
            atmospheric_heat_efficiency: Fraction::new(0.85), // 85% heat to atmosphere
            wind_sensitivity: Fraction::new(1.0),             // Maximum wind effect (fine fuel)
            crown_fire_temp_multiplier: Fraction::new(0.0),   // No crown fire (grass)

            // Radiative properties (dry grass ~0.6-0.7)
            emissivity_unburned: Fraction::new(0.65), // Dry grass/straw emissivity
            emissivity_burning: Fraction::new(0.90),  // Grass fire flames

            // Combustion temperature response (fine fuel - rapid response)
            temperature_response_range: 350.0, // Kelvin above ignition (fast)

            // Slope coefficients (grass - more slope-sensitive)
            slope_uphill_factor_base: Degrees::new(8.0), // degrees (more sensitive)
            slope_uphill_power: 1.8,                     // higher power (rapid increase)
            slope_downhill_divisor: Degrees::new(25.0),  // degrees
            slope_factor_minimum: Fraction::new(0.4), // higher minimum (grass doesn't slow as much)

            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.85), // Moderate efficiency (fast burn, incomplete)
            surface_area_geometry_factor: 0.15,         // Fine grass has high surface area
            flame_area_coefficient: 9.0,                // Wide, short flames (grass fire geometry)
            absorption_efficiency_base: Fraction::new(0.90), // Fine fuel, excellent absorption

            volatile_oil_content: Fraction::new(0.0),
            oil_vaporization_temp: Celsius::new(0.0),
            oil_autoignition_temp: Celsius::new(0.0),
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 2000.0,

            // Van Wagner Crown Fire Model parameters (grass - no crown fire)
            crown_bulk_density: KgPerCubicMeter::new(0.0), // N/A for grass
            crown_base_height: Meters::new(0.0),           // N/A for grass
            foliar_moisture_content: Percent::new(0.0), // N/A for grass (base_moisture used instead)

            // Nelson Timelag parameters (very fine fuels only)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(1.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
            ], // All 1-hour timelag

            // Canopy structure (grassland - no vertical structure)
            canopy_structure: crate::physics::CanopyStructure::grassland(),
        }
    }

    /// Create Shrubland/Scrub
    #[must_use]
    pub fn shrubland() -> Self {
        Fuel {
            id: 4,
            name: "Shrubland/Scrub".to_string(),
            heat_content: KjPerKg::new(19000.0),
            // Scientific basis: Mixed live/dead woody fuels
            // Live component has higher moisture, dead twigs ignite easier
            // Piloted ignition 280-320°C for mixed shrub fuel beds
            // Reference: Rothermel (1972) chaparral and shrub fuel models
            ignition_temperature: Celsius::new(295.0),
            // Auto-ignition: ~125°C higher for mixed woody shrub fuels
            auto_ignition_temperature: Celsius::new(420.0),
            max_flame_temperature: Celsius::new(1000.0),
            specific_heat: KjPerKgK::new(1.8),
            thermal_conductivity: ThermalConductivity::new(0.08), // W/(m·K) - mixed woody shrub
            thermal_diffusivity: ThermalDiffusivity::new(1.6e-7), // m²/s - medium fuel
            bulk_density: KgPerCubicMeter::new(350.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(10.0),
            fuel_bed_depth: Meters::new(0.4),
            base_moisture: Fraction::new(0.15),
            moisture_of_extinction: Fraction::new(0.30),
            burn_rate_coefficient: 0.10,
            ember_production: Fraction::new(0.24), // Medium: 24% chance per second
            ember_receptivity: Fraction::new(0.6),
            max_spotting_distance: Meters::new(3000.0), // 3km

            // Ember physics (medium-sized woody embers)
            ember_mass_kg: 0.0004,                             // 0.4g embers
            ember_launch_velocity_factor: Fraction::new(0.35), // Medium launch velocity

            // Rothermel parameters (medium shrub fuel)
            mineral_damping: Fraction::new(0.55), // Moderate-low mineral content
            particle_density: KgPerCubicMeter::new(450.0), // kg/m³ (woody shrubs)
            effective_heating: Fraction::new(0.48), // Medium fuel
            packing_ratio: Fraction::new(0.65),   // Moderately packed brush
            optimum_packing_ratio: Fraction::new(0.30), // Shrub optimal compaction

            // Thermal behavior (medium shrub fuel)
            cooling_rate: RatePerSecond::new(0.10), // Moderate cooling
            self_heating_fraction: Fraction::new(0.32), // Retains 32% of combustion heat
            convective_heat_coefficient: HeatTransferCoefficient::new(500.0), // Moderate convection
            atmospheric_heat_efficiency: Fraction::new(0.80), // 80% heat to atmosphere
            wind_sensitivity: Fraction::new(0.85),  // High wind effect (exposed)
            crown_fire_temp_multiplier: Fraction::new(0.85), // Moderate crown fires
            // Radiative properties (mixed shrub ~0.70-0.75)
            emissivity_unburned: Fraction::new(0.72), // Shrub/brush emissivity
            emissivity_burning: Fraction::new(0.91),  // Shrub fire flames

            // Combustion temperature response (medium fuel)
            temperature_response_range: 450.0, // Kelvin above ignition

            // Slope coefficients (shrubland)
            slope_uphill_factor_base: Degrees::new(12.0), // degrees (less sensitive than grass)
            slope_uphill_power: 1.5,                      // moderate power
            slope_downhill_divisor: Degrees::new(28.0),   // degrees
            slope_factor_minimum: Fraction::new(0.35),    // minimum factor
            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.88), // Good efficiency (woody fuel)
            surface_area_geometry_factor: 0.10,         // Medium-sized branches
            flame_area_coefficient: 6.5,                // Medium-tall flames (shrub fire geometry)
            absorption_efficiency_base: Fraction::new(0.75), // Medium fuel, good absorption

            volatile_oil_content: Fraction::new(0.01),
            oil_vaporization_temp: Celsius::new(180.0),
            oil_autoignition_temp: Celsius::new(250.0),
            bark_properties: BarkProperties::FIBROUS,
            bark_ladder_intensity: 300.0,
            crown_fire_threshold: 1200.0,

            // Van Wagner Crown Fire Model parameters (shrubland)
            crown_bulk_density: KgPerCubicMeter::new(0.08), // kg/m³ (low for shrubs)
            crown_base_height: Meters::new(0.5),            // m (low shrub canopy)
            foliar_moisture_content: Percent::new(110.0),   // % (higher for live shrubs)

            // Nelson Timelag parameters (fine to medium)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.40),
                Fraction::new(0.35),
                Fraction::new(0.20),
                Fraction::new(0.05),
            ], // Emphasis on finer fuels

            // Canopy structure (grassland - shrubs don't have tree canopy)
            canopy_structure: crate::physics::CanopyStructure::grassland(),
        }
    }

    /// Create Dead Wood/Litter
    #[must_use]
    pub fn dead_wood_litter() -> Self {
        Fuel {
            id: 5,
            name: "Dead Wood/Litter".to_string(),
            heat_content: KjPerKg::new(19500.0),
            // Scientific basis: Dead organic matter, very dry (<10% moisture)
            // Decomposed material has lower ignition point than solid wood
            // Piloted ignition 240-280°C for forest floor litter
            // Reference: USDA Forest Service fuel moisture studies, Nelson (2000)
            ignition_temperature: Celsius::new(258.0),
            // Auto-ignition: ~115-120°C higher for dry decomposed litter
            auto_ignition_temperature: Celsius::new(375.0),
            max_flame_temperature: Celsius::new(950.0),
            specific_heat: KjPerKgK::new(1.3), // Heats faster
            thermal_conductivity: ThermalConductivity::new(0.07), // W/(m·K) - dry porous litter
            thermal_diffusivity: ThermalDiffusivity::new(2.2e-7), // m²/s - dry fuel heats quickly
            bulk_density: KgPerCubicMeter::new(300.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(9.0),
            fuel_bed_depth: Meters::new(0.2),
            base_moisture: Fraction::new(0.05), // Very dry
            moisture_of_extinction: Fraction::new(0.25),
            burn_rate_coefficient: 0.12,
            ember_production: Fraction::new(0.35), // Moderate: 35% chance per second
            ember_receptivity: Fraction::new(0.9), // Highly susceptible
            max_spotting_distance: Meters::new(5000.0), // 5km

            // Ember physics (wood produces typical-sized embers)
            ember_mass_kg: 0.0005, // 0.5g embers (typical)
            ember_launch_velocity_factor: Fraction::new(0.4), // Moderate launch velocity

            // Rothermel parameters (medium-coarse dead fuel)
            mineral_damping: Fraction::new(0.45), // Higher mineral/ash content (dead material)
            particle_density: KgPerCubicMeter::new(400.0), // kg/m³ (decomposed wood)
            effective_heating: Fraction::new(0.42), // Medium-coarse fuel
            packing_ratio: Fraction::new(0.55),   // Loose litter bed
            optimum_packing_ratio: Fraction::new(0.22), // Dead fuel optimal compaction

            // Thermal behavior (dead coarse fuel)
            cooling_rate: RatePerSecond::new(0.06), // Slow cooling (insulated by litter)
            self_heating_fraction: Fraction::new(0.38), // Retains 38% of combustion heat
            convective_heat_coefficient: HeatTransferCoefficient::new(350.0), // Low convection (ground)
            atmospheric_heat_efficiency: Fraction::new(0.65), // 65% heat to atmosphere
            wind_sensitivity: Fraction::new(0.50),            // Low wind effect (ground level)
            crown_fire_temp_multiplier: Fraction::new(0.0),   // No crown fire (ground litter)

            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.90), // High efficiency (dry dead wood)
            surface_area_geometry_factor: 0.09,         // Compacted litter
            flame_area_coefficient: 7.0,                // Moderate flames (ground fire geometry)
            absorption_efficiency_base: Fraction::new(0.80), // Loose dead fuel, good absorption

            volatile_oil_content: Fraction::new(0.0),
            oil_vaporization_temp: Celsius::new(0.0),
            oil_autoignition_temp: Celsius::new(0.0),
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 1500.0,

            // Van Wagner Crown Fire Model parameters (ground litter - no crown)
            crown_bulk_density: KgPerCubicMeter::new(0.0),
            crown_base_height: Meters::new(0.0),
            foliar_moisture_content: Percent::new(0.0),

            // Nelson Timelag parameters (mixed dead fuels)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.20),
                Fraction::new(0.35),
                Fraction::new(0.35),
                Fraction::new(0.10),
            ], // Varied size classes

            // Canopy structure (grassland - ground litter)
            canopy_structure: crate::physics::CanopyStructure::grassland(),

            // Radiative heat transfer (dry dead wood/litter)
            emissivity_unburned: Fraction::new(0.75), // Dry organic matter
            emissivity_burning: Fraction::new(0.93),  // Active flames
            temperature_response_range: 550.0,        // Kelvin above ignition

            // Slope response (ground litter - less slope-sensitive)
            slope_uphill_factor_base: Degrees::new(8.0), // Lower base (ground fuel)
            slope_uphill_power: 1.3,                     // Gentler slope response
            slope_downhill_divisor: Degrees::new(25.0),  // Moderate downhill reduction
            slope_factor_minimum: Fraction::new(0.4),    // Higher minimum (insulated)
        }
    }

    /// Create Green Vegetation - fire resistant
    #[must_use]
    pub fn green_vegetation() -> Self {
        Fuel {
            id: 6,
            name: "Green Vegetation".to_string(),
            heat_content: KjPerKg::new(18000.0),
            // Scientific basis: Live vegetation with 60%+ moisture content
            // Water must evaporate before pyrolysis can occur (2260 kJ/kg latent heat)
            // Piloted ignition requires 350-400°C to overcome moisture barrier
            // Reference: Xanthopoulos & Wakimoto (1993), live fuel ignition studies
            ignition_temperature: Celsius::new(378.0), // Hard to ignite due to high moisture
            // Auto-ignition: ~120°C higher, extremely resistant to radiant-only ignition
            auto_ignition_temperature: Celsius::new(498.0),
            max_flame_temperature: Celsius::new(800.0),
            specific_heat: KjPerKgK::new(2.2),
            thermal_conductivity: ThermalConductivity::new(0.25), // W/(m·K) - high moisture increases conductivity
            thermal_diffusivity: ThermalDiffusivity::new(0.8e-7), // m²/s - water content slows heating
            bulk_density: KgPerCubicMeter::new(400.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(8.0),
            fuel_bed_depth: Meters::new(0.3),
            base_moisture: Fraction::new(0.60), // Very high moisture
            moisture_of_extinction: Fraction::new(0.40),
            burn_rate_coefficient: 0.04,
            ember_production: Fraction::new(0.03), // Very low: 3% chance per second
            ember_receptivity: Fraction::new(0.2), // Resistant to spot fires
            max_spotting_distance: Meters::new(200.0),

            // Ember physics (minimal ember production from green fuel)
            ember_mass_kg: 0.0003,                            // 0.3g embers
            ember_launch_velocity_factor: Fraction::new(0.2), // Minimal launch velocity

            // Rothermel parameters (live herbaceous/foliage)
            mineral_damping: Fraction::new(0.75), // Low mineral content (living tissue)
            particle_density: KgPerCubicMeter::new(350.0), // kg/m³ (living plant tissue)
            effective_heating: Fraction::new(0.50), // Fine to medium fuel
            packing_ratio: Fraction::new(0.70),   // Moderately dense vegetation
            optimum_packing_ratio: Fraction::new(0.32), // Live fuel optimal compaction

            // Thermal behavior (live fuel - moisture dominated)
            cooling_rate: RatePerSecond::new(0.12), // Fast cooling (moisture evaporation)
            self_heating_fraction: Fraction::new(0.20), // Low retention (moisture absorbs heat)
            convective_heat_coefficient: HeatTransferCoefficient::new(550.0), // High convection (moisture)
            atmospheric_heat_efficiency: Fraction::new(0.90), // 90% heat to atmosphere (cooling)
            wind_sensitivity: Fraction::new(0.75),            // Moderate-high wind effect
            crown_fire_temp_multiplier: Fraction::new(0.80),  // Cooler fires (moisture)

            // Combustion and geometry
            combustion_efficiency: Fraction::new(0.70), // Low efficiency (high moisture, incomplete)
            surface_area_geometry_factor: 0.11,         // Live foliage
            flame_area_coefficient: 7.5,                // Moderate-wide flames (live fuel geometry)
            absorption_efficiency_base: Fraction::new(0.85), // Fine live fuel, excellent absorption

            volatile_oil_content: Fraction::new(0.0),
            oil_vaporization_temp: Celsius::new(0.0),
            oil_autoignition_temp: Celsius::new(0.0),
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 2500.0,

            // Van Wagner Crown Fire Model parameters (green vegetation)
            crown_bulk_density: KgPerCubicMeter::new(0.05), // kg/m³ (very low, mostly water)
            crown_base_height: Meters::new(0.2),            // m (low ground vegetation)
            foliar_moisture_content: Percent::new(150.0),   // % (very high, green foliage)

            // Nelson Timelag parameters (fine live fuels)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.80),
                Fraction::new(0.15),
                Fraction::new(0.05),
                Fraction::new(0.0),
            ], // Mostly fine live fuels

            // Canopy structure (grassland - low vegetation)
            canopy_structure: crate::physics::CanopyStructure::grassland(),

            // Radiative heat transfer (green vegetation)
            emissivity_unburned: Fraction::new(0.95), // High water content
            emissivity_burning: Fraction::new(0.92),  // Active flames (less intense)
            temperature_response_range: 480.0,        // Kelvin above ignition (cooler)

            // Slope response (green vegetation - minimal slope effect)
            slope_uphill_factor_base: Degrees::new(5.0), // Lowest base (fire-resistant)
            slope_uphill_power: 1.0,                     // Linear response
            slope_downhill_divisor: Degrees::new(20.0),  // Minimal downhill effect
            slope_factor_minimum: Fraction::new(0.6),    // High minimum (moisture barrier)
        }
    }

    /// Get fuel by ID
    #[must_use]
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::eucalyptus_stringybark()),
            2 => Some(Self::eucalyptus_smooth_bark()),
            3 => Some(Self::dry_grass()),
            4 => Some(Self::shrubland()),
            5 => Some(Self::dead_wood_litter()),
            6 => Some(Self::green_vegetation()),
            _ => None,
        }
    }

    /// Calculate actual max flame temperature based on current conditions
    #[must_use]
    pub fn calculate_max_flame_temperature(&self, moisture_fraction: f32) -> f32 {
        let base_temp = 800.0 + (*self.heat_content - 18000.0) / 10.0;
        let oil_bonus = *self.volatile_oil_content * 3000.0;
        let moisture_penalty = moisture_fraction * 400.0;
        (base_temp + oil_bonus - moisture_penalty).clamp(600.0, 1500.0)
    }

    /// Check if this fuel can burn
    #[must_use]
    pub fn is_burnable(&self) -> bool {
        *self.heat_content > 0.0 && *self.ignition_temperature > 0.0
    }

    /// Get thermal transmissivity (0-1, how much heat passes through)
    /// Non-burnable objects like water, rock, concrete block heat
    #[must_use]
    pub fn thermal_transmissivity(&self) -> f32 {
        if self.is_burnable() {
            0.9 // Burnable fuels don't block much
        } else {
            // Non-burnable surfaces block heat
            match self.name.as_str() {
                "Water" => 0.1,    // Water blocks 90% of radiant heat
                "Rock" => 0.3,     // Rock blocks 70%
                "Concrete" => 0.2, // Concrete blocks 80%
                "Metal" => 0.4,    // Metal conducts but still blocks some
                _ => 0.5,          // Default non-burnable
            }
        }
    }

    /// Create non-burnable water fuel
    #[must_use]
    pub fn water() -> Self {
        Fuel {
            id: 10,
            name: "Water".to_string(),
            heat_content: KjPerKg::new(0.0),
            ignition_temperature: Celsius::new(0.0),
            auto_ignition_temperature: Celsius::new(0.0), // Non-burnable
            max_flame_temperature: Celsius::new(0.0),
            specific_heat: KjPerKgK::new(4.18), // Water has very high specific heat
            thermal_conductivity: ThermalConductivity::new(0.60), // W/(m·K) - water conductivity
            thermal_diffusivity: ThermalDiffusivity::new(1.43e-7), // m²/s - water thermal diffusivity
            bulk_density: KgPerCubicMeter::new(1000.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(0.0),
            fuel_bed_depth: Meters::new(0.0),
            base_moisture: Fraction::new(1.0),
            moisture_of_extinction: Fraction::new(1.0),
            burn_rate_coefficient: 0.0,
            ember_production: Fraction::new(0.0),
            ember_receptivity: Fraction::new(0.0),
            max_spotting_distance: Meters::new(0.0),

            // Ember physics (non-burnable - no embers)
            ember_mass_kg: 0.0,
            ember_launch_velocity_factor: Fraction::new(0.0),

            // Rothermel parameters (non-burnable)
            mineral_damping: Fraction::new(1.0), // N/A for non-burnable
            particle_density: KgPerCubicMeter::new(1000.0), // Water density
            effective_heating: Fraction::new(0.0), // N/A
            packing_ratio: Fraction::new(1.0),   // N/A
            optimum_packing_ratio: Fraction::new(1.0), // N/A

            // Thermal behavior (water - cooling only)
            cooling_rate: RatePerSecond::new(0.20), // Fast cooling (evaporation)
            self_heating_fraction: Fraction::new(0.0),
            convective_heat_coefficient: HeatTransferCoefficient::new(1000.0), // High cooling
            atmospheric_heat_efficiency: Fraction::new(1.0),                   // All heat absorbed
            wind_sensitivity: Fraction::new(0.0),                              // No wind effect
            crown_fire_temp_multiplier: Fraction::new(0.0),
            combustion_efficiency: Fraction::new(0.0), // Non-burnable
            surface_area_geometry_factor: 0.0,         // N/A for water
            flame_area_coefficient: 0.0,               // Non-burnable - no flames
            absorption_efficiency_base: Fraction::new(1.0), // Water absorbs all heat (cooling)

            volatile_oil_content: Fraction::new(0.0),
            oil_vaporization_temp: Celsius::new(0.0),
            oil_autoignition_temp: Celsius::new(0.0),
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 9999.0,

            // Van Wagner Crown Fire Model parameters (non-burnable)
            crown_bulk_density: KgPerCubicMeter::new(0.0),
            crown_base_height: Meters::new(0.0),
            foliar_moisture_content: Percent::new(0.0),

            // Nelson Timelag parameters (N/A - non-burnable)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
            ],

            // Canopy structure (grassland - water has no structure)
            canopy_structure: crate::physics::CanopyStructure::grassland(),

            // Radiative heat transfer (water - perfect absorber)
            emissivity_unburned: Fraction::new(0.96), // Water is excellent emitter
            emissivity_burning: Fraction::new(0.0),   // No burning
            temperature_response_range: 0.0,          // N/A

            // Slope response (water - no fire spread)
            slope_uphill_factor_base: Degrees::new(0.0), // No fire spread
            slope_uphill_power: 0.0,
            slope_downhill_divisor: Degrees::new(1.0),
            slope_factor_minimum: Fraction::new(0.0),
        }
    }

    /// Create non-burnable rock fuel
    #[must_use]
    pub fn rock() -> Self {
        Fuel {
            id: 11,
            name: "Rock".to_string(),
            heat_content: KjPerKg::new(0.0),
            ignition_temperature: Celsius::new(0.0),
            auto_ignition_temperature: Celsius::new(0.0), // Non-burnable
            max_flame_temperature: Celsius::new(0.0),
            specific_heat: KjPerKgK::new(0.84), // Rock specific heat
            thermal_conductivity: ThermalConductivity::new(2.0), // W/(m·K) - rock/granite conductivity
            thermal_diffusivity: ThermalDiffusivity::new(8.8e-7), // m²/s - rock thermal diffusivity
            bulk_density: KgPerCubicMeter::new(2700.0),
            surface_area_to_volume: SurfaceAreaToVolume::new(0.0),
            fuel_bed_depth: Meters::new(0.0),
            base_moisture: Fraction::new(0.0),
            moisture_of_extinction: Fraction::new(1.0),
            burn_rate_coefficient: 0.0,
            ember_production: Fraction::new(0.0),
            ember_receptivity: Fraction::new(0.0),
            max_spotting_distance: Meters::new(0.0),

            // Ember physics (non-burnable - no embers)
            ember_mass_kg: 0.0,
            ember_launch_velocity_factor: Fraction::new(0.0),

            // Rothermel parameters (non-burnable)
            mineral_damping: Fraction::new(0.0), // Not applicable
            particle_density: KgPerCubicMeter::new(2700.0), // Rock density
            effective_heating: Fraction::new(0.0), // N/A
            packing_ratio: Fraction::new(1.0),   // N/A
            optimum_packing_ratio: Fraction::new(1.0), // N/A

            // Thermal behavior (rock - heat sink)
            cooling_rate: RatePerSecond::new(0.03), // Very slow cooling (thermal mass)
            self_heating_fraction: Fraction::new(0.0),
            convective_heat_coefficient: HeatTransferCoefficient::new(200.0), // Low convection (smooth)
            atmospheric_heat_efficiency: Fraction::new(0.30),                 // Absorbs heat
            wind_sensitivity: Fraction::new(0.0),                             // No wind effect
            crown_fire_temp_multiplier: Fraction::new(0.0),
            combustion_efficiency: Fraction::new(0.0), // Non-burnable
            surface_area_geometry_factor: 0.0,         // N/A for rock
            flame_area_coefficient: 0.0,               // Non-burnable - no flames
            absorption_efficiency_base: Fraction::new(0.5), // Rock absorbs some heat (thermal mass)

            volatile_oil_content: Fraction::new(0.0),
            oil_vaporization_temp: Celsius::new(0.0),
            oil_autoignition_temp: Celsius::new(0.0),
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 9999.0,

            // Van Wagner Crown Fire Model parameters (non-burnable)
            crown_bulk_density: KgPerCubicMeter::new(0.0),
            crown_base_height: Meters::new(0.0),
            foliar_moisture_content: Percent::new(0.0),

            // Nelson Timelag parameters (N/A for rock)
            timelag_1h: Hours::new(1.0),
            timelag_10h: Hours::new(10.0),
            timelag_100h: Hours::new(100.0),
            timelag_1000h: Hours::new(1000.0),
            size_class_distribution: [
                Fraction::new(0.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
                Fraction::new(0.0),
            ],

            // Canopy structure (grassland - rock has no structure)
            canopy_structure: crate::physics::CanopyStructure::grassland(),

            // Radiative heat transfer (rock - depends on type)
            emissivity_unburned: Fraction::new(0.90), // Most rocks ~0.88-0.95
            emissivity_burning: Fraction::new(0.0),   // No burning
            temperature_response_range: 0.0,          // N/A

            // Slope response (rock - no fire spread)
            slope_uphill_factor_base: Degrees::new(0.0), // No fire spread
            slope_uphill_power: 0.0,
            slope_downhill_divisor: Degrees::new(1.0),
            slope_factor_minimum: Fraction::new(0.0),
        }
    }
}
