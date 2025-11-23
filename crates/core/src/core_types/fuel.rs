use serde::{Deserialize, Serialize};

/// Bark properties that affect fire behavior
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BarkProperties {
    pub bark_type_id: u8,        // Numeric ID for the bark type
    pub ladder_fuel_factor: f32, // 0-1 scale, how much it acts as ladder fuel
    pub flammability: f32,       // 0-1 scale, ignition ease
    pub shedding_rate: f32,      // 0-1 scale, how much bark sheds as embers
    pub insulation_factor: f32,  // 0-1 scale, protection of inner wood
    pub surface_roughness: f32,  // affects airflow and heat retention
}

impl BarkProperties {
    /// Smooth bark - minimal ladder fuel
    pub const SMOOTH: BarkProperties = BarkProperties {
        bark_type_id: 0,
        ladder_fuel_factor: 0.1,
        flammability: 0.3,
        shedding_rate: 0.1,
        insulation_factor: 0.2,
        surface_roughness: 0.2,
    };

    /// Fibrous bark - moderate ladder fuel
    pub const FIBROUS: BarkProperties = BarkProperties {
        bark_type_id: 1,
        ladder_fuel_factor: 0.5,
        flammability: 0.6,
        shedding_rate: 0.4,
        insulation_factor: 0.5,
        surface_roughness: 0.6,
    };

    /// Stringybark - EXTREME ladder fuel (causes crown fires)
    pub const STRINGYBARK: BarkProperties = BarkProperties {
        bark_type_id: 2,
        ladder_fuel_factor: 1.0,
        flammability: 0.9,
        shedding_rate: 0.8,
        insulation_factor: 0.4,
        surface_roughness: 0.9,
    };

    /// Ironbark - dense, slow burning
    pub const IRONBARK: BarkProperties = BarkProperties {
        bark_type_id: 3,
        ladder_fuel_factor: 0.2,
        flammability: 0.4,
        shedding_rate: 0.2,
        insulation_factor: 0.8,
        surface_roughness: 0.4,
    };

    /// Paperbark - highly flammable
    pub const PAPERBARK: BarkProperties = BarkProperties {
        bark_type_id: 4,
        ladder_fuel_factor: 0.7,
        flammability: 0.95,
        shedding_rate: 0.9,
        insulation_factor: 0.3,
        surface_roughness: 0.5,
    };

    /// Non-bark (for non-tree fuels)
    pub const NONE: BarkProperties = BarkProperties {
        bark_type_id: 255,
        ladder_fuel_factor: 0.0,
        flammability: 0.0,
        shedding_rate: 0.0,
        insulation_factor: 0.0,
        surface_roughness: 0.1,
    };

    /// Get bark type name
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
    pub id: u8,
    pub name: String,

    // Thermal properties
    pub heat_content: f32,          // kJ/kg (18,000-22,000 typical)
    pub ignition_temperature: f32,  // °C (250-400)
    pub max_flame_temperature: f32, // °C (800-1500 based on fuel)
    pub specific_heat: f32,         // kJ/(kg·K) - CRITICAL

    // Physical properties
    pub bulk_density: f32,           // kg/m³
    pub surface_area_to_volume: f32, // m²/m³ for heat transfer
    pub fuel_bed_depth: f32,         // meters

    // Moisture properties
    pub base_moisture: f32,          // Fraction (0-1)
    pub moisture_of_extinction: f32, // Won't burn above this

    // Fire behavior
    pub burn_rate_coefficient: f32,
    pub ember_production: f32,      // 0-1 scale
    pub ember_receptivity: f32,     // 0-1 (how easily spot fires ignite)
    pub max_spotting_distance: f32, // meters

    // Rothermel model parameters (fuel-specific)
    pub mineral_damping: f32,       // 0-1 (mineral/ash content effect, wood=0.41, grass=0.7-0.9)
    pub particle_density: f32,      // kg/m³ (ρ_p, softwood=450, hardwood=550, grass=300)
    pub effective_heating: f32,     // 0-1 (fine=0.5-0.6, medium=0.4-0.5, coarse=0.3-0.4)
    pub packing_ratio: f32,         // 0-1 (β, actual/optimum, compacted=0.8, loose=0.5)

    // Australian-specific
    pub volatile_oil_content: f32,       // kg/kg (eucalypts: 0.02-0.05)
    pub oil_vaporization_temp: f32,      // °C (170 for eucalyptus)
    pub oil_autoignition_temp: f32,      // °C (232 for eucalyptus)
    pub bark_properties: BarkProperties, // Bark characteristics for ladder fuels
    pub bark_ladder_intensity: f32,      // kW/m for stringybark
    pub crown_fire_threshold: f32,       // kW/m intensity needed
}

