use serde::{Deserialize, Serialize};

/// Australian bark types critical for fire behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarkType {
    Smooth,          // Less fire risk
    Fibrous,         // Moderate ladder fuel
    Stringybark,     // EXTREME ladder fuel (causes crown fires)
    IronBark,        // Dense, slow burning
    PaperBark,       // Highly flammable
}

/// Comprehensive fuel type with scientifically accurate properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fuel {
    // Identification
    pub id: u8,
    pub name: String,
    
    // Thermal properties
    pub heat_content: f32,           // kJ/kg (18,000-22,000 typical)
    pub ignition_temperature: f32,   // °C (250-400)
    pub max_flame_temperature: f32,  // °C (800-1500 based on fuel)
    pub specific_heat: f32,          // kJ/(kg·K) - CRITICAL
    
    // Physical properties
    pub bulk_density: f32,           // kg/m³
    pub surface_area_to_volume: f32, // m²/m³ for heat transfer
    pub fuel_bed_depth: f32,         // meters
    
    // Moisture properties
    pub base_moisture: f32,          // Fraction (0-1)
    pub moisture_of_extinction: f32, // Won't burn above this
    
    // Fire behavior
    pub burn_rate_coefficient: f32,
    pub ember_production: f32,       // 0-1 scale
    pub ember_receptivity: f32,      // 0-1 (how easily spot fires ignite)
    pub max_spotting_distance: f32,  // meters
    
    // Australian-specific
    pub volatile_oil_content: f32,   // kg/kg (eucalypts: 0.02-0.05)
    pub oil_vaporization_temp: f32,  // °C (170 for eucalyptus)
    pub oil_autoignition_temp: f32,  // °C (232 for eucalyptus)
    pub bark_type: BarkType,         // Critical for ladder fuels
    pub bark_ladder_intensity: f32,  // kW/m for stringybark
    pub crown_fire_threshold: f32,   // kW/m intensity needed
}

impl Fuel {
    /// Create Eucalyptus Stringybark - extreme ladder fuel
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
            ember_production: 0.9,  // EXTREME ember production
            ember_receptivity: 0.6,
            max_spotting_distance: 25000.0,  // 25km spotting!
            volatile_oil_content: 0.04,
            oil_vaporization_temp: 170.0,
            oil_autoignition_temp: 232.0,
            bark_type: BarkType::Stringybark,
            bark_ladder_intensity: 650.0,  // Very high ladder fuel intensity
            crown_fire_threshold: 300.0,   // Low threshold (30% of normal)
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
            max_spotting_distance: 10000.0,  // 10km
            volatile_oil_content: 0.02,
            oil_vaporization_temp: 170.0,
            oil_autoignition_temp: 232.0,
            bark_type: BarkType::Smooth,
            bark_ladder_intensity: 200.0,
            crown_fire_threshold: 1000.0,  // Normal threshold
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
            specific_heat: 2.1,  // Higher specific heat
            bulk_density: 200.0,
            surface_area_to_volume: 12.0,  // High surface area
            fuel_bed_depth: 0.1,
            base_moisture: 0.05,  // Very dry
            moisture_of_extinction: 0.25,
            burn_rate_coefficient: 0.15,  // Burns fast
            ember_production: 0.2,  // Minimal embers
            ember_receptivity: 0.8,  // Easy to ignite
            max_spotting_distance: 500.0,
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_type: BarkType::Smooth,
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
            volatile_oil_content: 0.01,
            oil_vaporization_temp: 180.0,
            oil_autoignition_temp: 250.0,
            bark_type: BarkType::Fibrous,
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
            specific_heat: 1.3,  // Heats faster
            bulk_density: 300.0,
            surface_area_to_volume: 9.0,
            fuel_bed_depth: 0.2,
            base_moisture: 0.05,  // Very dry
            moisture_of_extinction: 0.25,
            burn_rate_coefficient: 0.12,
            ember_production: 0.5,
            ember_receptivity: 0.9,  // Highly susceptible
            max_spotting_distance: 1000.0,
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_type: BarkType::Smooth,
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
            ignition_temperature: 350.0,  // Hard to ignite
            max_flame_temperature: 800.0,
            specific_heat: 2.2,
            bulk_density: 400.0,
            surface_area_to_volume: 8.0,
            fuel_bed_depth: 0.3,
            base_moisture: 0.60,  // Very high moisture
            moisture_of_extinction: 0.40,
            burn_rate_coefficient: 0.04,
            ember_production: 0.1,
            ember_receptivity: 0.2,  // Resistant to spot fires
            max_spotting_distance: 200.0,
            volatile_oil_content: 0.0,
            oil_vaporization_temp: 0.0,
            oil_autoignition_temp: 0.0,
            bark_type: BarkType::Smooth,
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
}
