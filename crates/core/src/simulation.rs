use std::collections::HashSet;
use rayon::prelude::*;
use crate::element::{FuelElement, Vec3, FuelPart};
use crate::fuel::Fuel;
use crate::spatial::SpatialIndex;
use crate::weather::WeatherSystem;
use crate::ember::Ember;
use crate::physics::*;
use crate::australian;
use crate::pyrocumulonimbus::PyroCbSystem;

/// Main fire simulation
pub struct FireSimulation {
    // Fuel elements
    elements: Vec<Option<FuelElement>>,
    burning_elements: HashSet<u32>,
    next_element_id: u32,
    
    // Spatial indexing
    spatial_index: SpatialIndex,
    
    // Weather
    pub weather: WeatherSystem,
    
    // Embers
    embers: Vec<Ember>,
    next_ember_id: u32,
    
    // Pyrocumulonimbus system
    pub pyrocb_system: PyroCbSystem,
    
    // Configuration
    max_search_radius: f32,
    
    // Statistics
    pub total_fuel_consumed: f32,
    pub total_area_burned: f32,
    pub simulation_time: f32,
    pub max_fire_intensity: f32,
}

impl FireSimulation {
    /// Create a new fire simulation
    pub fn new(width: f32, height: f32, depth: f32) -> Self {
        let bounds = (
            Vec3::new(-width / 2.0, -height / 2.0, 0.0),
            Vec3::new(width / 2.0, height / 2.0, depth),
        );
        
        let spatial_index = SpatialIndex::new(bounds, 15.0); // 15m cells
        
        FireSimulation {
            elements: Vec::new(),
            burning_elements: HashSet::new(),
            next_element_id: 0,
            spatial_index,
            weather: WeatherSystem::default(),
            embers: Vec::new(),
            next_ember_id: 0,
            pyrocb_system: PyroCbSystem::new(),
            max_search_radius: 15.0,
            total_fuel_consumed: 0.0,
            total_area_burned: 0.0,
            simulation_time: 0.0,
            max_fire_intensity: 0.0,
        }
    }
    
    /// Add a fuel element to the simulation
    pub fn add_fuel_element(
        &mut self,
        position: Vec3,
        fuel: Fuel,
        mass: f32,
        part_type: FuelPart,
        parent_id: Option<u32>,
    ) -> u32 {
        let id = self.next_element_id;
        self.next_element_id += 1;
        
        let element = FuelElement::new(id, position, fuel, mass, part_type, parent_id);
        
        // Add to spatial index
        self.spatial_index.insert(id, position);
        
        // Add to elements array
        if id as usize >= self.elements.len() {
            self.elements.resize((id as usize + 1) * 2, None);
        }
        self.elements[id as usize] = Some(element);
        
        id
    }
    
    /// Get a fuel element by ID
    pub fn get_element(&self, id: u32) -> Option<&FuelElement> {
        self.elements.get(id as usize)?.as_ref()
    }
    
    /// Get a mutable fuel element by ID
    pub fn get_element_mut(&mut self, id: u32) -> Option<&mut FuelElement> {
        self.elements.get_mut(id as usize)?.as_mut()
    }
    
    /// Ignite a specific element
    pub fn ignite_element(&mut self, element_id: u32, initial_temp: f32) {
        if let Some(element) = self.get_element_mut(element_id) {
            element.ignite(initial_temp);
            self.burning_elements.insert(element_id);
        }
    }
    
    /// Set weather conditions
    pub fn set_weather(&mut self, weather: WeatherSystem) {
        self.weather = weather;
    }
    