impl Fuel {
    /// Create Eucalyptus Stringybark - extreme ladder fuel
    ///
    /// # Scientific References
    /// - Pausas et al. (2017) - "Fuelbed ignition potential and bark morphology"
    /// - Forest Education Foundation - "Eucalypts and Fire"
    /// - Black Saturday 2009 Royal Commission (25km spotting observations)
    /// - Oil properties: eucalyptol vaporization and autoignition temperatures
    pub fn eucalyptus_stringybark() -> Self {
        Fuel {
            id: 1,
            name: "Eucalyptus Stringybark".to_string(),
            heat_content: 21000.0,
            ignition_temperature: 280.0,
            max_flame_temperature: 1400.0,
            specific_heat: 1.5,
            bulk_density: 550.0,
            surface_area_to_volume: 8.0,
            fuel_bed_depth: 0.5,
            base_moisture: 0.10,
            moisture_of_extinction: 0.35,
            burn_rate_coefficient: 0.08,
            ember_production: 0.9, // EXTREME ember production
            ember_receptivity: 0.6,
            max_spotting_distance: 25000.0, // 25km spotting!
            
            // Rothermel parameters (eucalyptus hardwood)
            mineral_damping: 0.41,      // Moderate mineral content (wood)
            particle_density: 550.0,    // kg/m³ (eucalyptus hardwood)
            effective_heating: 0.45,    // Medium-coarse fuel
            packing_ratio: 0.6,         // Fibrous bark, moderately packed
            
            volatile_oil_content: 0.04,
            oil_vaporization_temp: 170.0,
            oil_autoignition_temp: 232.0,
            bark_properties: BarkProperties::STRINGYBARK,
            bark_ladder_intensity: 650.0, // Very high ladder fuel intensity
            crown_fire_threshold: 300.0,  // Low threshold (30% of normal)
        }
    }

    /// Create Eucalyptus Smooth Bark - less ladder fuel
    pub fn eucalyptus_smooth_bark() -> Self {
        Fuel {
            id: 2,
            name: "Eucalyptus Smooth Bark".to_string(),
            heat_content: 20000.0,
            ignition_temperature: 290.0,
            max_flame_temperature: 1300.0,
            specific_heat: 1.5,
            bulk_density: 600.0,
            surface_area_to_volume: 6.0,
            fuel_bed_depth: 0.3,
            base_moisture: 0.12,
            moisture_of_extinction: 0.35,
            burn_rate_coefficient: 0.06,
            ember_production: 0.5,
            ember_receptivity: 0.5,
            max_spotting_distance: 10000.0, // 10km
            
            // Rothermel parameters (eucalyptus hardwood, denser)
            mineral_damping: 0.41,      // Moderate mineral content
            particle_density: 600.0,    // kg/m³ (dense eucalyptus)
            effective_heating: 0.40,    // Coarse fuel
            packing_ratio: 0.5,         // Smooth bark, loosely packed
            
            volatile_oil_content: 0.02,
            oil_vaporization_temp: 170.0,
            oil_autoignition_temp: 232.0,
            bark_properties: BarkProperties::SMOOTH,
            bark_ladder_intensity: 200.0,
            crown_fire_threshold: 1000.0, // Normal threshold
        }
    }

    /// Create Dry Grass - fast ignition
    pub fn dry_grass() -> Self {
        Fuel {
            id: 3,
            name: "Dry Grass".to_string(),
            heat_content: 18500.0,
            ignition_temperature: 250.0,
            max_flame_temperature: 900.0,
            specific_heat: 2.1, // Higher specific heat
            bulk_density: 200.0,
            surface_area_to_volume: 12.0, // High surface area
            fuel_bed_depth: 0.1,
            base_moisture: 0.05, // Very dry
            moisture_of_extinction: 0.25,
            burn_rate_coefficient: 0.15, // Burns fast
            ember_production: 0.2,       // Minimal embers
            ember_receptivity: 0.8,      // Easy to ignite
            max_spotting_distance: 500.0,
            
            // Rothermel parameters (fine herbaceous fuel)
            mineral_damping: 0.85,      // Low mineral content (grass)
            particle_density: 300.0,    // kg/m³ (light herbaceous)
            effective_heating: 0.55,    // Fine fuel heats quickly
            packing_ratio: 0.8,         // Compacted grass bed
            
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 2000.0,
        }
    }

    /// Create Shrubland/Scrub
    pub fn shrubland() -> Self {
        Fuel {
            id: 4,
            name: "Shrubland/Scrub".to_string(),
            heat_content: 19000.0,
            ignition_temperature: 300.0,
            max_flame_temperature: 1000.0,
            specific_heat: 1.8,
            bulk_density: 350.0,
            surface_area_to_volume: 10.0,
            fuel_bed_depth: 0.4,
            base_moisture: 0.15,
            moisture_of_extinction: 0.30,
            burn_rate_coefficient: 0.10,
            ember_production: 0.4,
            ember_receptivity: 0.6,
            max_spotting_distance: 2000.0,
            
            // Rothermel parameters (medium shrub fuel)
            mineral_damping: 0.55,      // Moderate-low mineral content
            particle_density: 450.0,    // kg/m³ (woody shrubs)
            effective_heating: 0.48,    // Medium fuel
            packing_ratio: 0.65,        // Moderately packed brush
            
            volatile_oil_content: 0.01,
            oil_vaporization_temp: 180.0,
            oil_autoignition_temp: 250.0,
            bark_properties: BarkProperties::FIBROUS,
            bark_ladder_intensity: 300.0,
            crown_fire_threshold: 1200.0,
        }
    }