    /// Main simulation update
    pub fn update(&mut self, dt: f32) {
        // 1. Update weather
        self.weather.update(dt);
        let ffdi_multiplier = self.weather.spread_rate_multiplier();
        let wind_vector = self.weather.wind_vector();
        let ambient_temp = self.weather.temperature;
        
        // 2. Process each burning element (parallel processing)
        let burning_ids: Vec<u32> = self.burning_elements.iter().copied().collect();
        
        // OPTIMIZATION: Pre-compute values that don't change per iteration
        let max_search_radius_sq = self.max_search_radius * self.max_search_radius;
        
        // Parallel processing: collect heat transfer data for all elements
        // OPTIMIZATION: Limit targets per source to prevent O(n²) explosion with many burning elements
        const MAX_TARGETS_PER_SOURCE: usize = 100;
        
        let heat_transfers: Vec<Vec<(u32, f32)>> = burning_ids.par_iter().map(|&element_id| {
            let mut transfers = Vec::new();
            
            if let Some(element) = self.get_element(element_id) {
                if !element.ignited {
                    return transfers;
                }
                
                // OPTIMIZATION: Cache element properties to reduce field accesses
                let element_pos = element.position;
                
                // Find nearby fuel
                let nearby = self.spatial_index.query_radius(element_pos, self.max_search_radius);
                
                // OPTIMIZATION: When there are many burning elements (>1000), prioritize closest targets
                let should_prioritize = burning_ids.len() > 1000;
                let mut target_distances: Vec<(u32, f32)> = Vec::with_capacity(
                    if should_prioritize { MAX_TARGETS_PER_SOURCE } else { nearby.len() }
                );
                
                // First pass: collect valid targets with distances
                for &target_id in &nearby {
                    if target_id == element_id {
                        continue;
                    }
                    
                    if let Some(target) = self.get_element(target_id) {
                        if target.ignited || !target.can_ignite() {
                            continue;
                        }
                        
                        // OPTIMIZATION: Calculate distance squared first (avoid sqrt)
                        let dx = target.position.x - element_pos.x;
                        let dy = target.position.y - element_pos.y;
                        let dz = target.position.z - element_pos.z;
                        let distance_sq = dx * dx + dy * dy + dz * dz;
                        
                        // OPTIMIZATION: Skip if too far (using squared distance)
                        if distance_sq < 0.01 || distance_sq > max_search_radius_sq {
                            continue;
                        }
                        
                        target_distances.push((target_id, distance_sq));
                    }
                }
                
                // OPTIMIZATION: Sort by distance and limit to closest targets only when under load
                if should_prioritize && target_distances.len() > MAX_TARGETS_PER_SOURCE {
                    target_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    target_distances.truncate(MAX_TARGETS_PER_SOURCE);
                }
                
                // Second pass: calculate heat transfer
                for (target_id, distance_sq) in target_distances {
                    if let Some(target) = self.get_element(target_id) {
                        let distance = distance_sq.sqrt();
                        
                        // Calculate heat components
                        let radiation = calculate_radiation_flux(element, target, distance);
                        let convection = calculate_convection_heat(element, target, distance);
                        
                        // Apply multipliers
                        let wind_boost = wind_radiation_multiplier(
                            element.position,
                            target.position,
                            wind_vector,
                        );
                        let vertical_factor = vertical_spread_factor(element, target);
                        let slope_factor = slope_spread_multiplier(element, target);
                        
                        let total_heat = (radiation + convection)
                            * wind_boost
                            * vertical_factor
                            * slope_factor
                            * ffdi_multiplier
                            * dt;
                        
                        transfers.push((target_id, total_heat));
                    }
                }
            }
            
            transfers
        }).collect();
        
        // Apply heat transfers sequentially (must be sequential to avoid race conditions)
        for transfers in heat_transfers {
            for (target_id, total_heat) in transfers {
                if let Some(target_mut) = self.get_element_mut(target_id) {
                    target_mut.apply_heat(total_heat, dt, ambient_temp);
                    
                    // Check if newly ignited
                    if target_mut.ignited {
                        self.burning_elements.insert(target_id);
                    }
                }
            }
        }
        
        // Process burning element state updates
        for &element_id in &burning_ids {
            // 3e. Burn fuel and update statistics
            if let Some(element) = self.get_element_mut(element_id) {
                let fuel_before = element.fuel_remaining;
                element.burn_fuel(dt);
                let fuel_consumed = fuel_before - element.fuel_remaining;
                
                // Update flame height
                element.update_flame_height();
                
                // Store for later use
                let should_generate_embers = element.ignited && element.fuel.ember_production > 0.0;
                let ember_data = if should_generate_embers {
                    Some((element.position, element.temperature, element.fuel_remaining, element.fuel.ember_production, element.fuel.id))
                } else {
                    None
                };
                
                // Check for oil vapor explosion
                let explosion = australian::update_oil_vaporization(element, dt);
                
                let still_burning = element.ignited;
                
                // Update stats after releasing borrow
                self.total_fuel_consumed += fuel_consumed;
                
                // 3f. Generate embers (probabilistic)
                if let Some((pos, temp, fuel_remaining, ember_prod, fuel_id)) = ember_data {
                    if rand::random::<f32>() < 0.1 * dt {
                        let new_embers = crate::ember::spawn_embers(
                            pos,
                            temp,
                            fuel_remaining,
                            ember_prod,
                            fuel_id,
                            &mut self.next_ember_id,
                        );
                        self.embers.extend(new_embers);
                    }
                }
                
                // 3g. Process explosion if occurred
                if let Some(explosion) = explosion {
                    self.process_explosion(explosion);
                }
                
                // 3h. Remove if fuel depleted
                if !still_burning {
                    self.burning_elements.remove(&element_id);
                }
            }
        }
        
        // 4. Update all embers (parallel)
        self.embers.par_iter_mut().for_each(|ember| {
            ember.update_physics(wind_vector, ambient_temp, dt);
        });
        
        // 5. Check ember spot fires
        // OPTIMIZATION: Limit ember checks to prevent performance degradation with many embers
        const MAX_EMBER_CHECKS_PER_FRAME: usize = 200;
        
        let embers_to_check: Vec<_> = self.embers.iter()
            .filter(|e| e.can_ignite())
            .take(MAX_EMBER_CHECKS_PER_FRAME)
            .map(|e| (e.position, e.temperature, e.source_fuel_type))
            .collect();
        
        for (pos, temp, _fuel_type) in embers_to_check {
            let nearby = self.spatial_index.query_radius(pos, 2.0);
            
            // OPTIMIZATION: Limit fuel checks per ember
            let check_limit = nearby.len().min(10);
            
            for &fuel_id in nearby.iter().take(check_limit) {
                if let Some(element) = self.get_element(fuel_id) {
                    if element.can_ignite() {
                        let ignition_prob = crate::ember::ember_ignition_probability(
                            &Ember::new(0, pos, Vec3::zeros(), temp, 0.001, _fuel_type),
                            element.fuel.ember_receptivity,
                        );
                        
                        if rand::random::<f32>() < ignition_prob * dt {
                            self.ignite_element(fuel_id, temp * 0.8);
                            break; // Only ignite one element per ember per frame
                        }
                    }
                }
            }
        }
        
        // 6. Remove dead embers
        self.embers.retain(|e| e.is_active());
        
        // 7. Check for pyroCb formation and update pyroCb system
        // Calculate total fire intensity across all burning elements
        let mut total_intensity = 0.0;
        let mut fire_center = Vec3::zeros();
        let mut burning_count = 0;
        
        for &element_id in &self.burning_elements {
            if let Some(element) = self.get_element(element_id) {
                let intensity = element.byram_fireline_intensity();
                total_intensity += intensity;
                fire_center += element.position * intensity;
                burning_count += 1;
            }
        }
        
        if burning_count > 0 && total_intensity > 0.0 {
            fire_center = fire_center / total_intensity;
            self.max_fire_intensity = self.max_fire_intensity.max(total_intensity);
            
            // Check for pyroCb formation conditions
            self.pyrocb_system.check_formation(
                fire_center,
                total_intensity,
                ambient_temp,
                self.weather.humidity,
                self.weather.wind_speed,
            );
        }
        
        // Update pyroCb system
        let mut rng = rand::thread_rng();
        self.pyrocb_system.update(
            dt,
            total_intensity,
            ambient_temp,
            self.weather.humidity,
            &mut rng,
        );
        
        // Process lightning strikes from pyroCb
        // Clone strikes to avoid borrow checker issues
        let lightning_strikes: Vec<_> = self.pyrocb_system.lightning_strikes.iter()
            .map(|strike| (strike.position, strike.energy, strike.ignition_radius))
            .collect();
        
        for (position, energy, radius) in lightning_strikes {
            let nearby = self.spatial_index.query_radius(position, radius);
            
            for &fuel_id in &nearby {
                if let Some(element) = self.get_element_mut(fuel_id) {
                    if element.can_ignite() {
                        // Lightning provides immediate high temperature
                        element.apply_heat(energy, 0.1, ambient_temp);
                        
                        if element.ignited {
                            self.burning_elements.insert(fuel_id);
                        }
                    }
                }
            }
        }
        
        // 8. Update statistics
        self.simulation_time += dt;
    }
    
    /// Process an explosion event
    fn process_explosion(&mut self, explosion: australian::ExplosionEvent) {
        let nearby = self.spatial_index.query_radius(explosion.position, explosion.blast_radius);
        let weather_temp = self.weather.temperature;
        
        for &target_id in &nearby {
            if let Some(element) = self.get_element_mut(target_id) {
                // Instantly heat all elements in blast radius
                let distance = (element.position - explosion.position).magnitude();
                let heat_fraction = 1.0 - (distance / explosion.blast_radius).min(1.0);
                let heat = explosion.energy * heat_fraction * 0.5; // 50% of explosion energy
                
                element.apply_heat(heat, 0.1, weather_temp);
                
                if element.ignited {
                    self.burning_elements.insert(target_id);
                }
            }
        }
    }
    
    /// Get all burning elements
    pub fn get_burning_elements(&self) -> Vec<&FuelElement> {
        self.burning_elements
            .iter()
            .filter_map(|&id| self.get_element(id))
            .collect()
    }
    
    /// Get all embers
    pub fn get_embers(&self) -> &[Ember] {
        &self.embers
    }
    
    /// Get number of burning elements
    pub fn burning_count(&self) -> usize {
        self.burning_elements.len()
    }
    
    /// Get number of active embers
    pub fn ember_count(&self) -> usize {
        self.embers.len()
    }
    