    /// Create Dead Wood/Litter
    pub fn dead_wood_litter() -> Self {
        Fuel {
            id: 5,
            name: "Dead Wood/Litter".to_string(),
            heat_content: 19500.0,
            ignition_temperature: 270.0,
            max_flame_temperature: 950.0,
            specific_heat: 1.3, // Heats faster
            bulk_density: 300.0,
            surface_area_to_volume: 9.0,
            fuel_bed_depth: 0.2,
            base_moisture: 0.05, // Very dry
            moisture_of_extinction: 0.25,
            burn_rate_coefficient: 0.12,
            ember_production: 0.5,
            ember_receptivity: 0.9, // Highly susceptible
            max_spotting_distance: 1000.0,
            
            // Rothermel parameters (medium-coarse dead fuel)
            mineral_damping: 0.45,      // Higher mineral/ash content (dead material)
            particle_density: 400.0,    // kg/m³ (decomposed wood)
            effective_heating: 0.42,    // Medium-coarse fuel
            packing_ratio: 0.55,        // Loose litter bed
            
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 1500.0,
        }
    }

    /// Create Green Vegetation - fire resistant
    pub fn green_vegetation() -> Self {
        Fuel {
            id: 6,
            name: "Green Vegetation".to_string(),
            heat_content: 18000.0,
            ignition_temperature: 350.0, // Hard to ignite
            max_flame_temperature: 800.0,
            specific_heat: 2.2,
            bulk_density: 400.0,
            surface_area_to_volume: 8.0,
            fuel_bed_depth: 0.3,
            base_moisture: 0.60, // Very high moisture
            moisture_of_extinction: 0.40,
            burn_rate_coefficient: 0.04,
            ember_production: 0.1,
            ember_receptivity: 0.2, // Resistant to spot fires
            max_spotting_distance: 200.0,
            
            // Rothermel parameters (live herbaceous/foliage)
            mineral_damping: 0.75,      // Low mineral content (living tissue)
            particle_density: 350.0,    // kg/m³ (living plant tissue)
            effective_heating: 0.50,    // Fine to medium fuel
            packing_ratio: 0.70,        // Moderately dense vegetation
            
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 2500.0,
        }
    }

    /// Get fuel by ID
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
    pub fn calculate_max_flame_temperature(&self, moisture_fraction: f32) -> f32 {
        let base_temp = 800.0 + (self.heat_content - 18000.0) / 10.0;
        let oil_bonus = self.volatile_oil_content * 3000.0;
        let moisture_penalty = moisture_fraction * 400.0;
        (base_temp + oil_bonus - moisture_penalty).clamp(600.0, 1500.0)
    }

    /// Check if this fuel can burn
    pub fn is_burnable(&self) -> bool {
        self.heat_content > 0.0 && self.ignition_temperature > 0.0
    }

    /// Get thermal transmissivity (0-1, how much heat passes through)
    /// Non-burnable objects like water, rock, concrete block heat
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
    pub fn water() -> Self {
        Fuel {
            id: 10,
            name: "Water".to_string(),
            heat_content: 0.0,
            ignition_temperature: 0.0,
            max_flame_temperature: 0.0,
            specific_heat: 4.18, // Water has very high specific heat
            bulk_density: 1000.0,
            surface_area_to_volume: 0.0,
            fuel_bed_depth: 0.0,
            base_moisture: 1.0,
            moisture_of_extinction: 1.0,
            burn_rate_coefficient: 0.0,
            ember_production: 0.0,
            ember_receptivity: 0.0,
            max_spotting_distance: 0.0,
            
            // Rothermel parameters (non-burnable)
            mineral_damping: 1.0,       // N/A for non-burnable
            particle_density: 1000.0,   // Water density
            effective_heating: 0.0,     // N/A
            packing_ratio: 1.0,         // N/A
            
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 9999.0,
        }
    }

    /// Create non-burnable rock fuel
    pub fn rock() -> Self {
        Fuel {
            id: 11,
            name: "Rock".to_string(),
            heat_content: 0.0,
            ignition_temperature: 0.0,
            max_flame_temperature: 0.0,
            specific_heat: 0.84, // Rock specific heat
            bulk_density: 2700.0,
            surface_area_to_volume: 0.0,
            fuel_bed_depth: 0.0,
            base_moisture: 0.0,
            moisture_of_extinction: 1.0,
            burn_rate_coefficient: 0.0,
            ember_production: 0.0,
            ember_receptivity: 0.0,
            max_spotting_distance: 0.0,
            
            // Rothermel parameters (non-burnable)
            mineral_damping: 1.0,       // N/A for non-burnable
            particle_density: 2700.0,   // Rock density
            effective_heating: 0.0,     // N/A
            packing_ratio: 1.0,         // N/A
            
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_properties: BarkProperties::NONE,
            bark_ladder_intensity: 0.0,
            crown_fire_threshold: 9999.0,
        }
    }
}