    /// Get total number of elements
    pub fn element_count(&self) -> usize {
        self.elements.iter().filter(|e| e.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simulation_creation() {
        let sim = FireSimulation::new(1000.0, 1000.0, 100.0);
        assert_eq!(sim.element_count(), 0);
        assert_eq!(sim.burning_count(), 0);
    }
    
    #[test]
    fn test_add_and_ignite() {
        let mut sim = FireSimulation::new(1000.0, 1000.0, 100.0);
        
        let fuel = Fuel::dry_grass();
        let id = sim.add_fuel_element(
            Vec3::new(0.0, 0.0, 0.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        
        assert_eq!(sim.element_count(), 1);
        assert_eq!(sim.burning_count(), 0);
        
        sim.ignite_element(id, 500.0);
        assert_eq!(sim.burning_count(), 1);
    }
    
    #[test]
    fn test_fire_spread() {
        let mut sim = FireSimulation::new(1000.0, 1000.0, 100.0);
        
        let fuel = Fuel::dry_grass();
        
        // Add two adjacent fuel elements
        let id1 = sim.add_fuel_element(
            Vec3::new(0.0, 0.0, 0.0),
            fuel.clone(),
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        
        let id2 = sim.add_fuel_element(
            Vec3::new(2.0, 0.0, 0.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        
        // Ignite first element
        sim.ignite_element(id1, 600.0);
        
        // Update simulation - fire should spread
        for _ in 0..100 {
            sim.update(0.1);
            
            if sim.burning_count() > 1 {
                break;
            }
        }
        
        // Second element should eventually ignite
        let element2 = sim.get_element(id2).unwrap();
        assert!(element2.temperature > 25.0); // Should be heating up
    }
}
